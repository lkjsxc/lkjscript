use crate::budget::{ResourceCategory, RESOURCE_CATEGORY_COUNT, V1_RESOURCE_CATEGORY_COUNT};

use super::{v1, v2};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceCeilings {
    pub(crate) values: [u64; RESOURCE_CATEGORY_COUNT],
}

impl ResourceCeilings {
    pub const fn limit(self, category: ResourceCategory) -> u64 {
        self.values[category.index()]
    }

    pub const fn implementation_maxima() -> Self {
        Self { values: MAXIMA }
    }
}

const fn joined(
    first: [u64; V1_RESOURCE_CATEGORY_COUNT],
    second: [u64; v2::V2_RESOURCE_CATEGORY_COUNT],
) -> [u64; RESOURCE_CATEGORY_COUNT] {
    let mut result = [0; RESOURCE_CATEGORY_COUNT];
    let mut index = 0;
    while index < V1_RESOURCE_CATEGORY_COUNT {
        result[index] = first[index];
        index += 1;
    }
    let mut tail = 0;
    while tail < v2::V2_RESOURCE_CATEGORY_COUNT {
        result[index] = second[tail];
        index += 1;
        tail += 1;
    }
    result
}

pub(crate) const MAXIMA: [u64; RESOURCE_CATEGORY_COUNT] = joined(v1::MAXIMA, v2::MAXIMA);
pub(crate) const SANDBOX: [u64; RESOURCE_CATEGORY_COUNT] = joined(v1::SANDBOX, v2::SANDBOX);
pub(crate) const DEFAULT: [u64; RESOURCE_CATEGORY_COUNT] = joined(v1::DEFAULT, v2::DEFAULT);
pub(crate) const BUILD: [u64; RESOURCE_CATEGORY_COUNT] = joined(v1::BUILD, v2::BUILD);
pub(crate) const DETERMINISTIC: [u64; RESOURCE_CATEGORY_COUNT] =
    joined(v1::DETERMINISTIC, v2::DETERMINISTIC);
