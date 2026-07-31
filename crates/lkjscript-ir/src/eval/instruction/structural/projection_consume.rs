impl Evaluator<'_> {
    fn consume_structural_payload(
        &mut self,
        values: &mut [Option<EvalValue>],
        source: ValueId,
        representation: crate::StructuralRepresentationId,
        variant: VariantId,
        result_ty: &crate::SsaType,
    ) -> Result<EvalValue, Flow> {
        let facts = self.representation_facts(
            representation,
            crate::StructuralValueCategory::Owner,
        )?;
        let value_type = self.structural_type(&facts.ty)?;
        let (tag, fields) = destination_fields(&facts, Some(variant))?;
        let tag = tag.ok_or_else(|| Flow::Trap("payload consume expects enum".into()))?;
        if fields.as_slice() != [result_ty.clone()] {
            return Err(Flow::Trap(
                "payload consume requires one exact whole payload".into(),
            ));
        }
        let EvalValue::StructuralOwner(owner) = value(values, source)? else {
            return Err(Flow::Trap(
                "payload consume expects structural owner".into(),
            ));
        };
        if owner.value_type != value_type {
            return Err(Flow::Trap("payload consume owner type mismatch".into()));
        }
        match self
            .structural
            .runtime
            .value_node(owner.key, value_type)
            .map_err(map_structural_error)?
            .payload()
        {
            StructuralNodeView::Enum {
                tag: actual,
                fields,
            } if actual == tag && fields.len() == 1 => {}
            StructuralNodeView::Enum { .. } => {
                return Err(Flow::Trap(
                    "payload consume selected inactive variant".into(),
                ))
            }
            _ => return Err(Flow::Trap("payload consume expects enum payload".into())),
        }
        let removed = take_value(values, source)?;
        let EvalValue::StructuralOwner(owner) = removed else {
            return Err(Flow::Trap(
                "payload consume owner changed during transfer".into(),
            ));
        };
        let semantic = self
            .structural
            .runtime
            .export_semantic(owner.key, value_type)
            .map_err(map_structural_error)?;
        let lkjscript_core::SemanticPayload::Enum {
            tag: _,
            mut active_payload,
        } = semantic.payload
        else {
            return Err(Flow::Trap("payload consume export changed shape".into()));
        };
        let payload = active_payload
            .pop()
            .ok_or_else(|| Flow::Trap("payload consume export is empty".into()))?;
        self.semantic_to_eval(payload)
    }
}
