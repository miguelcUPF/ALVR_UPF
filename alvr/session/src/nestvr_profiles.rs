use crate::settings::NestVrProfile;
use settings_schema::Switch;

#[derive(Debug, Clone, Copy)]
pub struct ProfileConfig {
    pub max_bitrate_mbps: Option<f32>,
    pub min_bitrate_mbps: Option<f32>,
    pub initial_bitrate_mbps: f32,

    pub update_interval_nestvr_s: f32,
    pub step_size_mbps: f32,
    pub r_step_size_mbps: f32,
    pub capacity_scaling_factor: f32,
    pub rtt_explor_prob: f32,
    pub nfr_thresh: f32,
    pub rtt_thresh_scaling_factor: f32,
}

pub fn get_profile_config(max_bitrate_mbps: &Switch<f32>, min_bitrate_mbps: &Switch<f32>, initial_bitrate_mbps: &f32, nest_vr_profile: &NestVrProfile) -> ProfileConfig {
    match nest_vr_profile {
        NestVrProfile::Custom {
            update_interval_nestvr_s,
            step_size_mbps,
            r_step_size_mbps,
            capacity_scaling_factor,
            rtt_explor_prob,
            nfr_thresh,
            rtt_thresh_scaling_factor,
        } => ProfileConfig {
            max_bitrate_mbps: if let Switch::Enabled(max) = max_bitrate_mbps {
                Some(*max)
            } else {
                None
            },
            min_bitrate_mbps: if let Switch::Enabled(min) = min_bitrate_mbps {
                Some(*min)
            } else {
                None
            },
            initial_bitrate_mbps: *initial_bitrate_mbps,

            update_interval_nestvr_s: *update_interval_nestvr_s,
            step_size_mbps: *step_size_mbps,
            r_step_size_mbps: *r_step_size_mbps,
            capacity_scaling_factor: *capacity_scaling_factor,
            rtt_explor_prob: *rtt_explor_prob,
            nfr_thresh: *nfr_thresh,
            rtt_thresh_scaling_factor: *rtt_thresh_scaling_factor,
        },
        NestVrProfile::Generic => ProfileConfig {
            max_bitrate_mbps: if let Switch::Enabled(max) = max_bitrate_mbps {
                Some(*max)
            } else {
                None
            },
            min_bitrate_mbps: if let Switch::Enabled(min) = min_bitrate_mbps {
                Some(*min)
            } else {
                None
            },
            initial_bitrate_mbps: *initial_bitrate_mbps,

            update_interval_nestvr_s: 1.0,
            step_size_mbps: 10.0,
            r_step_size_mbps: 10.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.25,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 2.0,
        },
        NestVrProfile::MinMax => ProfileConfig {
            max_bitrate_mbps: if let Switch::Enabled(max) = max_bitrate_mbps {
                Some(*max)
            } else {
                None
            },
            min_bitrate_mbps: if let Switch::Enabled(min) = min_bitrate_mbps {
                Some(*min)
            } else {
                None
            },
            initial_bitrate_mbps: *initial_bitrate_mbps,

            update_interval_nestvr_s: 1.0,
            step_size_mbps: 100.0,
            r_step_size_mbps: 100.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.25,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 2.0,
        },
        NestVrProfile::Drop => ProfileConfig {
            max_bitrate_mbps: if let Switch::Enabled(max) = max_bitrate_mbps {
                Some(*max)
            } else {
                None
            },
            min_bitrate_mbps: if let Switch::Enabled(min) = min_bitrate_mbps {
                Some(*min)
            } else {
                None
            },
            initial_bitrate_mbps: *initial_bitrate_mbps,

            update_interval_nestvr_s: 1.0,
            step_size_mbps: 10.0,
            r_step_size_mbps: 100.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.25,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 2.0,
        },
        NestVrProfile::SwiftDecline => ProfileConfig {
            max_bitrate_mbps: if let Switch::Enabled(max) = max_bitrate_mbps {
                Some(*max)
            } else {
                None
            },
            min_bitrate_mbps: if let Switch::Enabled(min) = min_bitrate_mbps {
                Some(*min)
            } else {
                None
            },
            initial_bitrate_mbps: *initial_bitrate_mbps,

            update_interval_nestvr_s: 1.0,
            step_size_mbps: 10.0,
            r_step_size_mbps: 20.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.25,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 2.0,
        },
        NestVrProfile::Mobility => ProfileConfig {
            max_bitrate_mbps: if let Switch::Enabled(max) = max_bitrate_mbps {
                Some(*max)
            } else {
                None
            },
            min_bitrate_mbps: if let Switch::Enabled(min) = min_bitrate_mbps {
                Some(*min)
            } else {
                None
            },
            initial_bitrate_mbps: *initial_bitrate_mbps,

            update_interval_nestvr_s: 0.5,
            step_size_mbps: 5.0,
            r_step_size_mbps: 15.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.2,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 1.5,
        },
        NestVrProfile::Dense => ProfileConfig {
            max_bitrate_mbps: if let Switch::Enabled(max) = max_bitrate_mbps {
                Some(*max)
            } else {
                None
            },
            min_bitrate_mbps: if let Switch::Enabled(min) = min_bitrate_mbps {
                Some(*min)
            } else {
                None
            },
            initial_bitrate_mbps: *initial_bitrate_mbps,

            update_interval_nestvr_s: 1.0,
            step_size_mbps: 20.0,
            r_step_size_mbps: 25.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.5,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 3.0,
        },
    }
}
