use super::MemoryObligation;

pub const fn mixed_scalar(
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
        possible_aliases: "boxed VM handles may alias; value identity is scalar",
        copyability: "scalar copy",
        current_ownership: "copy value; some VM representations are traced boxes",
        escape_behavior: "unrestricted scalar escape",
        lifetime: "containing value or collector reachability for a box",
        strong_cycles: "impossible",
        weak_links: "none",
        destructor: "trivial scalar; sweep drops optional box",
        external_resources: "none",
        portability: "send semantically",
        contention: "single-owner",
        allocation_frequency: "value-dependent in VM; none in native scalar paths",
        size_class: "eight-byte scalar or leaf HeapObj",
        current_trace_fields: "none",
        current_exact_roots: "boxed VM/native handle only",
        object_identity: "none",
        current_placement: "mixed inline/register and GcHeap leaf",
        candidate_placements: "typed inline VM slot, stack, register, caller-destination",
        reclamation_plan: "unbox everywhere; no reclamation",
        producers,
        tests,
        status: "Current mixed representation; complete unboxing accepted",
    }
}
