use super::*;

#[derive(Clone)]
pub(super) struct RepresentationFacts {
    pub(super) type_id: crate::StructuralTypeId,
    pub(super) ty: crate::SsaType,
    pub(super) layout: crate::StructuralLayoutKind,
}

impl Evaluator<'_> {
    pub(super) fn representation_facts(
        &self,
        id: crate::StructuralRepresentationId,
        category: crate::StructuralValueCategory,
    ) -> Result<RepresentationFacts, Flow> {
        let memory = &self.program.program().memory;
        let representation = memory
            .representations
            .get(id.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == id && item.category == category)
            .ok_or_else(|| Flow::Trap("structural representation missing or stale".into()))?;
        let value_type = memory
            .types
            .get(representation.type_id.index().unwrap_or(usize::MAX))
            .filter(|item| {
                item.id == representation.type_id && item.layout == representation.layout
            })
            .ok_or_else(|| Flow::Trap("structural type metadata missing or stale".into()))?;
        let layout = memory
            .layouts
            .get(representation.layout.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == representation.layout)
            .ok_or_else(|| Flow::Trap("structural layout metadata missing or stale".into()))?;
        require_layout_type(&value_type.ty, &layout.kind, layout.identity)?;
        Ok(RepresentationFacts {
            type_id: value_type.id,
            ty: value_type.ty.clone(),
            layout: layout.kind.clone(),
        })
    }
}

pub(super) fn destination_fields(
    facts: &RepresentationFacts,
    active_variant: Option<VariantId>,
) -> Result<(Option<u16>, Vec<crate::SsaType>), Flow> {
    match (&facts.layout, active_variant) {
        (crate::StructuralLayoutKind::Product { fields, .. }, None) => Ok((None, fields.clone())),
        (crate::StructuralLayoutKind::Enum { variants, .. }, Some(active)) => variants
            .iter()
            .find(|variant| variant.variant == active)
            .map(|variant| (Some(variant.physical_tag), variant.fields.clone()))
            .ok_or_else(|| Flow::Trap("active structural enum variant is missing".into())),
        (crate::StructuralLayoutKind::String | crate::StructuralLayoutKind::Path, None) => {
            Ok((None, Vec::new()))
        }
        _ => Err(Flow::Trap(
            "structural destination active variant mismatch".into(),
        )),
    }
}

pub(super) fn aggregate_field_type(
    facts: &RepresentationFacts,
    value: &EvalValue,
    field: u16,
) -> Result<crate::SsaType, Flow> {
    let index = usize::from(field);
    match &facts.layout {
        crate::StructuralLayoutKind::Product { fields, .. } => fields.get(index).cloned(),
        crate::StructuralLayoutKind::Enum { variants, .. } => {
            if !matches!(
                value,
                EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_)
            ) {
                return Err(Flow::Trap(
                    "aggregate field borrow expects structural value".into(),
                ));
            }
            let mut candidate = None;
            for variant in variants {
                let Some(field_ty) = variant.fields.get(index) else {
                    continue;
                };
                if let Some(previous) = &candidate {
                    if previous != field_ty {
                        return Err(Flow::Trap(
                            "enum field borrow has variant-dependent type".into(),
                        ));
                    }
                } else {
                    candidate = Some(field_ty.clone());
                }
            }
            candidate
        }
        crate::StructuralLayoutKind::String | crate::StructuralLayoutKind::Path => None,
    }
    .ok_or_else(|| Flow::Trap("aggregate field is out of range".into()))
}

fn require_layout_type(
    ty: &crate::SsaType,
    layout: &crate::StructuralLayoutKind,
    identity: RuntimeLayoutId,
) -> Result<(), Flow> {
    let valid = match (ty, layout) {
        (crate::SsaType::Str, crate::StructuralLayoutKind::String)
        | (crate::SsaType::Path, crate::StructuralLayoutKind::Path) => true,
        (
            crate::SsaType::Product(expected),
            crate::StructuralLayoutKind::Product { product, .. },
        ) => expected == product,
        (
            crate::SsaType::Enum { id: expected, .. },
            crate::StructuralLayoutKind::Enum {
                enum_id,
                runtime_layout,
                ..
            },
        ) => expected == enum_id && *runtime_layout == identity,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(Flow::Trap(
            "structural layout and semantic type mismatch".into(),
        ))
    }
}
