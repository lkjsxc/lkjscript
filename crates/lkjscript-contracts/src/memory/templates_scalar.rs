use super::MemoryObligation;

pub const fn inline_scalar(
    identity: &'static str,
    layout: &'static str,
    producers: &'static str,
    tests: &'static str,
) -> MemoryObligation {
    MemoryObligation {
        identity,
        authority: "source through evaluator, VM and native",
        semantic_type: identity,
        runtime_layout: layout,
        value_semantics: "value",
        mutability: "immutable",
        possible_aliases: "value copies only; no object identity",
        copyability: "scalar copy",
        current_ownership: "copy value",
        escape_behavior: "unrestricted scalar escape",
        lifetime: "containing value",
        strong_cycles: "impossible",
        weak_links: "none",
        destructor: "trivial scalar",
        external_resources: "none",
        portability: "send semantically",
        contention: "single-owner",
        allocation_frequency: "none",
        size_class: "eight-byte payload in a sixteen-byte typed VM value",
        current_trace_fields: "none",
        current_exact_roots: "none",
        object_identity: "none",
        current_placement: "typed VM value, evaluator value, stack, register, caller destination",
        candidate_placements: "typed stack, register, caller destination",
        reclamation_plan: "none",
        producers,
        tests,
        status: "Current inline scalar representation",
    }
}
