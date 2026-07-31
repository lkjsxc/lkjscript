/// One exact runtime object family still permitted to use tracing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LegacyTracedFamily {
    pub identity: &'static str,
    pub heap_variant: &'static str,
}

/// Closed non-increasing migration registry for `HeapObj` variants.
///
/// Removing an entry is an accepted migration. Adding one requires an explicit
/// architectural reversal and changed contract evidence.
pub const LEGACY_TRACED_FAMILIES: &[LegacyTracedFamily] = &[
    LegacyTracedFamily {
        identity: "enum",
        heap_variant: "Enum",
    },
    LegacyTracedFamily {
        identity: "pair",
        heap_variant: "Pair",
    },
    LegacyTracedFamily {
        identity: "product",
        heap_variant: "Product",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_non_increasing_registry_contains_only_enum_pair_and_product() {
        assert_eq!(
            LEGACY_TRACED_FAMILIES,
            [
                LegacyTracedFamily {
                    identity: "enum",
                    heap_variant: "Enum",
                },
                LegacyTracedFamily {
                    identity: "pair",
                    heap_variant: "Pair",
                },
                LegacyTracedFamily {
                    identity: "product",
                    heap_variant: "Product",
                },
            ]
        );
    }
}
