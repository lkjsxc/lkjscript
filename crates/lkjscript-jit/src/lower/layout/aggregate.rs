use super::*;

mod owner_storage;

impl StructuralCatalog {
    pub(in crate::lower) fn aggregate(
        &self,
        type_id: lkjscript_ir::StructuralTypeId,
        active_variant: Option<lkjscript_ir::VariantId>,
    ) -> Result<lkjscript_native::StructuralAggregateDescriptor, LoweringError> {
        let ty = self
            .type_ids
            .get(&type_id)
            .ok_or_else(|| invalid_structural("structural aggregate type is absent"))?;
        let value_type = self
            .types
            .get(ty)
            .copied()
            .ok_or_else(|| invalid_structural("structural aggregate identity is absent"))?;
        let (kind, fields, variant_bytes) = match self.layouts.get(&type_id) {
            Some(lkjscript_ir::StructuralLayoutKind::Product { fields, .. }) => {
                if active_variant.is_some() {
                    return Err(invalid_structural(
                        "structural product names an active enum variant",
                    ));
                }
                (
                    lkjscript_native::StructuralAggregateKind::Product,
                    fields.as_slice(),
                    None,
                )
            }
            Some(lkjscript_ir::StructuralLayoutKind::Enum { variants, .. }) => {
                let active = active_variant.ok_or_else(|| {
                    invalid_structural("structural enum destination has no active variant")
                })?;
                let variant = variants
                    .iter()
                    .find(|variant| variant.variant == active)
                    .ok_or_else(|| invalid_structural("structural enum variant is absent"))?;
                (
                    lkjscript_native::StructuralAggregateKind::Enum(
                        u16::try_from(variant.physical_tag).map_err(|_| {
                            invalid_structural("structural enum tag exceeds native eligibility")
                        })?,
                    ),
                    variant.fields.as_slice(),
                    Some(active.bytes()),
                )
            }
            Some(
                lkjscript_ir::StructuralLayoutKind::String
                | lkjscript_ir::StructuralLayoutKind::Path,
            ) => {
                return Err(invalid_structural(
                    "structural leaf cannot create an aggregate destination",
                ));
            }
            None => return Err(invalid_structural("structural aggregate layout is absent")),
        };
        let fields = fields
            .iter()
            .map(|field| {
                self.value_type(field).ok_or_else(|| {
                    invalid_structural("structural aggregate field has no closed native identity")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut identity = b"lkjscript.jit.structural-aggregate\0".to_vec();
        identity.extend_from_slice(&self.plan.bytes());
        identity.extend_from_slice(&type_id.raw().to_le_bytes());
        if let Some(variant) = variant_bytes {
            identity.extend_from_slice(&variant);
        }
        Ok(lkjscript_native::StructuralAggregateDescriptor::new(
            identity_word(&identity),
            value_type,
            kind,
            fields,
        ))
    }

    pub(in crate::lower) fn destination(
        &self,
        function: &Function,
        value: ValueId,
    ) -> Result<
        (
            lkjscript_native::StructuralAggregateDescriptor,
            lkjscript_native::StructuralStorageRoute,
            u16,
        ),
        LoweringError,
    > {
        let instruction = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find(|instruction| instruction.id == value)
            .ok_or_else(|| invalid_structural("structural destination definition is absent"))?;
        match instruction.kind {
            InstructionKind::DestinationCreate {
                representation,
                active_variant,
            } => {
                let (type_id, _) = self.representation(
                    representation,
                    lkjscript_ir::StructuralValueCategory::Destination,
                )?;
                let storage = self.representation_storage(
                    representation,
                    lkjscript_ir::StructuralValueCategory::Destination,
                )?;
                Ok((self.aggregate(type_id, active_variant)?, storage, 0))
            }
            InstructionKind::DestinationFieldInit {
                destination, field, ..
            } => {
                let (aggregate, storage, initialized) = self.destination(function, destination)?;
                if field != u64::from(initialized) {
                    return Err(invalid_structural(
                        "structural destination fields are not initialized in order",
                    ));
                }
                let next = initialized.checked_add(1).ok_or_else(|| {
                    invalid_structural("structural destination initialization overflow")
                })?;
                Ok((aggregate, storage, next))
            }
            _ => Err(invalid_structural(
                "structural destination value has the wrong definition",
            )),
        }
    }

    pub(in crate::lower) fn view(
        &self,
        root: lkjscript_native::StructuralTypeIdentity,
        projected: lkjscript_native::StructuralTypeIdentity,
        path: Vec<u16>,
        kind: lkjscript_native::StructuralProjectionKind,
        mutable: bool,
    ) -> lkjscript_native::StructuralProjectionDescriptor {
        let mut identity = b"lkjscript.jit.structural-projection\0".to_vec();
        identity.extend_from_slice(&self.plan.bytes());
        identity.extend_from_slice(&root.layout().to_le_bytes());
        identity.extend_from_slice(&projected.layout().to_le_bytes());
        identity.extend_from_slice(&[kind as u8]);
        for field in &path {
            identity.extend_from_slice(&field.to_le_bytes());
        }
        let view = lkjscript_native::StructuralViewType::new(
            identity_word(&identity),
            root,
            projected,
            mutable,
        );
        lkjscript_native::StructuralProjectionDescriptor::new(view, kind, path)
    }
}
