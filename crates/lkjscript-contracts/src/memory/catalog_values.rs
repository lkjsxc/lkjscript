use super::templates::{borrowed, inline};
use super::templates_scalar::inline_scalar;
use super::templates_special::{affine_byte_vector, immutable_bytes};
use super::MemoryObligation;

pub(super) const VALUES: &[MemoryObligation] = &[
    inline(
        "bool",
        "source through native",
        "bool",
        "tagged immediate bool",
        "constants and operations",
        "scalar differential suites",
    ),
    immutable_bytes(),
    borrowed(
        "byte-slice",
        "shared byte view",
        "immutable",
        "borrow expression and exact operation signatures",
        "ownership flow and boundary suites",
    ),
    borrowed(
        "byte-slice-mut",
        "exclusive byte view",
        "exclusive mutable",
        "borrow-mut expression and exact operation signatures",
        "ownership alias and mutation suites",
    ),
    affine_byte_vector(
        "byte-vector",
        "new-byte-vector and ownership lowering",
        "ownership and bulk-byte suites",
    ),
    inline(
        "capability",
        "source through VM",
        "typed provider capability",
        "tagged immediate capability token",
        "main grants and explicit parameters",
        "capability confinement and malformed bytecode suites",
    ),
    inline_scalar(
        "f64",
        "exact IEEE-754 bits in typed evaluator, VM, and native values",
        "constants and numeric operations",
        "numeric differential and conversion suites",
    ),
    inline_scalar(
        "i64",
        "complete signed bits in typed evaluator, VM, and native values",
        "constants and numeric operations",
        "complete range and native scalar suites",
    ),
    inline(
        "symbol",
        "source, HIR, SSA, bytecode artifact, VM",
        "immutable symbol constant identity",
        "Value::Symbol artifact index; reachable returned text is copied",
        "validated constants and runtime metadata",
        "constant, return, equality and malformed bytecode suites",
    ),
    inline(
        "unit",
        "source through native",
        "unit",
        "tagged immediate unit",
        "constants and structured control",
        "scalar and structured outcome suites",
    ),
];
