use crate::settings::NestVrProfile;

#[derive(Debug, Clone, Copy)]
pub struct ProfileConfig {
    pub max_bitrate_mbps: f32,
    pub min_bitrate_mbps: f32,
    pub initial_bitrate_mbps: f32,
    pub update_interval_nestvr_s: f32,
    pub step_size_mbps: f32,
    pub r_step_size_mbps: f32,
    pub capacity_scaling_factor: f32,
    pub rtt_explor_prob: f32,
    pub nfr_thresh: f32,
    pub rtt_thresh_scaling_factor: f32,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        ProfileConfig {
            max_bitrate_mbps: 0.0,
            min_bitrate_mbps: 0.0,
            initial_bitrate_mbps: 0.0,
            update_interval_nestvr_s: 0.0,
            step_size_mbps: 0.0,
            r_step_size_mbps: 0.0,
            capacity_scaling_factor: 0.0,
            rtt_explor_prob: 0.0,
            nfr_thresh: 0.0,
            rtt_thresh_scaling_factor: 0.0,
        }
    }
}

pub fn get_profile_config(
    max_bitrate_mbps: f32,
    min_bitrate_mbps: f32,
    initial_bitrate_mbps: f32,
    nest_vr_profile: &NestVrProfile,
) -> ProfileConfig {
    let base_config = ProfileConfig {
        max_bitrate_mbps,
        min_bitrate_mbps,
        initial_bitrate_mbps,
        ..Default::default()
    };

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
            update_interval_nestvr_s: *update_interval_nestvr_s,
            step_size_mbps: *step_size_mbps,
            r_step_size_mbps: *r_step_size_mbps,
            capacity_scaling_factor: *capacity_scaling_factor,
            rtt_explor_prob: *rtt_explor_prob,
            nfr_thresh: *nfr_thresh,
            rtt_thresh_scaling_factor: *rtt_thresh_scaling_factor,
            ..base_config
        },
        NestVrProfile::Generic => ProfileConfig {
            update_interval_nestvr_s: 1.0,
            step_size_mbps: 10.0,
            r_step_size_mbps: 10.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.25,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 2.0,
            ..base_config
        },
        NestVrProfile::MinMax => ProfileConfig {
            update_interval_nestvr_s: 1.0,
            step_size_mbps: 100.0,
            r_step_size_mbps: 100.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.25,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 2.0,
            ..base_config
        },
        NestVrProfile::Drop => ProfileConfig {
            update_interval_nestvr_s: 1.0,
            step_size_mbps: 10.0,
            r_step_size_mbps: 100.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.25,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 2.0,
            ..base_config
        },
        NestVrProfile::SwiftDecline => ProfileConfig {
            update_interval_nestvr_s: 1.0,
            step_size_mbps: 10.0,
            r_step_size_mbps: 20.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.25,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 2.0,
            ..base_config
        },
        NestVrProfile::Mobility => ProfileConfig {
            update_interval_nestvr_s: 0.5,
            step_size_mbps: 5.0,
            r_step_size_mbps: 15.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.2,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 1.5,
            ..base_config
        },
        NestVrProfile::Dense => ProfileConfig {
            update_interval_nestvr_s: 1.0,
            step_size_mbps: 20.0,
            r_step_size_mbps: 25.0,
            capacity_scaling_factor: 0.9,
            rtt_explor_prob: 0.5,
            nfr_thresh: 0.95,
            rtt_thresh_scaling_factor: 3.0,
            ..base_config
        },
    }
}
