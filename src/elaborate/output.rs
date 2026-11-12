//! Capability methods are explicit K operations; neither source spelling nor an
//! imported host function grants authority without a checked capability value.
use super::{Builder, Result, a, err, k};

impl Builder<'_, '_> {
    pub(super) fn output_method(
        &mut self,
        target: k::Place,
        base: usize,
        name: &str,
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let (operation, result) = match (&self.compiler.types[base].kind, name) {
            (k::TypeKind::World, "stdout") if args.is_empty() => (
                "world_stdout",
                self.compiler.intern(String::new(), k::TypeKind::Stdout),
            ),
            (k::TypeKind::Stdout, "write") if args.len() == 1 => ("stdout_write", k::UNIT),
            _ => {
                return Err(err(
                    span,
                    "E_TYPE",
                    "World::stdout takes no arguments; Stdout::write takes &[u8]",
                ));
            }
        };
        let receiver = self.borrow(target, true, span.line)?.0;
        let mut values = vec![receiver];
        if operation == "stdout_write" {
            let slice = self.compiler.intern(
                String::new(),
                k::TypeKind::Slice {
                    inner: k::U8,
                    mutable: false,
                },
            );
            let bytes = self.compiler.reference(slice, false);
            values.push(self.expr(args[0], Some(bytes))?.0);
        }
        let dst = self.temp(result);
        self.emit(
            k::Op::Intrinsic {
                dst,
                name: operation.to_string(),
                args: values,
            },
            span.line,
        );
        Ok((dst, result))
    }
}
