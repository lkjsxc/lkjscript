use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_match(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [scrutinee_form, arms_form] = args else {
            return Err(self.error("match expects exactly one scrutinee and arms/"));
        };
        let arm_forms = match arms_form {
            AstExpr::Call { name, args } if name == "arms" && !args.is_empty() => args,
            AstExpr::Call { name, .. } if name == "arms" => {
                return Err(self.error("match arms/ must not be empty"));
            }
            _ => return Err(self.error("match expects arms/ second")),
        };
        let scrutinee_value = self.resolve_expr(scrutinee_form)?;
        if !matches!(
            scrutinee_value.ty,
            Type::Bool | Type::I64 | Type::Enum { .. } | Type::Product(_)
        ) {
            return Err(self.error(format!(
                "type {} has no closed pattern space",
                scrutinee_value.ty,
            )));
        }
        let outer_slot = self.next_slot;
        let scrutinee = self.allocate_hidden_match_local(scrutinee_value.ty.clone())?;
        let arm_slot = self.next_slot;
        let (planned, bodies) = self.resolve_match_arms(arm_forms, &scrutinee, arm_slot)?;
        let plan_id = MatchPlanId::new(
            u64::try_from(self.analyzer.match_plans.len())
                .map_err(|_| self.error("match plan identity exceeds u64"))?,
        );
        let plan = super::plan::build_plan(
            plan_id,
            Origin::Source(self.origin),
            scrutinee,
            planned,
            &self.analyzer.enums,
            &self.analyzer.products,
        )?;
        let result_type = plan.result_type.clone();
        self.analyzer
            .match_plans
            .try_reserve(1)
            .map_err(|_| Error::host("match plan allocation failed"))?;
        self.analyzer.match_plans.push(plan);
        self.next_slot = outer_slot;
        Ok(self.expression(
            result_type,
            ExprKind::Match {
                plan: plan_id,
                scrutinee: Box::new(scrutinee_value),
                arms: bodies,
            },
        ))
    }

    fn resolve_match_arms(
        &mut self,
        forms: &[AstExpr],
        scrutinee: &MatchLocal,
        arm_slot: usize,
    ) -> Result<(Vec<PlannedMatchArm>, Vec<Expr>)> {
        let mut planned = Vec::new();
        planned
            .try_reserve(forms.len())
            .map_err(|_| Error::host("planned match arm allocation failed"))?;
        let mut bodies: Vec<Expr> = Vec::new();
        bodies
            .try_reserve(forms.len())
            .map_err(|_| Error::host("match arm body allocation failed"))?;
        for (index, form) in forms.iter().enumerate() {
            let AstExpr::Call { name, args } = form else {
                return Err(self.error("arms/ contains only arm/ forms"));
            };
            let [pattern_form, body_form] = args.as_slice() else {
                return Err(self.error("arm/ expects exactly one pattern and one body"));
            };
            if name != "arm" {
                return Err(self.error("arms/ contains only arm/ forms"));
            }
            let id =
                u64::try_from(index).map_err(|_| self.error("match arm identity exceeds u64"))?;
            self.scopes.push(HashMap::new());
            self.next_slot = arm_slot;
            let pattern = self.parse_match_pattern(pattern_form, &scrutinee.ty)?;
            let body = self.resolve_expr(body_form)?;
            self.scopes.pop();
            self.next_slot = arm_slot;
            planned.push(PlannedMatchArm {
                id,
                pattern,
                body_type: body.ty.clone(),
            });
            bodies.push(body);
        }
        Ok((planned, bodies))
    }
}
