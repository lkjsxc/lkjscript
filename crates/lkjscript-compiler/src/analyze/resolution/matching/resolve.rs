use super::usefulness::Usefulness;
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
        let mut result_type = Type::Never;
        for body in &bodies {
            result_type = Type::join_control(&result_type, &body.ty).ok_or_else(|| {
                self.error(format!(
                    "reachable match arm types must be exactly equal: {} vs {}",
                    result_type, body.ty
                ))
            })?;
        }
        self.check_usefulness(&planned, &scrutinee)?;
        let plan_id = MatchPlanId::new(
            u64::try_from(self.analyzer.match_plans.len())
                .map_err(|_| self.error("match plan identity exceeds u64"))?,
        );
        let (tests, projections, bindings) = super::plan::flatten_plan(&planned)?;
        let edge_capacity = planned
            .len()
            .checked_add(1)
            .ok_or_else(|| Error::host("match edge count overflow"))?;
        let mut edges = Vec::new();
        edges
            .try_reserve(edge_capacity)
            .map_err(|_| Error::host("match edge allocation failed"))?;
        for index in 1..planned.len() {
            edges.push(MatchEdgeTarget::Arm(
                u64::try_from(index).map_err(|_| self.error("match arm index exceeds u64"))?,
            ));
        }
        edges.extend([MatchEdgeTarget::Default, MatchEdgeTarget::Unreachable]);
        let mut lowered =
            self.expression(Type::Never, ExprKind::MatchUnreachable { plan: plan_id });
        for (arm, body) in planned.iter().zip(bodies).rev() {
            let value = self.match_load(&scrutinee);
            let condition = self.match_condition(&arm.pattern, value.clone())?;
            let success = self.match_success(&arm.pattern, value, body)?;
            lowered = self.match_if(condition, success, lowered);
        }
        lowered = self.local_scope(&scrutinee, scrutinee_value, lowered);
        self.analyzer
            .match_plans
            .try_reserve(1)
            .map_err(|_| Error::host("match plan allocation failed"))?;
        self.analyzer.match_plans.push(MatchPlan {
            id: plan_id,
            origin: self.origin,
            scrutinee,
            result_type,
            arms: planned,
            tests,
            projections,
            bindings,
            edges,
            exhaustive: true,
            witness: None,
        });
        self.next_slot = outer_slot;
        Ok(lowered)
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

    fn check_usefulness(&self, planned: &[PlannedMatchArm], scrutinee: &MatchLocal) -> Result<()> {
        let mut matrix = Vec::new();
        matrix
            .try_reserve(planned.len())
            .map_err(|_| Error::host("match usefulness input matrix allocation failed"))?;
        let mut usefulness = Usefulness::new(&self.analyzer.enums, &self.analyzer.products)?;
        for arm in planned {
            let candidate = [&arm.pattern];
            if usefulness
                .useful(&matrix, &candidate, std::slice::from_ref(&scrutinee.ty))?
                .is_none()
            {
                return Err(self.error(format!("useless or subsumed match arm {}", arm.id)));
            }
            matrix.push(&arm.pattern);
        }
        let wildcard = MatchPattern::Wildcard {
            ty: scrutinee.ty.clone(),
        };
        if let Some(witness) =
            usefulness.useful(&matrix, &[&wildcard], std::slice::from_ref(&scrutinee.ty))?
        {
            let root = *witness
                .first()
                .ok_or_else(|| Error::msg("match usefulness returned an empty witness"))?;
            let rendered = usefulness.render_witness(root)?;
            return Err(self.error(format!(
                "nonexhaustive match; canonical typed witness: {rendered}",
            )));
        }
        Ok(())
    }
}
