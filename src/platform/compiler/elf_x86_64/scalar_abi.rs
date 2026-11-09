//! Boxed callable boundaries around native graph entries with raw I64 arguments.
//!
//! Direct graph calls share one signature-defined ABI even when a function keeps
//! boxed internal slots. Immutable callable prefixes and aggregate fields retain
//! the existing boxed representation.
use super::asm::{Label, R::*, *};
use super::{
    Emitter, NormalizedFunctionBody, Result, TypeForm, layout_extent, layout_offset, unsupported,
};

impl Emitter<'_> {
    pub(super) fn raw_parameter(&self, target: usize, parameter: usize) -> bool {
        self.program.functions.get(target).is_some_and(|function| {
            matches!(&function.body, NormalizedFunctionBody::Code(_))
                && function.parameters.get(parameter).is_some_and(|parameter| {
                    matches!(
                        self.program.types.get(&parameter.ty).map(|ty| &ty.form),
                        Some(TypeForm::I64)
                    )
                })
        })
    }

    pub(super) fn raw_result(&self, target: usize) -> bool {
        self.program.functions.get(target).is_some_and(|function| {
            matches!(&function.body, NormalizedFunctionBody::Code(_))
                && matches!(
                    self.program
                        .types
                        .get(&function.result.ty)
                        .map(|ty| &ty.form),
                    Some(TypeForm::I64)
                )
        })
    }

    pub(super) fn callable_label(&self, target: usize) -> Result<Label> {
        if let Some(label) = self.callable_entries.get(target).copied().flatten() {
            Ok(label)
        } else {
            self.label(target)
        }
    }

    pub(super) fn callable_body(&self, target: usize) -> Option<Label> {
        self.callable_bodies
            .get(target)
            .copied()
            .flatten()
            .or_else(|| self.bodies.get(target).copied().flatten())
    }

    pub(super) fn emit_callable_adapters(&mut self, index: usize) -> Result<()> {
        let ordinary = self.callable_entries.get(index).copied().flatten();
        let tail = self.callable_bodies.get(index).copied().flatten();
        if ordinary.is_none() && tail.is_none() {
            return Ok(());
        }
        let function = self
            .program
            .functions
            .get(index)
            .ok_or_else(|| unsupported("native callable adapter target is absent"))?;
        if !matches!(&function.body, NormalizedFunctionBody::Code(_)) {
            return Err(unsupported(
                "native scalar callable adapters require a graph body",
            ));
        }
        let parameters = function.parameter_count;
        let result_is_raw = self.raw_result(index);

        if let Some(entry) = ordinary {
            self.a.bind(entry);
            self.runtime.checkpoint(&mut self.a);
            self.prologue(0);

            // Reserve all outgoing slots before any polling call can place its
            // call record below SP. The caller's boxed argument window remains
            // untouched and is still addressed through this wrapper's RBP.
            let extent = layout_extent(parameters as usize, 8, 0)?;
            if extent != 0 {
                self.a.mov(Ax, Sp);
                self.a.group_wide(5, Ax, extent);
                self.a.when(B, self.runtime.capacity);
                self.a.mov(Sp, Ax);
            }
            for parameter in 0..parameters {
                if parameter.is_multiple_of(256) {
                    self.runtime.checkpoint(&mut self.a);
                }
                let offset = parameters - 1 - parameter;
                self.argument(Ax, offset);
                if self.raw_parameter(index, parameter as usize) {
                    self.a.load(Ax, Ax, 8);
                }
                self.a
                    .store_offset(Sp, layout_offset(offset as usize, 8, 0)?, Ax);
            }
            self.a.call(self.label(index)?);
            if result_is_raw {
                self.a.call(self.runtime.integer);
            }
            // The normal epilogue removes the complete outgoing window and
            // restores the original BX/R12/R13/RBP before returning boxed Ax.
            self.epilogue();
        }

        if let Some(entry) = tail {
            let destination = self
                .bodies
                .get(index)
                .copied()
                .flatten()
                .ok_or_else(|| unsupported("native callable tail adapter has no graph body"))?;
            self.a.bind(entry);
            self.runtime.checkpoint(&mut self.a);
            for parameter in 0..parameters {
                if parameter.is_multiple_of(256) {
                    self.runtime.checkpoint(&mut self.a);
                }
                if self.raw_parameter(index, parameter as usize) {
                    let offset = layout_offset(parameter as usize, -8, -32)?;
                    self.a.load_offset(Ax, Bp, offset);
                    self.a.load(Ax, Ax, 8);
                    self.a.store_offset(Bp, offset, Ax);
                }
            }
            // tail_frame already installed the whole target frame. This entry
            // converts only incoming boxed parameter locals, then shares the
            // direct-tail adapter, including any internal boxed fallback.
            self.a.jump(destination);
        }
        Ok(())
    }
}
