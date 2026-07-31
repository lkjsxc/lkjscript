type BranchResult = (
    Option<ValueId>,
    Option<BlockId>,
    BTreeMap<BindingId, ValueId>,
    Vec<ValueId>,
);

impl FunctionBuilder<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn merge_branches(
        &mut self,
        result_type: SsaType,
        expression_origin: hir::SourceId,
        incoming_env: BTreeMap<BindingId, ValueId>,
        incoming_slots: BTreeMap<BindingId, u16>,
        incoming_unplaced: Vec<ValueId>,
        then_result: BranchResult,
        else_result: BranchResult,
    ) -> Result<Option<ValueId>> {
        match (then_result.0, else_result.0) {
            (None, None) => {
                self.current = None;
                self.env = incoming_env;
                self.slots = incoming_slots;
                self.unplaced_owners = incoming_unplaced;
                Ok(None)
            }
            (Some(value), None) => {
                self.current = then_result.1;
                self.env = then_result.2;
                self.slots = incoming_slots;
                self.unplaced_owners = then_result.3;
                Ok(Some(value))
            }
            (None, Some(value)) => {
                self.current = else_result.1;
                self.env = else_result.2;
                self.slots = incoming_slots;
                self.unplaced_owners = else_result.3;
                Ok(Some(value))
            }
            (Some(then_value), Some(else_value)) => self.merge_live_branches(
                result_type,
                expression_origin,
                incoming_env,
                incoming_slots,
                then_result,
                else_result,
                then_value,
                else_value,
            ),
        }
    }

    fn drop_branch_residuals(
        &mut self,
        branch: &mut BranchResult,
        values: &[ValueId],
        slots: &BTreeMap<BindingId, u16>,
        expression_origin: hir::SourceId,
    ) -> Result<()> {
        self.current = branch.1;
        self.env = branch.2.clone();
        self.slots = slots.clone();
        self.unplaced_owners = branch.3.clone();
        for value in values {
            self.drop_unplaced_structural_owner(*value, expression_origin)?;
        }
        branch.1 = self.current;
        branch.2 = self.env.clone();
        branch.3 = self.unplaced_owners.clone();
        Ok(())
    }

    fn normalize_structural_branch_result(
        &mut self,
        branch: &mut BranchResult,
        value: ValueId,
        ty: &SsaType,
        slots: &BTreeMap<BindingId, u16>,
        expression_origin: hir::SourceId,
    ) -> Result<ValueId> {
        if branch.3.contains(&value) {
            return Ok(value);
        }
        self.current = branch.1;
        self.env = branch.2.clone();
        self.slots = slots.clone();
        self.unplaced_owners = branch.3.clone();
        let representation = self.structural_representation(ty, StructuralValueCategory::Owner)?;
        let copied = self.append(
            ty.clone(),
            InstructionKind::StructuralCopy {
                representation,
                value,
            },
            EffectSet::ALLOCATES,
            expression_origin,
        )?;
        branch.0 = Some(copied);
        branch.1 = self.current;
        branch.2 = self.env.clone();
        branch.3 = self.unplaced_owners.clone();
        Ok(copied)
    }
}
