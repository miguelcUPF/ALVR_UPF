#![allow(clippy::if_same_then_else)]

use crate::{
    decoder::{self, DECODER_INIT_CONFIG},
    logging_backend::{LogMirrorData, LOG_CHANNEL_SENDER},
    platform,
    sockets::AnnouncerSocket,
    statistics::StatisticsManager,
    storage::Config,
    ClientCoreEvent, EVENT_QUEUE, LIFECYCLE_STATE, STATISTICS_MANAGER,
};
use alvr_audio::AudioDevice;
use alvr_common::{
    debug, error,
    glam::UVec2,
    info,
    once_cell::sync::Lazy,
    parking_lot::{Condvar, RwLock},
    wait_rwlock, warn, AnyhowToCon, ConResult, ConnectionError, ConnectionState, LifecycleState,
    OptLazy, StatesWebrtc, ToCon, ALVR_VERSION,
};
use alvr_packets::{
    ClientConnectionResult, ClientControlPacket, ClientStatistics, Haptics, ServerControlPacket,
    StreamConfigPacket, Tracking, VideoPacketHeader, VideoStreamingCapabilities, AUDIO, HAPTICS,
    STATISTICS, TRACKING, VIDEO,
};
use alvr_session::{settings_schema::Switch, SessionConfig};
use alvr_sockets::{
    ControlSocketSender, PeerType, ProtoControlSocket, StreamSender, StreamSocketBuilder,
    KEEPALIVE_INTERVAL, KEEPALIVE_TIMEOUT,
};
use serde_json as json;
use std::{
    collections::HashMap,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

#[derive(Copy, Clone)]
pub struct VideoStatsRx {
    pub frame_index: u32,

    pub frame_span: f32,
    pub frame_interarrival: f32,

    pub interarrival_jitter: f32,
    pub ow_delay: f32,

    pub rx_bytes: u32,
    pub bytes_in_frame: u32,
    pub bytes_in_frame_app: u32,

    pub frames_skipped: u32,
    pub frames_dropped: u32,

    pub rx_shard_counter: u32,
    pub duplicated_shard_counter: u32,

    pub highest_rx_frame_index: i32,
    pub highest_rx_shard_index: i32,
    pub threshold_gcc: f32,
    pub internal_state_gcc: StatesWebrtc,
}
#[cfg(target_os = "android")]
use crate::audio;
#[cfg(not(target_os = "android"))]
use alvr_audio as audio;

const INITIAL_MESSAGE: &str = concat!(
    "Searching for streamer...\n",
    "Open ALVR on your PC then click \"Trust\"\n",
    "next to the client entry",
);
const NETWORK_UNREACHABLE_MESSAGE: &str = "Cannot connect to the internet";
// const INCOMPATIBLE_VERSIONS_MESSAGE: &str = concat!(
//     "Streamer and client have\n",
//     "incompatible types.\n",
//     "Please update either the app\n",
//     "on the PC or on the headset",
// );
const STREAM_STARTING_MESSAGE: &str = "The stream will begin soon\nPlease wait...";
const SERVER_RESTART_MESSAGE: &str = "The streamer is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The streamer has disconnected.";
const CONNECTION_TIMEOUT_MESSAGE: &str = "Connection timeout.";

const DISCOVERY_RETRY_PAUSE: Duration = Duration::from_millis(500);
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const CONNECTION_RETRY_INTERVAL: Duration = Duration::from_secs(1);
const HANDSHAKE_ACTION_TIMEOUT: Duration = Duration::from_secs(2);
const STREAMING_RECV_TIMEOUT: Duration = Duration::from_millis(500);

const MAX_UNREAD_PACKETS: usize = 10; // Applies per stream

pub static CONNECTION_STATE: Lazy<Arc<RwLock<ConnectionState>>> =
    Lazy::new(|| Arc::new(RwLock::new(ConnectionState::Disconnected)));
pub static DISCONNECTED_NOTIF: Condvar = Condvar::new();

pub static CONTROL_SENDER: OptLazy<ControlSocketSender<ClientControlPacket>> =
    alvr_common::lazy_mut_none();
pub static TRACKING_SENDER: OptLazy<StreamSender<Tracking>> = alvr_common::lazy_mut_none();
pub static STATISTICS_SENDER: OptLazy<StreamSender<ClientStatistics>> =
    alvr_common::lazy_mut_none();

fn set_hud_message(message: &str) {
    let message = format!(
        "ALVR v{}\nhostname: {}\nIP: {}\n\n{message}",
        *ALVR_VERSION,
        Config::load().hostname,
        platform::local_ip(),
    );

    EVENT_QUEUE
        .lock()
        .push_back(ClientCoreEvent::UpdateHudMessage(message));
}

fn is_streaming() -> bool {
    *CONNECTION_STATE.read() == ConnectionState::Streaming
}

pub fn connection_lifecycle_loop(
    recommended_view_resolution: UVec2,
    supported_refresh_rates: Vec<f32>,
) {
    set_hud_message(INITIAL_MESSAGE);

    while *LIFECYCLE_STATE.read() != LifecycleState::ShuttingDown {
        if *LIFECYCLE_STATE.read() == LifecycleState::Resumed {
            if let Err(e) =
                connection_pipeline(recommended_view_resolution, supported_refresh_rates.clone())
            {
                let message = format!("Connection error:\n{e}\nCheck the PC for more details");
                set_hud_message(&message);
                error!("Connection error: {e}");
            }
        } else {
            debug!("Skip try connection because the device is sleeping");
        }

        *CONNECTION_STATE.write() = ConnectionState::Disconnected;
        DISCONNECTED_NOTIF.notify_all();

        thread::sleep(CONNECTION_RETRY_INTERVAL);
    }
}

fn connection_pipeline(
    recommended_view_resolution: UVec2,
    supported_refresh_rates: Vec<f32>,
) -> ConResult {
    let (mut proto_control_socket, server_ip) = {
        let config = Config::load();
        let announcer_socket = AnnouncerSocket::new(&config.hostname).to_con()?;
        let listener_socket =
            alvr_sockets::get_server_listener(HANDSHAKE_ACTION_TIMEOUT).to_con()?;

        loop {
            if *LIFECYCLE_STATE.write() != LifecycleState::Resumed {
                return Ok(());
            }

            if let Err(e) = announcer_socket.broadcast() {
                warn!("Broadcast error: {e:?}");

                set_hud_message(NETWORK_UNREACHABLE_MESSAGE);

                thread::sleep(RETRY_CONNECT_MIN_INTERVAL);

                set_hud_message(INITIAL_MESSAGE);

                return Ok(());
            }

            if let Ok(pair) = ProtoControlSocket::connect_to(
                DISCOVERY_RETRY_PAUSE,
                PeerType::Server(&listener_socket),
            ) {
                break pair;
            }
        }
    };

    let mut connection_state_lock = CONNECTION_STATE.write();
    let disconnect_notif = Arc::new(Condvar::new());

    *connection_state_lock = ConnectionState::Connecting;

    let microphone_sample_rate = AudioDevice::new_input(None)
        .unwrap()
        .input_sample_rate()
        .unwrap();

    proto_control_socket
        .send(&ClientConnectionResult::ConnectionAccepted {
            client_protocol_id: alvr_common::protocol_id(),
            display_name: platform::device_model(),
            server_ip,
            streaming_capabilities: Some(VideoStreamingCapabilities {
                default_view_resolution: recommended_view_resolution,
                supported_refresh_rates,
                microphone_sample_rate,
            }),
        })
        .to_con()?;
    let config_packet =
        proto_control_socket.recv::<StreamConfigPacket>(HANDSHAKE_ACTION_TIMEOUT)?;

    let settings = {
        let mut session_desc = SessionConfig::default();
        session_desc
            .merge_from_json(&json::from_str(&config_packet.session).to_con()?)
            .to_con()?;
        session_desc.to_settings()
    };

    let negotiated_config =
        json::from_str::<HashMap<String, json::Value>>(&config_packet.negotiated).to_con()?;

    let view_resolution = negotiated_config
        .get("view_resolution")
        .and_then(|v| json::from_value(v.clone()).ok())
        .unwrap_or(UVec2::ZERO);
    let refresh_rate_hint = negotiated_config
        .get("refresh_rate_hint")
        .and_then(|v| v.as_f64())
        .unwrap_or(60.0) as f32;
    let game_audio_sample_rate = negotiated_config
        .get("game_audio_sample_rate")
        .and_then(|v| v.as_u64())
        .unwrap_or(44100) as u32;

    let streaming_start_event = ClientCoreEvent::StreamingStarted {
        view_resolution,
        refresh_rate_hint,
        settings: Box::new(settings.clone()),
    };

    *STATISTICS_MANAGER.lock() = Some(StatisticsManager::new(
        settings.connection.statistics_history_size,
        Duration::from_secs_f32(1.0 / refresh_rate_hint),
        if let Switch::Enabled(config) = settings.headset.controllers {
            config.steamvr_pipeline_frames
        } else {
            0.0
        },
    ));

    let (mut control_sender, mut control_receiver) = proto_control_socket
        .split(STREAMING_RECV_TIMEOUT)
        .to_con()?;

    match control_receiver.recv(HANDSHAKE_ACTION_TIMEOUT) {
        Ok(ServerControlPacket::StartStream) => {
            info!("Stream starting");
            set_hud_message(STREAM_STARTING_MESSAGE);
        }
        Ok(ServerControlPacket::Restarting) => {
            info!("Server restarting");
            set_hud_message(SERVER_RESTART_MESSAGE);
            return Ok(());
        }
        Err(e) => {
            info!("Server disconnected. Cause: {e}");
            set_hud_message(SERVER_DISCONNECTED_MESSAGE);
            return Ok(());
        }
        _ => {
            info!("Unexpected packet");
            set_hud_message("Unexpected packet");
            return Ok(());
        }
    }

    let stream_socket_builder = StreamSocketBuilder::listen_for_server(
        Duration::from_secs(1),
        settings.connection.stream_port,
        settings.connection.stream_protocol,
        settings.connection.dscp,
        settings.connection.client_send_buffer_bytes,
        settings.connection.client_recv_buffer_bytes,
    )
    .to_con()?;

    if let Err(e) = control_sender.send(&ClientControlPacket::StreamReady) {
        info!("Server disconnected. Cause: {e:?}");
        set_hud_message(SERVER_DISCONNECTED_MESSAGE);
        return Ok(());
    }

    let mut stream_socket = stream_socket_builder.accept_from_server(
        server_ip,
        settings.connection.stream_port,
        settings.connection.packet_size as _,
        HANDSHAKE_ACTION_TIMEOUT,
    )?;

    info!("Connected to server");
    let mut videoStats = VideoStatsRx {
        frame_index: 0,

        frame_span: 0.0,
        frame_interarrival: 0.0,

        interarrival_jitter: 0.0,
        ow_delay: 0.0,

        rx_bytes: 0,
        bytes_in_frame: 0,
        bytes_in_frame_app: 0,

        frames_skipped: 0,
        frames_dropped: 0,

        rx_shard_counter: 0,
        duplicated_shard_counter: 0,

        highest_rx_frame_index: 0,
        highest_rx_shard_index: 0,
        internal_state_gcc: StatesWebrtc::NORMAL,
        threshold_gcc: 0.0,
    };
    {
        let config = &mut *DECODER_INIT_CONFIG.lock();

        config.max_buffering_frames = settings.video.max_buffering_frames;
        config.buffering_history_weight = settings.video.buffering_history_weight;
        config.options = settings.video.mediacodec_extra_options;
    }

    let mut video_receiver =
        stream_socket.subscribe_to_stream::<VideoPacketHeader>(VIDEO, MAX_UNREAD_PACKETS);
    let mut game_audio_receiver = stream_socket.subscribe_to_stream(AUDIO, MAX_UNREAD_PACKETS);
    let tracking_sender = stream_socket.request_stream(TRACKING);
    let mut haptics_receiver =
        stream_socket.subscribe_to_stream::<Haptics>(HAPTICS, MAX_UNREAD_PACKETS);
    let statistics_sender = stream_socket.request_stream(STATISTICS);


    let last_instant_IDR_client = Instant::now(); 
    let interval_IDR_seconds_f32 = settings.connection.minimum_idr_interval_ms as f32 / 1000.0;  

    let video_receive_thread = thread::spawn(move || {
        let mut stream_corrupted = false;
        while is_streaming() {
            let data = match video_receiver.recv(STREAMING_RECV_TIMEOUT) {
                Ok(data) => data,
                Err(ConnectionError::TryAgain(_)) => continue,
                Err(ConnectionError::Other(_)) => return,
            };
            let Ok((header, nal)) = data.get() else {
                return;
            };

            videoStats.frame_index = data.get_frame_index();
            videoStats.frame_span = data.get_frame_span();
            videoStats.interarrival_jitter = data.get_interarrival_jitter();
            videoStats.ow_delay = data.get_ow_delay();
            videoStats.bytes_in_frame = data.get_bytes_in_frame();
            videoStats.bytes_in_frame_app = data.get_bytes_in_frame_app();
            videoStats.highest_rx_frame_index = data.get_highest_rx_frame_index();
            videoStats.highest_rx_shard_index = data.get_highest_rx_shard_index();

            videoStats.frame_interarrival += data.get_frame_interarrival();
            videoStats.rx_bytes += data.get_rx_bytes();
            videoStats.frames_skipped += data.get_frames_skipped();
            videoStats.rx_shard_counter += data.get_rx_shard_counter();
            videoStats.duplicated_shard_counter += data.get_duplicated_shard_counter();

            videoStats.internal_state_gcc = data.get_gcc_state();
            videoStats.threshold_gcc = data.get_adaptive_threshold();

            // periodically request IDR using value from session settings minimum_idr_interval_ms, 

            if Instant::now().saturating_duration_since(last_instant_IDR_client).as_secs_f32() >= interval_IDR_seconds_f32 {
                if let Some(sender) = &mut *CONTROL_SENDER.lock(){
                    sender.send(&ClientControlPacket::RequestIdr).ok(); 
                } 
            }

            if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                stats.report_video_packet_received(header.timestamp);
            }

            if header.is_idr {
                stream_corrupted = false;
            } else if data.had_packet_loss() {
                stream_corrupted = true;
                if let Some(sender) = &mut *CONTROL_SENDER.lock() {
                    sender.send(&ClientControlPacket::RequestIdr).ok();
                }
                warn!(
                    "Network dropped {} video packets",
                    data.get_frames_skipped()
                );
            }
            if !stream_corrupted || !settings.connection.avoid_video_glitching {
                if !decoder::push_nal(header.timestamp, nal) {
                    stream_corrupted = true;
                    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
                        sender.send(&ClientControlPacket::RequestIdr).ok();
                        videoStats.frames_dropped += 1;
                    }

                    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                        stats.report_video_packet_dropped(data.get_frame_index());
                    }
                    warn!(
                        "Dropped video packet {}. Reason: Decoder saturation",
                        data.get_frame_index()
                    )
                } else {
                    // case where frame is decoded correctly
                    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                        stats.report_video_statistics(header.timestamp, videoStats);
                    }
                    videoStats.frame_interarrival = 0.0;
                    videoStats.rx_bytes = 0;
                    videoStats.duplicated_shard_counter = 0;
                    videoStats.rx_shard_counter = 0;
                    videoStats.frames_skipped = 0;
                    videoStats.frames_dropped = 0;
                }
            } else {
                if let Some(sender) = &mut *CONTROL_SENDER.lock() {
                    sender.send(&ClientControlPacket::RequestIdr).ok();
                }
                videoStats.frames_dropped += 1;
                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_video_packet_dropped(data.get_frame_index());
                }
                warn!(
                    "Dropped video packet {}. Reason: Waiting for IDR frame",
                    data.get_frame_index()
                )
            }
        }
    });

    let game_audio_thread = if let Switch::Enabled(config) = settings.audio.game_audio {
        let device = AudioDevice::new_output(None, None).to_con()?;

        thread::spawn(move || {
            while is_streaming() {
                alvr_common::show_err(audio::play_audio_loop(
                    is_streaming,
                    &device,
                    2,
                    game_audio_sample_rate,
                    config.buffering.clone(),
                    &mut game_audio_receiver,
                ));
            }
        })
    } else {
        thread::spawn(|| ())
    };

    let microphone_thread = if matches!(settings.audio.microphone, Switch::Enabled(_)) {
        let device = AudioDevice::new_input(None).to_con()?;

        let microphone_sender = stream_socket.request_stream(AUDIO);

        thread::spawn(move || {
            while is_streaming() {
                match audio::record_audio_blocking(
                    Arc::new(is_streaming),
                    microphone_sender.clone(),
                    &device,
                    1,
                    false,
                ) {
                    Ok(()) => break,
                    Err(e) => {
                        error!("Audio record error: {e}");

                        continue;
                    }
                }
            }
        })
    } else {
        thread::spawn(|| ())
    };

    let haptics_receive_thread = thread::spawn(move || {
        while is_streaming() {
            let data = match haptics_receiver.recv(STREAMING_RECV_TIMEOUT) {
                Ok(packet) => packet,
                Err(ConnectionError::TryAgain(_)) => continue,
                Err(ConnectionError::Other(_)) => return,
            };
            let Ok(haptics) = data.get_header() else {
                return;
            };

            EVENT_QUEUE.lock().push_back(ClientCoreEvent::Haptics {
                device_id: haptics.device_id,
                duration: haptics.duration,
                frequency: haptics.frequency,
                amplitude: haptics.amplitude,
            });
        }
    });

    let (log_channel_sender, log_channel_receiver) = mpsc::channel();

    let control_send_thread = thread::spawn({
        let disconnect_notif = Arc::clone(&disconnect_notif);
        move || {
            let mut keepalive_deadline = Instant::now();

            #[cfg(target_os = "android")]
            let mut battery_deadline = Instant::now();

            while is_streaming() && *LIFECYCLE_STATE.read() == LifecycleState::Resumed {
                if let (Ok(packet), Some(sender)) = (
                    log_channel_receiver.recv_timeout(STREAMING_RECV_TIMEOUT),
                    &mut *CONTROL_SENDER.lock(),
                ) {
                    if let Err(e) = sender.send(&packet) {
                        info!("Server disconnected. Cause: {e:?}");
                        set_hud_message(SERVER_DISCONNECTED_MESSAGE);

                        break;
                    }
                }

                if Instant::now() > keepalive_deadline {
                    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
                        sender.send(&ClientControlPacket::KeepAlive).ok();

                        keepalive_deadline = Instant::now() + KEEPALIVE_INTERVAL;
                    }
                }

                #[cfg(target_os = "android")]
                if Instant::now() > battery_deadline {
                    let (gauge_value, is_plugged) = platform::get_battery_status();
                    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
                        sender
                            .send(&ClientControlPacket::Battery(crate::BatteryPacket {
                                device_id: *alvr_common::HEAD_ID,
                                gauge_value,
                                is_plugged,
                            }))
                            .ok();
                    }

                    battery_deadline = Instant::now() + Duration::from_secs(5);
                }
            }

            disconnect_notif.notify_one();
        }
    });

    let control_receive_thread = thread::spawn({
        let disconnect_notif = Arc::clone(&disconnect_notif);
        move || {
            let mut disconnection_deadline = Instant::now() + KEEPALIVE_TIMEOUT;
            while is_streaming() {
                let maybe_packet = control_receiver.recv(STREAMING_RECV_TIMEOUT);

                match maybe_packet {
                    Ok(ServerControlPacket::InitializeDecoder(config)) => {
                        decoder::create_decoder(config, settings.video.force_software_decoder);
                    }
                    Ok(ServerControlPacket::Restarting) => {
                        info!("{SERVER_RESTART_MESSAGE}");
                        set_hud_message(SERVER_RESTART_MESSAGE);
                        disconnect_notif.notify_one();
                    }
                    Ok(_) => (),
                    Err(ConnectionError::TryAgain(_)) => {
                        if Instant::now() > disconnection_deadline {
                            info!("{CONNECTION_TIMEOUT_MESSAGE}");
                            set_hud_message(CONNECTION_TIMEOUT_MESSAGE);
                            disconnect_notif.notify_one();
                        } else {
                            continue;
                        }
                    }
                    Err(e) => {
                        info!("{SERVER_DISCONNECTED_MESSAGE} Cause: {e}");
                        set_hud_message(SERVER_DISCONNECTED_MESSAGE);
                        disconnect_notif.notify_one();
                    }
                }

                disconnection_deadline = Instant::now() + KEEPALIVE_TIMEOUT;
            }
        }
    });

    let stream_receive_thread = thread::spawn({
        let disconnect_notif = Arc::clone(&disconnect_notif);
        move || {
            while is_streaming() {
                match stream_socket.recv() {
                    Ok(()) => (),
                    Err(ConnectionError::TryAgain(_)) => continue,
                    Err(e) => {
                        info!("Client disconnected. Cause: {e}");
                        set_hud_message(SERVER_DISCONNECTED_MESSAGE);
                        disconnect_notif.notify_one();
                    }
                }
            }
        }
    });

    *CONTROL_SENDER.lock() = Some(control_sender);
    *TRACKING_SENDER.lock() = Some(tracking_sender);
    *STATISTICS_SENDER.lock() = Some(statistics_sender);
    if let Switch::Enabled(filter_level) = settings.logging.client_log_report_level {
        *LOG_CHANNEL_SENDER.lock() = Some(LogMirrorData {
            sender: log_channel_sender,
            filter_level,
        });
    }
    EVENT_QUEUE.lock().push_back(streaming_start_event);

    *connection_state_lock = ConnectionState::Streaming;

    // Unlock CONNECTION_STATE and block thread
    wait_rwlock(&disconnect_notif, &mut connection_state_lock);

    *connection_state_lock = ConnectionState::Disconnecting;

    *CONTROL_SENDER.lock() = None;
    *TRACKING_SENDER.lock() = None;
    *STATISTICS_SENDER.lock() = None;
    *LOG_CHANNEL_SENDER.lock() = None;

    EVENT_QUEUE
        .lock()
        .push_back(ClientCoreEvent::StreamingStopped);

    #[cfg(target_os = "android")]
    {
        *crate::decoder::DECODER_SINK.lock() = None;
        *crate::decoder::DECODER_SOURCE.lock() = None;
    }

    // Remove lock to allow threads to properly exit:
    drop(connection_state_lock);

    video_receive_thread.join().ok();
    game_audio_thread.join().ok();
    microphone_thread.join().ok();
    haptics_receive_thread.join().ok();
    control_send_thread.join().ok();
    control_receive_thread.join().ok();
    stream_receive_thread.join().ok();

    Ok(())
}
