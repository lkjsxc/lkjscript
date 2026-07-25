use serde::Serialize;

use crate::semantic::charges::ProtocolLimits;
use crate::semantic::schema::ResourceProfile;

pub const MAX_SESSION_FRAME_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_SESSION_CUMULATIVE_INPUT_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_SESSION_CUMULATIVE_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_SESSION_REQUESTS: u64 = 128;
pub const MAX_SESSION_LIFETIME_FUEL: u64 = 10_000_000;
pub const MAX_SESSION_RETAINED_METADATA_BYTES: u64 = 256 * 1024;
pub const MAX_SESSION_REVISION: u64 = 1_000_000;
pub(super) const MAX_REQUEST_ID_BYTES: usize = 256;

#[derive(Clone, Copy, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionLimits {
    pub frame_input_bytes: u64,
    pub frame_output_bytes: u64,
    pub cumulative_input_bytes: u64,
    pub cumulative_output_bytes: u64,
    pub request_count: u64,
    pub lifetime_fuel: u64,
    pub retained_metadata_bytes: u64,
    pub retained_revisions: u64,
    pub cache_entries: u64,
    pub maximum_revision: u64,
}

impl SessionLimits {
    pub fn for_profile(profile: ResourceProfile) -> Self {
        let protocol = ProtocolLimits::for_profile(profile);
        Self {
            frame_input_bytes: protocol.request_bytes.min(MAX_SESSION_FRAME_BYTES),
            frame_output_bytes: u64::try_from(protocol.response_bytes)
                .unwrap_or(u64::MAX)
                .min(MAX_SESSION_FRAME_BYTES),
            cumulative_input_bytes: protocol
                .request_bytes
                .saturating_mul(MAX_SESSION_REQUESTS)
                .min(MAX_SESSION_CUMULATIVE_INPUT_BYTES),
            cumulative_output_bytes: u64::try_from(protocol.response_bytes)
                .unwrap_or(u64::MAX)
                .saturating_mul(MAX_SESSION_REQUESTS)
                .min(MAX_SESSION_CUMULATIVE_OUTPUT_BYTES),
            request_count: MAX_SESSION_REQUESTS,
            lifetime_fuel: protocol
                .work_units
                .saturating_mul(MAX_SESSION_REQUESTS)
                .min(MAX_SESSION_LIFETIME_FUEL),
            retained_metadata_bytes: u64::try_from(protocol.response_bytes)
                .unwrap_or(u64::MAX)
                .min(MAX_SESSION_RETAINED_METADATA_BYTES),
            retained_revisions: 1,
            cache_entries: 0,
            maximum_revision: MAX_SESSION_REVISION,
        }
    }
}
