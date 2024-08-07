use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct APStats {
    pub interfaces: Vec<Interface>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Interface {
    pub interface: String,
    pub mac: String,
    pub essid: String,
    pub mode: String,
    pub channel: String,
    pub channel_ghz: String,
    pub ht_mode: String,
    pub tx_power_dbm: String,
    pub link_quality: String,
    pub signal_dbm: String,
    pub noise_dbm: String,
    pub bitrate_mbps: String,
    pub rx_pck_s: String,
    pub tx_pck_s: String,
    pub rx_kbytes_s: String,
    pub tx_kbytes_s: String,
    pub rx_cmp_s: String,
    pub tx_cmp_s: String,
    pub rx_mcst_s: String,
    pub if_util: String,
    pub clients: Vec<Client>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Client {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub signal_dbm: String,
    pub noise_dbm: String,
    pub snr_db: String,
    pub last_comm_ms: String,
    pub current_time_ms: String,
    pub rx: RxStats,
    pub tx: TxStats,
    pub expected_throughput_mbps: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RxStats {
    pub bitrate_mbps: String,
    pub mcs: String,
    pub bandwidth_mhz: String,
    pub ss: String,
    pub packets: String,
    pub bytes: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxStats {
    pub bitrate_mbps: String,
    pub mcs: String,
    pub bandwidth_mhz: String,
    pub ss: String,
    pub packets: String,
    pub bytes: String,
    pub retries: String,
    pub failed: String,
}