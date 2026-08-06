use crate::memory_plan::{MemoryAggregateMode, MemoryClosureClass, MemoryDropGlueKind, MemoryType};
use crate::ssa::*;

pub(in crate::ssa) fn lower_structural_memory(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    products: &HashMap<String, ProductId>,
) -> Result<StructuralMemoryMetadata> {
    let mut memory = StructuralMemoryMetadata::default();
    for fact in &plan.type_facts {
        if fact.closure.class != MemoryClosureClass::Deterministic
            || memory_type_has_resource(&fact.ty)
            || !matches!(
                fact.ty,
                MemoryType::String
                    | MemoryType::Path
                    | MemoryType::Product(_)
                    | MemoryType::Enum { .. }
            )
        {
            continue;
        }
        let ty = lower_memory_type(&fact.ty, products)?;
        let type_id = StructuralTypeId::new(
            u64::try_from(memory.types.len())
                .map_err(|_| Error::msg("structural type table exceeds u64"))?,
        );
        let layout_id = StructuralLayoutId::new(
            u64::try_from(memory.layouts.len())
                .map_err(|_| Error::msg("structural layout table exceeds u64"))?,
        );
        let kind = layout_kind(program, &fact.ty, products)?;
        let identity = match &kind {
            StructuralLayoutKind::Enum { runtime_layout, .. } => *runtime_layout,
            StructuralLayoutKind::String
            | StructuralLayoutKind::Path
            | StructuralLayoutKind::Product { .. } => structural_layout_identity(plan, &fact.ty)?,
        };
        memory.layouts.push(StructuralLayoutMetadata {
            id: layout_id,
            identity,
            kind,
        });
        memory.types.push(StructuralTypeMetadata {
            id: type_id,
            witness: MemoryWitnessId::new(fact.witness.as_bytes()),
            ty,
            layout: layout_id,
            mode: match fact.mode {
                MemoryAggregateMode::Copy => StructuralTypeMode::Copy,
                MemoryAggregateMode::ImmutableValue => StructuralTypeMode::Immutable,
                MemoryAggregateMode::Affine => StructuralTypeMode::Affine,
            },
        });
    }
    install_value_representations(&mut memory, plan, products)?;
    install_memory_witnesses(&mut memory, plan, products)?;
    if !memory.types.is_empty() || !memory.witnesses.is_empty() {
        memory.plan = lkjscript_ir::MemoryPlanId::new(plan.id.as_bytes());
    }
    Ok(memory)
}

fn memory_type_has_resource(ty: &MemoryType) -> bool {
    match ty {
        MemoryType::Resource(_) => true,
        MemoryType::List(inner) => memory_type_has_resource(inner),
        MemoryType::Enum { arguments, .. } => arguments.iter().any(memory_type_has_resource),
        MemoryType::Function { parameters, result } => {
            parameters.iter().any(memory_type_has_resource) || memory_type_has_resource(result)
        }
        MemoryType::ForAll { body, .. } => memory_type_has_resource(body),
        _ => false,
    }
}

pub(in crate::ssa) fn structural_glue(
    memory: &StructuralMemoryMetadata,
    ty: &SsaType,
) -> Result<DropGlueIdentity> {
    let item = memory
        .type_for(ty)
        .ok_or_else(|| Error::msg("structural HIR glue has no exact SSA type metadata"))?;
    if item.mode == StructuralTypeMode::Copy {
        return Err(Error::msg(
            "copy structural type cannot carry owner drop glue",
        ));
    }
    let layout = memory
        .layouts
        .get(item.layout.index().unwrap_or(usize::MAX))
        .ok_or_else(|| Error::msg("structural HIR glue has no exact SSA layout metadata"))?;
    let glue = match &layout.kind {
        StructuralLayoutKind::String => StructuralDropGlueIdentity::String {
            type_id: item.id,
            layout: item.layout,
        },
        StructuralLayoutKind::Path => StructuralDropGlueIdentity::Path {
            type_id: item.id,
            layout: item.layout,
        },
        StructuralLayoutKind::Product { product, .. } => StructuralDropGlueIdentity::Product {
            type_id: item.id,
            product: *product,
            layout: item.layout,
        },
        StructuralLayoutKind::Enum {
            enum_id,
            runtime_layout,
            ..
        } => StructuralDropGlueIdentity::Enum {
            type_id: item.id,
            enum_id: *enum_id,
            layout: item.layout,
            runtime_layout: *runtime_layout,
        },
    };
    Ok(DropGlueIdentity::Structural(glue))
}

pub(in crate::ssa) fn glue_type(
    kind: &MemoryDropGlueKind,
    products: &HashMap<String, ProductId>,
) -> Result<Option<SsaType>> {
    Ok(match kind {
        MemoryDropGlueKind::String => Some(SsaType::Str),
        MemoryDropGlueKind::Path => Some(SsaType::Path),
        MemoryDropGlueKind::Product(name) => {
            Some(SsaType::Product(*products.get(name).ok_or_else(|| {
                Error::msg("structural product glue has no ProductId")
            })?))
        }
        MemoryDropGlueKind::Enum { id, arguments } => Some(SsaType::Enum {
            id: lkjscript_ir::EnumId::new(*id),
            arguments: arguments
                .iter()
                .map(|ty| lower_memory_type(ty, products))
                .collect::<Result<Vec<_>>>()?,
        }),
        MemoryDropGlueKind::ByteVector
        | MemoryDropGlueKind::Bytes
        | MemoryDropGlueKind::Resource(_) => None,
    })
}

fn structural_layout_identity(plan: &HirMemoryPlan, ty: &MemoryType) -> Result<RuntimeLayoutId> {
    let mut bytes = b"lkjscript.ssa.structural-layout\0canonical-platform-contract".to_vec();
    for field in [
        plan.id.as_bytes(),
        crate::memory_plan::memory_type_identity(ty)?,
    ] {
        bytes.extend_from_slice(&32_u64.to_be_bytes());
        bytes.extend_from_slice(&field);
    }
    Ok(RuntimeLayoutId::new(lkjscript_contracts::sha256(&bytes)))
}

include!("structural_support.rs");
include!("model/witness.rs");
