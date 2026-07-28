use lkjscript_core::{CleanupPhase, CleanupSubject};

use crate::{DropGlueIdentity, FailureCleanupAction, FailureCleanupId, Function};

use super::*;

impl Evaluator<'_> {
    pub(crate) fn execute_unentered_instruction_cleanup(
        &mut self,
        function: &Function,
        instruction: &crate::Instruction,
        values: &mut [Option<EvalValue>],
    ) {
        let crate::InstructionKind::Call { arguments, .. } = &instruction.kind else {
            return;
        };
        let transferred: Vec<_> = arguments
            .iter()
            .copied()
            .filter(|argument| {
                function.blocks.iter().any(|block| {
                    block.instructions.iter().any(|candidate| {
                        candidate.id == *argument
                            && matches!(candidate.kind, crate::InstructionKind::Move { .. })
                    })
                })
            })
            .collect();
        let mut owned = Vec::with_capacity(transferred.len());
        for value in transferred {
            match take_cleanup_value(values, value) {
                Ok(value) => owned.push(value),
                Err(message) => self.cleanup_failures.push(
                    CleanupPhase::Ordinary,
                    CleanupSubject::UniqueStorage,
                    message,
                ),
            }
        }
        self.execute_unentered_argument_cleanup(owned);
    }

    pub(crate) fn execute_unentered_argument_cleanup(&mut self, arguments: Vec<EvalValue>) {
        for value in arguments.into_iter().rev() {
            match value {
                value @ (EvalValue::ByteVector(_) | EvalValue::Bytes(_)) => {
                    if let Err(flow) = self.unique.drop_owner(value) {
                        self.cleanup_failures.push(
                            CleanupPhase::Ordinary,
                            CleanupSubject::UniqueStorage,
                            flow.detail(),
                        );
                    }
                }
                EvalValue::Resource(resource) => {
                    let kind = resource.kind();
                    if matches!(
                        kind,
                        lkjscript_core::ResourceKind::InputStream
                            | lkjscript_core::ResourceKind::OutputStream
                    ) {
                        continue;
                    }
                    if let Err(message) = self.resources.drop_owned(resource, kind) {
                        self.cleanup_failures.push(
                            CleanupPhase::Ordinary,
                            CleanupSubject::Resource(kind),
                            message,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn execute_failure_cleanup(
        &mut self,
        function: &Function,
        cleanup: Option<FailureCleanupId>,
        values: &mut [Option<EvalValue>],
    ) {
        let Some(cleanup) = cleanup else {
            return;
        };
        let Some(plan) = function
            .failure_cleanups
            .get(cleanup.index().unwrap_or(usize::MAX))
            .filter(|plan| plan.id == cleanup)
        else {
            self.cleanup_failures.push(
                CleanupPhase::Ordinary,
                CleanupSubject::UniqueStorage,
                "verified evaluator failure-cleanup plan is unavailable",
            );
            return;
        };
        let actions = plan.actions.clone();
        for action in actions {
            match action {
                FailureCleanupAction::EndBorrow { value, .. } => {
                    let result = take_cleanup_value(values, value)
                        .and_then(|value| self.unique.end_borrow(value).map_err(Flow::detail));
                    if let Err(message) = result {
                        self.cleanup_failures.push(
                            CleanupPhase::Ordinary,
                            CleanupSubject::UniqueStorage,
                            message,
                        );
                    }
                }
                FailureCleanupAction::DropOwner { value, glue, .. } => match glue {
                    DropGlueIdentity::ByteVector | DropGlueIdentity::Bytes => {
                        let result = take_cleanup_value(values, value)
                            .and_then(|value| self.unique.drop_owner(value).map_err(Flow::detail));
                        if let Err(message) = result {
                            self.cleanup_failures.push(
                                CleanupPhase::Ordinary,
                                CleanupSubject::UniqueStorage,
                                message,
                            );
                        }
                    }
                    DropGlueIdentity::Resource(kind) => {
                        let result = take_cleanup_value(values, value).and_then(|value| {
                            let EvalValue::Resource(resource) = value else {
                                return Err(
                                    "evaluator failure cleanup expected a resource owner".into()
                                );
                            };
                            self.resources.drop_owned(resource, kind)
                        });
                        if let Err(message) = result {
                            self.cleanup_failures.push(
                                CleanupPhase::Ordinary,
                                CleanupSubject::Resource(kind),
                                message,
                            );
                        }
                    }
                },
            }
        }
    }
}

fn take_cleanup_value(
    values: &mut [Option<EvalValue>],
    value: ValueId,
) -> Result<EvalValue, String> {
    values
        .get_mut(value.index().unwrap_or(usize::MAX))
        .and_then(Option::take)
        .ok_or_else(|| format!("evaluator failure cleanup lost SSA value {}", value.raw()))
}
