use crate::budget::{RESOURCE_CATEGORY_COUNT, V1_RESOURCE_CATEGORY_COUNT};

pub(crate) const V2_RESOURCE_CATEGORY_COUNT: usize =
    RESOURCE_CATEGORY_COUNT - V1_RESOURCE_CATEGORY_COUNT;

const SANDBOX_BASE: [u64; V2_RESOURCE_CATEGORY_COUNT] = [
    1_024, 4_096, 16_384, 65_536, 16_384, 8_192, 32_768, 32_768, 1_000_000, 8_192, 1_048_576,
    1_024, 16_384, 1_000_000, 65_536, 1_000_000, 4_194_304, 4_194_304, 65_536, 256, 4_194_304,
    1_024, 256, 1_024, 8_192, 32_768, 4_194_304, 65_536, 1_000_000,
];

const fn scaled(multiplier: u64) -> [u64; V2_RESOURCE_CATEGORY_COUNT] {
    let mut result = [0; V2_RESOURCE_CATEGORY_COUNT];
    let mut index = 0;
    while index < V2_RESOURCE_CATEGORY_COUNT {
        result[index] = SANDBOX_BASE[index] * multiplier;
        index += 1;
    }
    result
}

pub(crate) const SANDBOX: [u64; V2_RESOURCE_CATEGORY_COUNT] = SANDBOX_BASE;
pub(crate) const DETERMINISTIC: [u64; V2_RESOURCE_CATEGORY_COUNT] = scaled(4);
pub(crate) const DEFAULT: [u64; V2_RESOURCE_CATEGORY_COUNT] = scaled(8);
pub(crate) const BUILD: [u64; V2_RESOURCE_CATEGORY_COUNT] = scaled(32);
pub(crate) const MAXIMA: [u64; V2_RESOURCE_CATEGORY_COUNT] = scaled(64);
