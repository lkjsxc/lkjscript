//! Default derives expand into an ordinary source trait implementation.
use super::*;
impl Parser {
    pub(super) fn derive_default(&mut self, index: usize) -> Result<(), Diagnostic> {
        let definition = self.program.structs[index].clone();
        let mut requested = 0;
        for attr in &definition.attrs {
            if attr.name == "derive" {
                for name in &attr.args {
                    if name == "Default" {
                        requested += 1;
                    }
                }
            }
        }
        if requested == 0 {
            return Ok(());
        }
        if requested != 1 {
            return Err(Diagnostic::new(
                definition.span,
                "E_TRAIT",
                "duplicate derive Default",
            ));
        }
        let span = definition.span;
        let bound = self.add_type(
            Type::Path(Path {
                declaration: None,
                name: "std::default::Default".to_string(),
                args: Vec::new(),
            }),
            span,
        );
        let mut generics = definition.generics.clone();
        let mut arguments = Vec::new();
        let mut parameter_index = 0;
        while parameter_index < generics.len() {
            let parameter = &mut generics[parameter_index];
            if !parameter.lifetime {
                parameter.bounds.push(bound);
            }
            arguments.push(self.add_type(
                Type::Path(Path {
                    declaration: None,
                    name: parameter.name.clone(),
                    args: Vec::new(),
                }),
                parameter.span,
            ));
            parameter_index += 1;
        }
        let target_path = Path {
            declaration: None,
            name: definition.name.clone(),
            args: arguments,
        };
        let target = self.add_type(Type::Path(target_path.clone()), span);
        let mut fields = Vec::new();
        for field in &definition.fields {
            let callee = self.add_expr(
                Expr::Path(Path {
                    declaration: None,
                    name: "std::default::__lkj_default".to_string(),
                    args: vec![field.ty],
                }),
                field.span,
            );
            let value = self.add_expr(
                Expr::Call {
                    callee,
                    args: Vec::new(),
                },
                field.span,
            );
            fields.push((field.name.clone(), value));
        }
        let value = if definition.unit {
            self.add_expr(Expr::Path(target_path), span)
        } else if definition.tuple {
            let callee = self.add_expr(Expr::Path(target_path), span);
            let mut args = Vec::new();
            for (_, value) in fields {
                args.push(value);
            }
            self.add_expr(Expr::Call { callee, args }, span)
        } else {
            self.add_expr(
                Expr::Struct {
                    path: target_path,
                    fields,
                    base: None,
                },
                span,
            )
        };
        let body = self.program.blocks.len();
        self.program.blocks.push(Block {
            statements: Vec::new(),
            tail: Some(value),
            span,
        });
        let method = self.program.functions.len();
        self.program.functions.push(Function {
            name: "default".to_string(),
            module: definition.module.clone(),
            generics: Vec::new(),
            params: Vec::new(),
            result: target,
            body: Some(body),
            attrs: Vec::new(),
            public: Visibility::Public,
            span,
        });
        self.program.impls.push(Impl {
            module: definition.module,
            generics,
            target,
            trait_ty: Some(bound),
            methods: vec![method],
            associated: Vec::new(),
            span,
        });
        Ok(())
    }
}
