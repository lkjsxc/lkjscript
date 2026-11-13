//! One prerequisite traversal for pure trait checks and typed callable checks.
//! The caller supplies type equations first, then callable evidence only after
//! ordinary prerequisites succeed. Pausing never marks a pending goal proved.
use super::*;

pub(crate) struct BoundContext<'a> {
    pub(crate) source: &'a Program,
    pub(crate) index: &'a ImplementationIndex,
    pub(crate) types: &'a [crate::k::Type],
    pub(crate) applications: &'a BTreeMap<usize, (String, Vec<usize>)>,
    pub(crate) rigid: &'a BTreeMap<usize, BTreeSet<TraitIdentity>>,
}

pub(crate) enum BoundStep {
    Done(bool),
    Infer {
        goal: Goal,
        witness: ImplementationWitness,
    },
    Validate {
        goal: Goal,
        witness: ImplementationWitness,
        constraints: Vec<(TypeId, usize)>,
    },
}

enum Task {
    Enter(Goal),
    Finish(Goal, Requirements),
}

pub(crate) struct BoundSolver {
    first: Goal,
    state: BTreeMap<Goal, u8>,
    structural: BTreeSet<Goal>,
    inferred: BTreeMap<Goal, ImplementationWitness>,
    tasks: Vec<Task>,
    completed: Vec<(Goal, Option<ImplementationWitness>)>,
}
impl BoundSolver {
    pub(crate) fn new(ty: usize, bound: TraitIdentity) -> Self {
        let first = Goal { ty, bound };
        Self {
            tasks: vec![Task::Enter(first.clone())],
            first,
            state: BTreeMap::new(),
            structural: BTreeSet::new(),
            inferred: BTreeMap::new(),
            completed: Vec::new(),
        }
    }
    pub(crate) fn inferred(&mut self, goal: Goal, witness: Option<ImplementationWitness>) {
        if let Some(witness) = witness {
            self.inferred.insert(goal.clone(), witness);
            self.tasks.push(Task::Enter(goal));
        } else {
            self.state.insert(goal, 1);
        }
    }
    pub(crate) fn validated(&mut self, goal: Goal, witness: Option<ImplementationWitness>) {
        if let Some(witness) = witness {
            self.state.insert(goal.clone(), 2);
            self.completed.push((goal, Some(witness)));
        } else {
            self.state.insert(goal, 1);
        }
    }
    pub(crate) fn completed(&mut self) -> Vec<(Goal, Option<ImplementationWitness>)> {
        std::mem::take(&mut self.completed)
    }
    pub(crate) fn advance(&mut self, context: BoundContext<'_>) -> BoundStep {
        while let Some(task) = self.tasks.pop() {
            match task {
                Task::Finish(goal, requirements) => {
                    if self.state.get(&goal) == Some(&1) {
                        continue;
                    }
                    let valid = requirements.children.iter().all(|child| {
                        self.state.get(child) == Some(&2)
                            || (self.state.get(child) == Some(&0)
                                && self.structural.contains(child))
                    });
                    if !valid {
                        self.state.insert(goal, 1);
                        continue;
                    }
                    if !requirements.parameterized.is_empty() {
                        return BoundStep::Validate {
                            goal,
                            witness: requirements.witness.unwrap(),
                            constraints: requirements.parameterized,
                        };
                    }
                    self.state.insert(goal.clone(), 2);
                    self.completed.push((goal, requirements.witness));
                }
                Task::Enter(goal) => {
                    if let Some(previous) = self.state.get(&goal) {
                        if *previous == 0 && !self.structural.contains(&goal) {
                            self.state.insert(goal, 1);
                        }
                        continue;
                    }
                    let Some(requirements) = trait_requirements(
                        context.source,
                        context.index,
                        context.types,
                        context.applications,
                        &goal,
                        context.rigid,
                        &self.inferred,
                    ) else {
                        self.state.insert(goal, 1);
                        continue;
                    };
                    if requirements.infer {
                        return BoundStep::Infer {
                            goal,
                            witness: requirements.witness.unwrap(),
                        };
                    }
                    self.state.insert(goal.clone(), 0);
                    if requirements.structural {
                        self.structural.insert(goal.clone());
                    }
                    let children = requirements.children.clone();
                    self.tasks.push(Task::Finish(goal, requirements));
                    self.tasks
                        .extend(children.into_iter().rev().map(Task::Enter));
                }
            }
        }
        BoundStep::Done(self.state.get(&self.first) == Some(&2))
    }
}
