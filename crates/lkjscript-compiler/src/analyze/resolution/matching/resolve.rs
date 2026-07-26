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
                "type {:?} has no closed Edition 2 pattern space",
                scrutinee_value.ty,
            )));
        }
        let outer_slot = self.next_slot;
        let scrutinee = self.allocate_hidden_match_local(scrutinee_value.ty.clone())?;
        let arm_slot = self.next_slot;
        let (planned, bodies) = self.resolve_match_arms(arm_forms, &scrutinee, arm_slot)?;
        let result_type = bodies
            .first()
            .ok_or_else(|| self.error("match lost all arms"))?
            .ty
            .clone();
        let patterns: Vec<_> = planned.iter().map(|arm| arm.pattern.clone()).collect();
        let charges = super::charges::plan(&patterns, planned.len())?;
        self.check_usefulness(&planned, &scrutinee, &charges)?;
        let plan_id = MatchPlanId::new(
            u32::try_from(self.analyzer.match_plans.len())
                .map_err(|_| self.error("match plan count exceeds u32"))?,
        );
        let (tests, projections, bindings) = super::plan::flatten_plan(&planned);
        let mut edges = (1..planned.len())
            .map(|index| MatchEdgeTarget::Arm(u16::try_from(index).unwrap_or(u16::MAX)))
            .collect::<Vec<_>>();
        edges.extend([MatchEdgeTarget::Default, MatchEdgeTarget::Unreachable]);
        self.analyzer.match_plans.push(MatchPlan {
            id: plan_id,
            origin: self.origin,
            scrutinee: scrutinee.clone(),
            result_type: result_type.clone(),
            arms: planned.clone(),
            tests,
            projections,
            bindings,
            edges,
            exhaustive: true,
            witness: None,
            charges,
        });
        let mut lowered =
            self.expression(result_type, ExprKind::MatchUnreachable { plan: plan_id });
        for (arm, body) in planned.iter().zip(bodies).rev() {
            let value = self.match_load(&scrutinee);
            let condition = self.match_condition(&arm.pattern, value.clone())?;
            let success = self.match_success(&arm.pattern, value, body)?;
            lowered = self.match_if(condition, success, lowered);
        }
        lowered = self.local_scope(&scrutinee, scrutinee_value, lowered);
        self.next_slot = outer_slot;
        Ok(lowered)
    }

    fn resolve_match_arms(
        &mut self,
        forms: &[AstExpr],
        scrutinee: &MatchLocal,
        arm_slot: usize,
    ) -> Result<(Vec<PlannedMatchArm>, Vec<Expr>)> {
        let mut planned = Vec::with_capacity(forms.len());
        let mut bodies: Vec<Expr> = Vec::with_capacity(forms.len());
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
                u16::try_from(index).map_err(|_| self.error("match has more than 65535 arms"))?;
            self.scopes.push(HashMap::new());
            self.next_slot = arm_slot;
            let pattern = self.parse_match_pattern(pattern_form, &scrutinee.ty)?;
            let body = self.resolve_expr(body_form)?;
            self.scopes.pop();
            self.next_slot = arm_slot;
            if bodies.first().is_some_and(|first| body.ty != first.ty) {
                return Err(self.error(format!(
                    "match arm types must be exactly equal: {:?} vs {:?}",
                    bodies[0].ty, body.ty,
                )));
            }
            planned.push(PlannedMatchArm {
                id,
                pattern,
                body_type: body.ty.clone(),
            });
            bodies.push(body);
        }
        Ok((planned, bodies))
    }

    fn check_usefulness(
        &self,
        planned: &[PlannedMatchArm],
        scrutinee: &MatchLocal,
        charges: &MatchPlanCharges,
    ) -> Result<()> {
        let mut matrix: Vec<Vec<MatchPattern>> = Vec::with_capacity(planned.len());
        for arm in planned {
            let mut useful = Usefulness::new(
                &self.analyzer.enums,
                &self.analyzer.products,
                charges.specialization_work,
            );
            if useful
                .useful(
                    &matrix,
                    std::slice::from_ref(&arm.pattern),
                    std::slice::from_ref(&scrutinee.ty),
                )?
                .is_none()
            {
                return Err(self.error(format!("useless or subsumed match arm {}", arm.id)));
            }
            matrix.push(vec![arm.pattern.clone()]);
        }
        let mut useful = Usefulness::new(
            &self.analyzer.enums,
            &self.analyzer.products,
            charges.specialization_work,
        );
        if let Some(witness) = useful.useful(
            &matrix,
            &[MatchPattern::Wildcard {
                ty: scrutinee.ty.clone(),
            }],
            std::slice::from_ref(&scrutinee.ty),
        )? {
            let rendered = super::witness::render(&witness[0]);
            let bytes = u64::try_from(rendered.len())
                .map_err(|_| self.error("match witness byte count exceeds u64"))?;
            if bytes > charges.witness_bytes {
                return Err(
                    self.error("canonical match witness exceeded its pre-allocation reservation")
                );
            }
            return Err(self.error(format!(
                "nonexhaustive match; canonical typed witness: {rendered}",
            )));
        }
        Ok(())
    }
}
