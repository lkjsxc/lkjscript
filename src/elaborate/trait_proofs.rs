//! Typed callable prerequisites extend the ordinary iterative trait solver.
//! Type equations do not prove a bound; full callable evidence is checked only
//! after every ordinary prerequisite. Session evidence is discarded afterward.
use super::{Compiler, Result, a, err};
use crate::generics::{
    BoundContext, BoundSolver, BoundStep, Goal, ImplementationWitness, TraitIdentity,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Session {
    active: BTreeSet<Goal>,
    proved: BTreeMap<Goal, Option<ImplementationWitness>>,
}

impl Compiler<'_> {
    fn infer_implementation(
        &mut self,
        mut witness: ImplementationWitness,
    ) -> Result<ImplementationWitness> {
        let parameters = self.source.impls[witness.implementation].generics.clone();
        self.prepare_callable_arguments(&parameters, &mut witness.substitutions)?;
        Ok(witness)
    }

    fn validate_implementation(
        &mut self,
        goal: &Goal,
        mut witness: ImplementationWitness,
        constraints: &[(a::TypeId, usize)],
    ) -> Result<ImplementationWitness> {
        let parameters = self.source.impls[witness.implementation].generics.clone();
        witness.substitutions.insert("Self".to_string(), goal.ty);
        // Child proofs are already available in this session. Resolving an
        // associated projection consumes that evidence, not a trait-name guess.
        self.prepare_parameter_associated(&parameters, &mut witness.substitutions)?;
        for (bound, ty) in constraints {
            let node = self.source.types[*bound].clone();
            let a::Type::Path(path) = node.kind else {
                return Err(err(
                    node.span,
                    "E_TRAIT",
                    "invalid parameterized trait prerequisite",
                ));
            };
            if !self.callable_bound(*ty, &path, &witness.substitutions, node.span)? {
                return Err(err(
                    node.span,
                    "E_UNSUPPORTED",
                    "parameterized trait prerequisite needs explicit evidence",
                ));
            }
        }
        Ok(witness)
    }

    fn prove_identity(
        &mut self,
        ty: usize,
        bound: &TraitIdentity,
    ) -> Option<Option<ImplementationWitness>> {
        let goal = Goal {
            ty,
            bound: bound.clone(),
        };
        let outermost = self.trait_proofs.is_none();
        if self.trait_proofs.is_none() {
            self.trait_proofs = Some(Session::default());
        }
        let session = self.trait_proofs.as_mut().unwrap();
        if let Some(proof) = session.proved.get(&goal) {
            return Some(proof.clone());
        }
        if !session.active.insert(goal.clone()) {
            return None;
        }
        let mut solver = BoundSolver::new(ty, bound.clone());
        let result = loop {
            let empty = BTreeMap::new();
            let rigid = self
                .contracts
                .as_ref()
                .map_or(&empty, |contracts| &contracts.rigid);
            let step = solver.advance(BoundContext {
                source: self.source,
                index: &self.implementations,
                types: &self.types,
                applications: &self.applications,
                rigid,
            });
            for (completed_goal, evidence) in solver.completed() {
                self.trait_proofs
                    .as_mut()
                    .unwrap()
                    .proved
                    .insert(completed_goal, evidence);
            }
            match step {
                BoundStep::Done(valid) => {
                    break if valid {
                        Some(
                            self.trait_proofs
                                .as_ref()
                                .unwrap()
                                .proved
                                .get(&goal)
                                .unwrap()
                                .clone(),
                        )
                    } else {
                        None
                    };
                }
                BoundStep::Infer { goal, witness } => {
                    let completed = self.infer_implementation(witness).ok();
                    solver.inferred(goal, completed);
                }
                BoundStep::Validate {
                    goal,
                    witness,
                    constraints,
                } => {
                    let completed = self
                        .validate_implementation(&goal, witness, &constraints)
                        .ok();
                    solver.validated(goal, completed);
                }
            }
        };
        self.trait_proofs.as_mut().unwrap().active.remove(&goal);
        if outermost {
            self.trait_proofs = None;
        }
        result
    }

    pub(super) fn satisfies_bound(&mut self, ty: usize, bound: &str) -> bool {
        self.satisfies_identity(ty, &TraitIdentity::Builtin(bound.to_string()))
    }
    pub(super) fn satisfies_identity(&mut self, ty: usize, bound: &TraitIdentity) -> bool {
        self.prove_identity(ty, bound).is_some()
    }
    pub(super) fn implementation_witness(
        &mut self,
        ty: usize,
        bound: &TraitIdentity,
    ) -> Option<ImplementationWitness> {
        let proof = self.prove_identity(ty, bound)?;
        proof.or_else(|| {
            crate::generics::select_implementation(
                self.source,
                &self.implementations,
                &self.types,
                &self.applications,
                ty,
                bound,
            )
        })
    }
}
