//! Collection-for is a source expansion over declared library protocols.
//! The outer match owns iterable temporaries. The user's pattern is a separate
//! let binding, so a refutable Item pattern is rejected instead of filtering.
use super::*;

struct Helpers {
    into: Path,
    next: Path,
    option: Path,
}
impl Helpers {
    fn new(resolver: &Resolver<'_>, span: Span) -> Result<Self> {
        let path = |name| -> Result<Path> {
            let function = resolver.library_function("iter", name, span)?;
            let declaration = resolver
                .declarations
                .iter()
                .position(|item| item.item == Item::Function(function))
                .ok_or_else(|| {
                    Diagnostic::new(
                        span,
                        "E_NAME",
                        "iteration helper has no declaration identity",
                    )
                })?;
            let function = &resolver.program.functions[function];
            Ok(Path {
                declaration: Some(declaration),
                name: qualified(&function.module, &function.name),
                args: Vec::new(),
            })
        };
        Ok(Self {
            into: path("__lkjscript_into_iter")?,
            next: path("__lkjscript_next")?,
            option: resolver.library_type("prelude", "Option", span)?,
        })
    }
}

struct Expansion<'a> {
    program: &'a mut Program,
    scope: ModuleId,
    span: Span,
}
impl Expansion<'_> {
    fn expression(&mut self, kind: Expr) -> ExprId {
        let id = self.program.exprs.len();
        self.program.exprs.push(ExprNode {
            kind,
            scope: self.scope,
            span: self.span,
        });
        id
    }
    fn pattern(&mut self, kind: Pattern) -> PatternId {
        let id = self.program.patterns.len();
        self.program.patterns.push(PatternNode {
            kind,
            scope: self.scope,
            span: self.span,
        });
        id
    }
    fn binding(&mut self, name: &str, mutable: bool) -> PatternId {
        self.pattern(Pattern::Bind {
            name: name.to_string(),
            mutable,
            by_ref: false,
            ref_mut: false,
        })
    }
    fn local(&mut self, name: &str) -> ExprId {
        self.expression(Expr::Path(Path {
            declaration: None,
            name: name.to_string(),
            args: Vec::new(),
        }))
    }
    fn block(&mut self, statements: Vec<Stmt>) -> BlockId {
        let id = self.program.blocks.len();
        self.program.blocks.push(Block {
            statements,
            tail: None,
            span: self.span,
        });
        id
    }
    fn call(&mut self, function: &Path, args: Vec<ExprId>) -> ExprId {
        let callee = self.expression(Expr::Path(function.clone()));
        self.expression(Expr::Call { callee, args })
    }
    fn item_body(&mut self, item: &str, pattern: PatternId, body: BlockId) -> BlockId {
        let value = self.local(item);
        let body_span = self.program.blocks[body].span;
        let body = self.expression(Expr::Block(body));
        let wild = self.pattern(Pattern::Wild);
        let unit = self.program.types.len();
        self.program.types.push(TypeNode {
            kind: Type::Tuple(Vec::new()),
            scope: self.scope,
            span: body_span,
        });
        let pattern_span = self.program.patterns[pattern].span;
        self.block(vec![
            Stmt::Let {
                pattern,
                ty: None,
                value: Some(value),
                span: pattern_span,
            },
            Stmt::Let {
                pattern: wild,
                ty: Some(unit),
                value: Some(body),
                span: body_span,
            },
        ])
    }
    fn collection(
        &mut self,
        id: ExprId,
        pattern: PatternId,
        iterable: ExprId,
        body: BlockId,
        helpers: &Helpers,
    ) -> Expr {
        let iterator_name = format!("$for:{id}:iterator");
        let item_name = format!("$for:{id}:item");
        let iterator_pattern = self.binding(&iterator_name, true);
        let iterator_value = self.call(&helpers.into, vec![iterable]);
        let iterator = self.local(&iterator_name);
        let receiver = self.expression(Expr::Borrow {
            mutable: true,
            value: iterator,
        });
        let next = self.call(&helpers.next, vec![receiver]);
        let item = self.binding(&item_name, true);
        let mut some = helpers.option.clone();
        some.name.push_str("::Some");
        let some = self.pattern(Pattern::Variant {
            path: some,
            fields: vec![item],
        });
        let mut none = helpers.option.clone();
        none.name.push_str("::None");
        let none = self.pattern(Pattern::Path(none));
        let body = self.item_body(&item_name, pattern, body);
        let body = self.expression(Expr::Block(body));
        let exhausted = self.expression(Expr::Break(None));
        let span = self.span;
        let selection = self.expression(Expr::Match {
            value: next,
            arms: vec![
                Arm {
                    pattern: some,
                    guard: None,
                    value: body,
                    span,
                },
                Arm {
                    pattern: none,
                    guard: None,
                    value: exhausted,
                    span,
                },
            ],
        });
        let body = self.block(vec![Stmt::Expr {
            value: selection,
            semicolon: true,
        }]);
        let condition = self.expression(Expr::Bool(true));
        let loop_body = self.expression(Expr::While { condition, body });
        let block = self.block(vec![Stmt::Expr {
            value: loop_body,
            semicolon: true,
        }]);
        let value = self.expression(Expr::Block(block));
        Expr::Match {
            value: iterator_value,
            arms: vec![Arm {
                pattern: iterator_pattern,
                guard: None,
                value,
                span: self.span,
            }],
        }
    }
}

pub(super) fn expand(program: &mut Program, resolver: &Resolver<'_>) -> Result<()> {
    let mut helpers = None;
    // Appended source nodes contain no collection-for. Original ids and spans
    // remain stable for diagnostics, imports, and definition evidence.
    for id in 0..program.exprs.len() {
        let node = program.exprs[id].clone();
        let Expr::For {
            pattern,
            iterator,
            body,
        } = node.kind
        else {
            continue;
        };
        let range = matches!(program.exprs[iterator].kind, Expr::Range { .. });
        let mut expansion = Expansion {
            program,
            scope: node.scope,
            span: node.span,
        };
        let kind = if range {
            // Keep the established numeric stepping contract while letting the
            // common pattern subsystem check every irrefutable range binding.
            let name = format!("$for:{id}:item");
            let binding = expansion.binding(&name, true);
            let body = expansion.item_body(&name, pattern, body);
            Expr::For {
                pattern: binding,
                iterator,
                body,
            }
        } else {
            if helpers.is_none() {
                helpers = Some(Helpers::new(resolver, node.span)?);
            }
            expansion.collection(id, pattern, iterator, body, helpers.as_ref().unwrap())
        };
        expansion.program.exprs[id].kind = kind;
    }
    Ok(())
}
