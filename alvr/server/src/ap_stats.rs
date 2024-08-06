use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct APStats {
    interfaces: Vec<Interface>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Interface {
    interface: String,
    mac: String,
    essid: String,
    mode: String,
    channel: String,
    channel_ghz: String,
    ht_mode: String,
    tx_power_dbm: String,
    link_quality: String,
    signal_dbm: String,
    noise_dbm: String,
    bitrate_mbps: String,
    rx_pck_s: String,
    tx_pck_s: String,
    rx_kbytes_s: String,
    tx_kbytes_s: String,
    rx_cmp_s: String,
    tx_cmp_s: String,
    rx_mcst_s: String,
    if_util: String,
    clients: Vec<Client>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Client {
    ip: String,
    mac: String,
    hostname: String,
    signal_dbm: String,
    noise_dbm: String,
    snr_db: String,
    last_comm_ms: String,
    current_time_ms: String,
    rx: RxStats,
    tx: TxStats,
    expected_throughput_mbps: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RxStats {
    bitrate_mbps: String,
    mcs: String,
    bandwidth_mhz: String,
    ss: String,
    packets: String,
    bytes: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxStats {
    bitrate_mbps: String,
    mcs: String,
    bandwidth_mhz: String,
    ss: String,
    packets: String,
    bytes: String,
    retries: String,
    failed: String,
}