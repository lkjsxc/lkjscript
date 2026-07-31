use lkjscript_core::{OwnedValue, SemanticPayload, SemanticValue, StructuralSnapshotLimits};

use crate::eval::{map_structural_error, structural_eligible, EvalValue, Evaluator, Flow};

impl Evaluator<'_> {
    pub(crate) fn adapt_return(&mut self, value: EvalValue) -> Result<crate::EvalOutcome, Flow> {
        let returned = match value {
            EvalValue::StructuralOwner(owner) => {
                let semantic = self
                    .structural
                    .runtime
                    .export_semantic(owner.key, owner.value_type)
                    .map_err(map_structural_error)?;
                self.owned_structural(semantic)?
            }
            EvalValue::StructuralView(view) | EvalValue::StructuralUtf8View(view) => {
                self.structural
                    .runtime
                    .end_view(view.key)
                    .map_err(map_structural_error)?;
                return Err(Flow::Trap(
                    "borrowed structural view escaped evaluation".into(),
                ));
            }
            EvalValue::StructuralDestination(destination) => {
                self.abort_structural_destination(destination.key);
                return Err(Flow::Trap(
                    "private structural destination escaped evaluation".into(),
                ));
            }
            value @ EvalValue::ByteVector(_) => {
                EvalValue::ReturnedByteVector(self.unique.export_owner(value)?)
            }
            value @ EvalValue::Bytes(_) => {
                EvalValue::ReturnedBytes(self.unique.export_owner(value)?)
            }
            value @ EvalValue::Path(_) => {
                let bytes = self.unique.export_owner(value)?;
                let value_type = self.structural_type(&crate::SsaType::Path)?;
                self.owned_structural(SemanticValue::new(value_type, SemanticPayload::Path(bytes)))?
            }
            EvalValue::StaticBytes(index) => {
                let bytes = self
                    .static_bytes
                    .get(index as usize)
                    .ok_or_else(|| Flow::Trap("invalid returned static bytes".into()))?;
                EvalValue::ReturnedBytes(copy_bytes(bytes)?)
            }
            EvalValue::StaticString(identity) => {
                let text = self
                    .structural
                    .static_string(identity)
                    .map_err(Flow::Trap)?;
                if structural_eligible(self.program.program(), &crate::SsaType::Str) {
                    let bytes = copy_bytes(text.as_bytes())?;
                    let value_type = self.structural_type(&crate::SsaType::Str)?;
                    self.owned_structural(SemanticValue::new(
                        value_type,
                        SemanticPayload::String(bytes),
                    ))?
                } else {
                    EvalValue::Str(text.to_owned())
                }
            }
            EvalValue::StaticSymbol(identity) => EvalValue::Symbol(
                self.structural
                    .static_symbol(identity)
                    .map_err(Flow::Trap)?
                    .to_owned(),
            ),
            EvalValue::Product(id, fields) => {
                EvalValue::Product(id, self.adapt_legacy_fields(fields)?)
            }
            EvalValue::Enum {
                enum_id,
                variant,
                layout,
                physical_tag,
                payload,
            } => EvalValue::Enum {
                enum_id,
                variant,
                layout,
                physical_tag,
                payload: self.adapt_legacy_fields(payload)?,
            },
            EvalValue::List(values) => EvalValue::List(self.adapt_legacy_fields(values)?),
            EvalValue::ReturnedOwned(_)
            | EvalValue::ReturnedByteVector(_)
            | EvalValue::ReturnedBytes(_) => {
                return Err(Flow::Trap("returned adapter re-entered evaluator".into()))
            }
            other => other,
        };
        Ok(crate::EvalOutcome::Returned(returned))
    }

    fn owned_structural(&self, value: SemanticValue) -> Result<EvalValue, Flow> {
        OwnedValue::from_structural(value, StructuralSnapshotLimits::DEFAULT)
            .map(EvalValue::ReturnedOwned)
            .map_err(|error| Flow::Trap(error.to_string()))
    }

    fn adapt_legacy_fields(&mut self, values: Vec<EvalValue>) -> Result<Vec<EvalValue>, Flow> {
        let mut output = Vec::new();
        output
            .try_reserve_exact(values.len())
            .map_err(|_| Flow::Resource("returned legacy adapter".into()))?;
        for value in values {
            let crate::EvalOutcome::Returned(value) = self.adapt_return(value)? else {
                return Err(Flow::Trap("invalid nested return adapter".into()));
            };
            if matches!(value, EvalValue::ReturnedOwned(_)) {
                return Err(Flow::Trap(
                    "legacy aggregate cannot return an owned structural child".into(),
                ));
            }
            output.push(value);
        }
        Ok(output)
    }
}

fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>, Flow> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| Flow::Resource("returned structural bytes".into()))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}
