use super::*;

unit_enum!(MemoryDropClass { Static = 0, Dead = 1, Conditional = 2, Open = 3 });
canonical_struct!(MemoryLoanPlan {
    function,
    place,
    loan,
    expression,
    binding,
    kind,
    semantic_uses,
    end_after,
    entry,
});
canonical_struct!(MemoryDropGluePlan {
    id,
    kind,
    drop_path
});
canonical_struct!(MemoryDropAction { path, glue });
canonical_struct!(MemoryDropBranch {
    active_variant,
    actions
});
canonical_struct!(MemoryDropPathPlan { id, ty, branches });
canonical_struct!(MemoryObligation {
    id,
    function,
    entry,
    kind,
    drop_glue,
    drop_path,
    drop_class,
});
canonical_struct!(MemoryPlanWork {
    functions,
    entries,
    expressions,
    uses,
    loans,
    constants,
    calls,
    obligations,
    type_nodes,
    witnesses,
    witness_groups,
    witness_group_edges,
    type_edges,
    scc_work,
    aggregate_fields,
    aggregate_variants,
    destinations,
    value_placements,
    placement_work,
    borrow_scopes,
    drop_paths,
    verifier_steps,
});

impl Canonical for MemoryDropGlueKind {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::ByteVector => output.tag(0),
            Self::Bytes => output.tag(1),
            Self::Resource(kind) => tagged(output, 2, kind),
            Self::String => output.tag(3),
            Self::Path => output.tag(4),
            Self::Product(id) => tagged(output, 5, &id.raw()),
            Self::Enum { id, arguments } => {
                output.tag(6)?;
                output.value(id)?;
                output.value(arguments)
            }
        }
    }
}

impl Canonical for MemoryDropPathElement {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::ProductField { index, field } => {
                output.tag(0)?;
                output.value(index)?;
                output.value(field)
            }
            Self::EnumField {
                variant,
                index,
                field,
            } => {
                output.tag(1)?;
                output.value(variant)?;
                output.value(index)?;
                output.value(field)
            }
        }
    }
}

impl Canonical for MemoryObligationKind {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::DropWholeValue => output.tag(0),
            Self::DropResource(kind) => tagged(output, 1, kind),
            Self::EndBorrow => output.tag(2),
        }
    }
}

fn tagged<T: Canonical + ?Sized>(output: &mut Encoder, tag: u8, value: &T) -> Result<()> {
    output.tag(tag)?;
    output.value(value)
}
