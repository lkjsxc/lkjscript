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
        identity: "buf",
        heap_variant: "Buf",
    },
    LegacyTracedFamily {
        identity: "enum",
        heap_variant: "Enum",
    },
    LegacyTracedFamily {
        identity: "pair",
        heap_variant: "Pair",
    },
    LegacyTracedFamily {
        identity: "path",
        heap_variant: "Path",
    },
    LegacyTracedFamily {
        identity: "product",
        heap_variant: "Product",
    },
    LegacyTracedFamily {
        identity: "string",
        heap_variant: "Str",
    },
];
