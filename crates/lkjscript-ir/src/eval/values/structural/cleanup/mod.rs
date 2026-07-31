use lkjscript_core::{CleanupPhase, CleanupSubject, StructuralFieldPath, StructuralProjection};

use crate::eval::{
    map_structural_error, EvalStructuralOwner, EvalStructuralView, EvalValue, Evaluator, Flow,
};

impl Evaluator<'_> {
    pub(crate) fn move_eval_value(&mut self, value: EvalValue) -> Result<EvalValue, Flow> {
        match value {
            EvalValue::StructuralOwner(owner) => self
                .structural
                .runtime
                .move_owned(owner.key, owner.value_type)
                .map(|key| {
                    EvalValue::StructuralOwner(EvalStructuralOwner {
                        key,
                        value_type: owner.value_type,
                    })
                })
                .map_err(map_structural_error),
            EvalValue::StructuralView(view) | EvalValue::StructuralUtf8View(view) => Err(
                Flow::Trap(format!("structural view {} cannot move", view.key.get())),
            ),
            EvalValue::StructuralDestination(_) => Err(Flow::Trap(
                "private structural destination cannot move".into(),
            )),
            other => Ok(other),
        }
    }

    pub(crate) fn borrow_eval_value(
        &mut self,
        value: &EvalValue,
        exclusive: bool,
    ) -> Result<EvalValue, Flow> {
        let owner = match value {
            EvalValue::StructuralOwner(owner) => *owner,
            EvalValue::StructuralView(view) if !exclusive && view.root_type == view.value_type => {
                EvalStructuralOwner {
                    key: view.owner,
                    value_type: view.root_type,
                }
            }
            EvalValue::StructuralUtf8View(_) => {
                return Err(Flow::Trap("UTF-8 view cannot be borrowed again".into()))
            }
            _ => return self.unique.borrow(value, exclusive),
        };
        let key = self
            .structural
            .runtime
            .borrow_projected(
                owner.key,
                owner.value_type,
                StructuralProjection::Field {
                    path: StructuralFieldPath::root(),
                    expected: owner.value_type,
                },
                exclusive,
            )
            .map_err(map_structural_error)?;
        Ok(EvalValue::StructuralView(EvalStructuralView {
            owner: owner.key,
            key,
            root_type: owner.value_type,
            value_type: owner.value_type,
        }))
    }

    pub(crate) fn end_eval_borrow(&mut self, value: EvalValue) -> Result<(), Flow> {
        match value {
            EvalValue::StructuralView(view) | EvalValue::StructuralUtf8View(view) => self
                .structural
                .runtime
                .end_view(view.key)
                .map_err(map_structural_error),
            other => self.unique.end_borrow(other),
        }
    }

    pub(crate) fn cleanup_eval_value(&mut self, value: EvalValue) -> Result<(), Flow> {
        match value {
            EvalValue::StructuralOwner(owner) => self
                .structural
                .runtime
                .drop_owned(owner.key, owner.value_type)
                .map_err(map_structural_error),
            EvalValue::StructuralView(view) | EvalValue::StructuralUtf8View(view) => self
                .structural
                .runtime
                .end_view(view.key)
                .map_err(map_structural_error),
            EvalValue::StructuralDestination(destination) => self
                .structural
                .runtime
                .abort_destination(destination.key)
                .map(|_| ())
                .map_err(map_structural_error),
            value @ (EvalValue::ByteVector(_) | EvalValue::Bytes(_) | EvalValue::Path(_)) => {
                self.unique.drop_owner(value)
            }
            EvalValue::Resource(resource) => {
                let kind = resource.kind();
                if matches!(
                    kind,
                    lkjscript_core::ResourceKind::InputStream
                        | lkjscript_core::ResourceKind::OutputStream
                ) {
                    Ok(())
                } else {
                    self.resources
                        .drop_owned(resource, kind)
                        .map_err(Flow::Trap)
                }
            }
            EvalValue::Product(_, fields) | EvalValue::List(fields) => {
                self.cleanup_legacy_values_reverse(fields)
            }
            EvalValue::Enum { payload, .. } => self.cleanup_legacy_values_reverse(payload),
            _ => Ok(()),
        }
    }

    pub(crate) fn cleanup_frame_values(&mut self, values: &mut [Option<EvalValue>]) {
        for slot in values.iter_mut().rev() {
            if let Some(value) = slot.take() {
                if let Err(flow) = self.cleanup_eval_value(value) {
                    self.note_structural_cleanup_failure(flow.detail());
                }
            }
        }
    }

    pub(crate) fn cleanup_frame_structural_values(&mut self, values: &mut [Option<EvalValue>]) {
        for slot in values.iter_mut().rev() {
            let cleanup = slot.as_ref().is_some_and(requires_structural_frame_cleanup);
            if cleanup {
                if let Some(value) = slot.take() {
                    if let Err(flow) = self.cleanup_eval_value(value) {
                        self.note_structural_cleanup_failure(flow.detail());
                    }
                }
            }
        }
    }

    pub(crate) fn note_structural_cleanup_failure(&mut self, message: String) {
        self.cleanup_failures.push(
            CleanupPhase::Ordinary,
            CleanupSubject::UniqueStorage,
            format!("structural value cleanup: {message}"),
        );
    }

    pub(crate) fn cleanup_legacy_values_reverse(
        &mut self,
        values: Vec<EvalValue>,
    ) -> Result<(), Flow> {
        let mut first = None;
        for value in values.into_iter().rev() {
            if let Err(error) = self.cleanup_eval_value(value) {
                if first.is_none() {
                    first = Some(error);
                } else {
                    self.note_structural_cleanup_failure(error.detail());
                }
            }
        }
        match first {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

fn requires_structural_frame_cleanup(value: &EvalValue) -> bool {
    match value {
        EvalValue::StructuralOwner(_)
        | EvalValue::StructuralView(_)
        | EvalValue::StructuralUtf8View(_)
        | EvalValue::StructuralDestination(_) => true,
        EvalValue::Enum { payload, .. }
        | EvalValue::Product(_, payload)
        | EvalValue::List(payload) => payload.iter().any(|value| {
            matches!(value, EvalValue::Resource(_)) || requires_structural_frame_cleanup(value)
        }),
        _ => false,
    }
}

mod call;
mod slot;
pub(crate) use slot::restore_slot;
