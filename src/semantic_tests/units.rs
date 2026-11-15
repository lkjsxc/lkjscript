fn instructions(ops: Vec<crate::ir::Op>, line: usize) -> Vec<crate::ir::Instruction> {
    let mut result = Vec::new();
    for op in ops {
        result.push(crate::ir::Instruction {
            file: None,
            op,
            line,
            column: 1,
        });
    }
    result
}
// Internal representation, checker, generic, parser and encoder contracts.
mod syntax_tests {
    use crate::ast::*;
    use crate::syntax::parse;
    pub fn precedence_and_arena() {
        let p = parse("fn answer() -> i64 { 1 + 2 * 3 } fn main() { assert_eq!(answer(), 7); }")
            .unwrap();
        assert_eq!(p.functions.len(), 2);
        let body = p.functions[0].body.unwrap();
        let tail = p.blocks[body].tail.unwrap();
        match &p.exprs[tail].kind {
            Expr::Binary { op, right, .. } => {
                assert_eq!(op, "+");
                match &p.exprs[*right].kind {
                    Expr::Binary { op, .. } => assert_eq!(op, "*"),
                    _ => panic!("multiplication missing"),
                }
            }
            _ => panic!("addition missing"),
        }
    }
    pub fn numeric_suffixes_preserve_operators_and_decimal_exponents() {
        for literal in vec![
            "6usize".to_string(),
            "6isize".to_string(),
            "0x1e".to_string(),
            "0x1eu8".to_string(),
            "1_000u64".to_string(),
        ] {
            let source = format!("fn f(){{{literal}}}");
            let program = parse(&source).unwrap();
            assert!(matches!(program.exprs[0].kind, Expr::Int(_)), "{literal}");
        }
        for expression in vec![
            "6usize+2usize".to_string(),
            "6isize-2isize".to_string(),
            "0x1e+2".to_string(),
            "0x1E-2".to_string(),
        ] {
            let source = format!("fn f(){{{expression}}}");
            let program = parse(&source).unwrap();
            let tail = program.blocks[program.functions[0].body.unwrap()]
                .tail
                .unwrap();
            assert!(
                matches!(program.exprs[tail].kind, Expr::Binary { .. }),
                "{expression}"
            );
        }
        for literal in vec![
            "1e+2".to_string(),
            "1E-2".to_string(),
            "1.25e+2f64".to_string(),
            "1_0e-2".to_string(),
        ] {
            let source = format!("fn f(){{{literal}}}");
            let program = parse(&source).unwrap();
            assert!(matches!(program.exprs[0].kind, Expr::Float(_)), "{literal}");
        }
    }
    pub fn declarations_and_borrows() {
        let source = "use std::collections::BTreeMap; #[derive(Clone, Copy)] struct Pair<T> { a: T, b: T } enum Maybe<T> { No, Yes(T) } trait Get { type Item; fn get(&self) -> Self::Item; } impl<T: Copy> Get for Pair<T> { type Item = T; fn get(&self) -> T { self.a } } fn view<'a>(x: &'a [u8]) -> &'a [u8] { x }";
        let p = parse(source).unwrap();
        assert_eq!(p.structs.len(), 1);
        assert_eq!(p.enums.len(), 1);
        assert_eq!(p.traits.len(), 1);
        assert_eq!(p.impls.len(), 1);
        assert_eq!(p.functions.len(), 3);
    }
    pub fn control_and_patterns() {
        let p = parse("fn f(mut x: i64) -> i64 { let mut v: Vec<i64> = vec![1, 2]; for i in 0..3 { v.push(i); } while x > 0 { x -= 1; } match Some(x) { Some(v) if v == 0 => v, _ => 1, } }").unwrap();
        assert_eq!(p.functions.len(), 1);
        assert!(!p.patterns.is_empty());
    }
    pub fn nested_generics_and_parentheses() {
        let p = parse("fn f(x: Vec<Vec<i64>>) -> i64 { (((1) + 2) * 3) }").unwrap();
        assert_eq!(p.functions.len(), 1);
    }
    pub fn deep_grouping_is_iterative() {
        let mut source = String::from("fn f() -> i64 { ");
        let mut i = 0;
        while i < 20_000 {
            source.push('(');
            i += 1;
        }
        source.push('1');
        i = 0;
        while i < 20_000 {
            source.push(')');
            i += 1;
        }
        source.push_str(" }");
        let p = parse(&source).unwrap();
        assert_eq!(p.exprs.len(), 1);
    }
    pub fn diagnostics_name_rejected_constructs() {
        let bytes = parse("fn f() { let b = b\"\\xFF\\0\"; let c = b'\\x80'; }").unwrap();
        assert!(
            bytes
                .exprs
                .iter()
                .any(|e| matches!(&e.kind, Expr::Bytes(v) if *v == vec![255, 0]))
        );
        assert_eq!(
            parse("fn f() { let b = b\"é\"; }").unwrap_err().code,
            "E_LEX"
        );
        parse("fn f() { let strings = [\"&&\", \"||\", \"}\", \"!\", \"if\"]; }").unwrap();
        assert_eq!(
            parse("fn f() { println!(\"ambient\"); }").unwrap_err().code,
            "E_CAPABILITY"
        );
        assert_eq!(
            parse("fn f() { unsafe {} }").unwrap_err().code,
            "E_UNSUPPORTED"
        );
        assert_eq!(
            parse("fn f() { mystery!(); }").unwrap_err().code,
            "E_UNSUPPORTED_MACRO"
        );
        assert_eq!(parse("fn f() { /* missing }").unwrap_err().code, "E_LEX");
    }
    pub fn views_and_closure_syntax() {
        parse("fn f() { let mut x = 1; let r = &&mut x; }").unwrap();
        let p = parse("struct View<'a> { bytes: &'a [u8] } fn f(v: &mut Vec<i64>) { v.push(v.len() as i64); let cb = move |x: i64| -> i64 { x + 1 }; let y = cb(2); }").unwrap();
        assert_eq!(p.structs.len(), 1);
        assert_eq!(p.functions.len(), 1);
    }
}
mod check_tests {
    use crate::check::*;
    use crate::ir::*;

    fn local(ty: usize, mutable: bool) -> Local {
        Local {
            name: String::new(),
            ty,
            mutable,
        }
    }
    fn op(op: Op) -> Instruction {
        Instruction {
            file: None,
            op,
            line: 1,
            column: 1,
        }
    }
    fn program(
        types: Vec<Type>,
        locals: Vec<Local>,
        params: usize,
        result: usize,
        ops: Vec<Op>,
    ) -> Program {
        let return_source = if is_view(&types, result) && params > 0 {
            Some(ReturnSource::Parameter {
                index: 0,
                projections: Vec::new(),
            })
        } else {
            None
        };
        Program {
            destructors: Vec::new(),
            types,
            functions: vec![Function {
                file: 0,
                return_source,
                name: "probe".to_string(),
                params,
                result,
                locals,
                body: super::instructions(ops, 1),
                test: false,
            }],
        }
    }
    fn rejection(p: Program, code: &str) {
        let result = check(p);
        assert!(result.is_err(), "malformed IR was accepted");
        assert_eq!(result.err().unwrap().code, code);
    }
    fn with_ref(mutable: bool) -> Vec<Type> {
        let mut types = primitives();
        types.push(Type {
            name: "reference".to_string(),
            kind: TypeKind::Ref {
                inner: I64,
                mutable,
            },
        });
        types
    }

    pub fn accepts_scalar() {
        assert!(
            check(program(
                primitives(),
                vec![local(I64, false)],
                0,
                I64,
                vec![Op::Int { dst: 0, value: 7 }, Op::Return { src: 0 }]
            ))
            .is_ok()
        );
    }

    pub fn rejects_forged_scalar() {
        let mut types = primitives();
        types.push(Type {
            name: "World".to_string(),
            kind: TypeKind::World,
        });
        rejection(
            program(
                types,
                vec![local(7, false)],
                0,
                7,
                vec![Op::Int { dst: 0, value: 7 }, Op::Return { src: 0 }],
            ),
            "E_TYPE",
        );
    }

    pub fn rejects_bad_operand_in_dead_code() {
        rejection(
            program(
                primitives(),
                vec![local(I64, false)],
                0,
                I64,
                vec![
                    Op::Int { dst: 0, value: 7 },
                    Op::Return { src: 0 },
                    Op::Read {
                        dst: 0,
                        place: Place::local(99),
                        moving: false,
                    },
                ],
            ),
            "E_TYPE",
        );
    }

    pub fn rejects_bad_later_signature_without_panicking() {
        let mut p = program(
            primitives(),
            vec![local(I64, false)],
            0,
            I64,
            vec![
                Op::Call {
                    dst: 0,
                    function: 1,
                    args: vec![0],
                },
                Op::Return { src: 0 },
            ],
        );
        p.functions.push(Function {
            file: 0,
            return_source: None,
            name: "bad".to_string(),
            params: 1,
            result: I64,
            locals: Vec::new(),
            body: Vec::new(),
            test: false,
        });
        rejection(p, "E_IR");
    }

    pub fn consuming_a_borrowed_owner_is_rejected() {
        let types = with_ref(false);
        let mut p = program(
            types,
            vec![local(I64, true), local(7, false), local(I64, false)],
            0,
            I64,
            vec![
                Op::Int { dst: 0, value: 7 },
                Op::Borrow {
                    dst: 1,
                    place: Place::local(0),
                    mutable: false,
                },
                Op::Call {
                    dst: 2,
                    function: 1,
                    args: vec![0],
                },
                Op::Return { src: 2 },
            ],
        );
        p.functions.push(Function {
            file: 0,
            return_source: None,
            name: "take".to_string(),
            params: 1,
            result: I64,
            locals: vec![local(I64, false)],
            body: vec![op(Op::Return { src: 0 })],
            test: false,
        });
        rejection(p, "E_BORROW");
    }

    pub fn immutable_reference_cannot_become_mutable_through_a_mut_binding() {
        let mut types = with_ref(false);
        types.push(Type {
            name: "mutable_reference".to_string(),
            kind: TypeKind::Ref {
                inner: I64,
                mutable: true,
            },
        });
        rejection(
            program(
                types,
                vec![local(7, true), local(8, false)],
                1,
                8,
                vec![
                    Op::Borrow {
                        dst: 1,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Deref],
                        },
                        mutable: true,
                    },
                    Op::Drop {
                        place: Place::local(0),
                    },
                    Op::Return { src: 1 },
                ],
            ),
            "E_BORROW",
        );
    }

    pub fn returned_parameter_view_preserves_external_origin() {
        assert!(
            check(program(
                with_ref(false),
                vec![local(7, false), local(7, false)],
                1,
                7,
                vec![
                    Op::Read {
                        dst: 1,
                        place: Place::local(0),
                        moving: false
                    },
                    Op::Drop {
                        place: Place::local(0)
                    },
                    Op::Return { src: 1 },
                ]
            ))
            .is_ok()
        );
    }

    pub fn returning_address_of_parameter_slot_is_rejected() {
        let mut types = with_ref(false);
        types.push(Type {
            name: "outer_reference".to_string(),
            kind: TypeKind::Ref {
                inner: 7,
                mutable: false,
            },
        });
        rejection(
            program(
                types,
                vec![local(7, false), local(8, false)],
                1,
                8,
                vec![
                    Op::Borrow {
                        dst: 1,
                        place: Place::local(0),
                        mutable: false,
                    },
                    Op::Return { src: 1 },
                ],
            ),
            "E_BORROW",
        );
    }

    pub fn recursive_containers_preserve_the_reference_storage_rule() {
        for reference in vec![false, true] {
            let mut types = primitives();
            types.push(Type {
                name: "Node".to_string(),
                kind: TypeKind::Struct {
                    fields: vec![
                        Field {
                            name: "next".to_string(),
                            ty: 8,
                        },
                        Field {
                            name: "value".to_string(),
                            ty: if reference { 9 } else { I64 },
                        },
                    ],
                    copy: false,
                },
            });
            types.push(Type {
                name: "BoxNode".to_string(),
                kind: TypeKind::Box { inner: 7 },
            });
            types.push(Type {
                name: "Reference".to_string(),
                kind: TypeKind::Ref {
                    inner: I64,
                    mutable: false,
                },
            });
            let value = program(
                types,
                vec![local(I64, false)],
                0,
                I64,
                vec![Op::Int { dst: 0, value: 42 }, Op::Return { src: 0 }],
            );
            if reference {
                rejection(value, "E_BORROW");
            } else {
                assert!(check(value).is_ok());
            }
        }
    }

    pub fn heap_storage_rejects_views() {
        let mut types = with_ref(false);
        types.push(Type {
            name: "BoxView".to_string(),
            kind: TypeKind::Box { inner: 7 },
        });
        rejection(
            program(
                types,
                vec![local(I64, false)],
                0,
                I64,
                vec![Op::Int { dst: 0, value: 7 }, Op::Return { src: 0 }],
            ),
            "E_BORROW",
        );
    }

    pub fn inactive_variant_cannot_forge_a_capability() {
        let mut types = primitives();
        types.push(Type {
            name: "World".to_string(),
            kind: TypeKind::World,
        });
        types.push(Type {
            name: "Either".to_string(),
            kind: TypeKind::Enum {
                copy: false,
                variants: vec![
                    Variant {
                        name: "Data".to_string(),
                        fields: vec![Field {
                            name: "0".to_string(),
                            ty: I64,
                        }],
                    },
                    Variant {
                        name: "Capability".to_string(),
                        fields: vec![Field {
                            name: "0".to_string(),
                            ty: 7,
                        }],
                    },
                ],
            },
        });
        rejection(
            program(
                types,
                vec![local(I64, false), local(8, false), local(7, false)],
                0,
                7,
                vec![
                    Op::Int { dst: 0, value: 7 },
                    Op::Variant {
                        dst: 1,
                        variant: 0,
                        fields: vec![0],
                    },
                    Op::Read {
                        dst: 2,
                        place: Place {
                            local: 1,
                            projections: vec![Projection::VariantField(1, 0)],
                        },
                        moving: true,
                    },
                    Op::Return { src: 2 },
                ],
            ),
            "E_TYPE",
        );
    }

    pub fn unknown_variant_field_needs_tag_branch() {
        let mut types = primitives();
        types.push(Type {
            name: "Either".to_string(),
            kind: TypeKind::Enum {
                copy: false,
                variants: vec![
                    Variant {
                        name: "Some".to_string(),
                        fields: vec![Field {
                            name: "0".to_string(),
                            ty: I64,
                        }],
                    },
                    Variant {
                        name: "None".to_string(),
                        fields: Vec::new(),
                    },
                ],
            },
        });
        rejection(
            program(
                types,
                vec![local(7, false), local(I64, false)],
                1,
                I64,
                vec![
                    Op::Read {
                        dst: 1,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::VariantField(0, 0)],
                        },
                        moving: false,
                    },
                    Op::Drop {
                        place: Place::local(0),
                    },
                    Op::Return { src: 1 },
                ],
            ),
            "E_TYPE",
        );
    }

    pub fn missing_explicit_cleanup_is_rejected() {
        rejection(
            program(
                primitives(),
                vec![local(I64, false), local(I64, false)],
                0,
                I64,
                vec![
                    Op::Int { dst: 0, value: 7 },
                    Op::Int { dst: 1, value: 8 },
                    Op::Return { src: 1 },
                ],
            ),
            "E_IR",
        );
    }

    pub fn by_value_recursive_types_are_rejected() {
        let mut types = primitives();
        types.push(Type {
            name: "Infinite".to_string(),
            kind: TypeKind::Struct {
                fields: vec![Field {
                    name: "self".to_string(),
                    ty: 7,
                }],
                copy: false,
            },
        });
        rejection(
            program(
                types,
                vec![local(I64, false)],
                0,
                I64,
                vec![Op::Int { dst: 0, value: 7 }, Op::Return { src: 0 }],
            ),
            "E_TYPE",
        );
    }

    pub fn bool_cannot_cast_directly_to_float() {
        rejection(
            program(
                primitives(),
                vec![local(BOOL, false), local(F64, false)],
                0,
                F64,
                vec![
                    Op::Int { dst: 0, value: 1 },
                    Op::Intrinsic {
                        dst: 1,
                        name: IntrinsicOp::Cast,
                        args: vec![0],
                    },
                    Op::Return { src: 1 },
                ],
            ),
            "E_TYPE",
        );
    }
    pub fn partially_moved_aggregate_cannot_be_consumed_whole() {
        let mut types = primitives();
        types.push(Type {
            name: "Box".to_string(),
            kind: TypeKind::Box { inner: I64 },
        });
        types.push(Type {
            name: "Pair".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![
                    Field {
                        name: "left".to_string(),
                        ty: 7,
                    },
                    Field {
                        name: "right".to_string(),
                        ty: I64,
                    },
                ],
            },
        });
        rejection(
            program(
                types,
                vec![local(8, false), local(7, false)],
                1,
                8,
                vec![
                    Op::Read {
                        dst: 1,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Field(0)],
                        },
                        moving: true,
                    },
                    Op::Return { src: 0 },
                ],
            ),
            "E_MOVE",
        );
    }

    pub fn field_assignment_cannot_resurrect_a_moved_owner() {
        let mut types = primitives();
        types.push(Type {
            name: "Pair".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![Field {
                    name: "value".to_string(),
                    ty: I64,
                }],
            },
        });
        rejection(
            program(
                types,
                vec![local(7, true), local(7, false), local(I64, false)],
                1,
                7,
                vec![
                    Op::Read {
                        dst: 1,
                        place: Place::local(0),
                        moving: true,
                    },
                    Op::Int { dst: 2, value: 9 },
                    Op::Assign {
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Field(0)],
                        },
                        src: 2,
                    },
                    Op::Return { src: 1 },
                ],
            ),
            "E_MOVE",
        );
    }

    pub fn indexing_requires_an_initialized_index() {
        let mut types = primitives();
        types.push(Type {
            name: "Vec".to_string(),
            kind: TypeKind::Vec { inner: I64 },
        });
        rejection(
            program(
                types,
                vec![local(7, false), local(U64, false), local(I64, false)],
                1,
                I64,
                vec![
                    Op::Read {
                        dst: 2,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Index(1)],
                        },
                        moving: false,
                    },
                    Op::Drop {
                        place: Place::local(0),
                    },
                    Op::Return { src: 2 },
                ],
            ),
            "E_MOVE",
        );
    }

    pub fn checked_tag_branch_allows_a_variant_field() {
        let mut types = primitives();
        types.push(Type {
            name: "Either".to_string(),
            kind: TypeKind::Enum {
                copy: false,
                variants: vec![
                    Variant {
                        name: "Some".to_string(),
                        fields: vec![Field {
                            name: "0".to_string(),
                            ty: I64,
                        }],
                    },
                    Variant {
                        name: "None".to_string(),
                        fields: Vec::new(),
                    },
                ],
            },
        });
        assert!(
            check(program(
                types,
                vec![
                    local(7, false),
                    local(BOOL, false),
                    local(I64, false),
                    local(I64, false)
                ],
                1,
                I64,
                vec![
                    Op::TagTest {
                        dst: 1,
                        place: Place::local(0),
                        variant: 0
                    },
                    Op::If { condition: 1 },
                    Op::Read {
                        dst: 2,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::VariantField(0, 0)]
                        },
                        moving: false
                    },
                    Op::Drop {
                        place: Place::local(0)
                    },
                    Op::Return { src: 2 },
                    Op::Else,
                    Op::Int { dst: 3, value: 0 },
                    Op::Drop {
                        place: Place::local(0)
                    },
                    Op::Return { src: 3 },
                    Op::End,
                ]
            ))
            .is_ok()
        );
    }

    pub fn shared_reborrow_of_mutable_parameter_is_usable() {
        let mut types = with_ref(true);
        types.push(Type {
            name: "shared".to_string(),
            kind: TypeKind::Ref {
                inner: I64,
                mutable: false,
            },
        });
        assert!(
            check(program(
                types,
                vec![local(7, false), local(8, false), local(I64, false)],
                1,
                I64,
                vec![
                    Op::Borrow {
                        dst: 1,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Deref]
                        },
                        mutable: false
                    },
                    Op::Read {
                        dst: 2,
                        place: Place {
                            local: 1,
                            projections: vec![Projection::Deref]
                        },
                        moving: false
                    },
                    Op::Drop {
                        place: Place::local(1)
                    },
                    Op::Drop {
                        place: Place::local(0)
                    },
                    Op::Return { src: 2 },
                ]
            ))
            .is_ok()
        );
    }

    pub fn mutable_parent_cannot_be_called_while_reborrow_is_live() {
        let mut types = with_ref(true);
        types.push(Type {
            name: "shared".to_string(),
            kind: TypeKind::Ref {
                inner: I64,
                mutable: false,
            },
        });
        let mut p = program(
            types,
            vec![local(7, false), local(8, false), local(UNIT, false)],
            1,
            UNIT,
            vec![
                Op::Borrow {
                    dst: 1,
                    place: Place {
                        local: 0,
                        projections: vec![Projection::Deref],
                    },
                    mutable: false,
                },
                Op::Call {
                    dst: 2,
                    function: 1,
                    args: vec![0],
                },
                Op::Drop {
                    place: Place::local(1),
                },
                Op::Return { src: 2 },
            ],
        );
        p.functions.push(Function {
            file: 0,
            return_source: None,
            name: "mutate".to_string(),
            params: 1,
            result: UNIT,
            test: false,
            locals: vec![local(7, false), local(UNIT, false)],
            body: vec![
                op(Op::Drop {
                    place: Place::local(0),
                }),
                op(Op::Unit { dst: 1 }),
                op(Op::Return { src: 1 }),
            ],
        });
        rejection(p, "E_BORROW");
    }

    pub fn moved_local_cannot_be_dropped_twice() {
        rejection(
            program(
                primitives(),
                vec![local(I64, false)],
                0,
                I64,
                vec![
                    Op::Int { dst: 0, value: 7 },
                    Op::Drop {
                        place: Place::local(0),
                    },
                    Op::Drop {
                        place: Place::local(0),
                    },
                    Op::Int { dst: 0, value: 8 },
                    Op::Return { src: 0 },
                ],
            ),
            "E_MOVE",
        );
    }
    fn view_types(mutable: bool) -> Vec<Type> {
        let mut types = with_ref(mutable);
        types.push(Type {
            name: "View".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![
                    Field {
                        name: "reference".to_string(),
                        ty: 7,
                    },
                    Field {
                        name: "cache".to_string(),
                        ty: I64,
                    },
                ],
            },
        });
        types
    }

    pub fn view_field_keeps_its_owner_borrowed() {
        rejection(
            program(
                view_types(false),
                vec![
                    local(I64, false),
                    local(7, false),
                    local(I64, false),
                    local(8, false),
                    local(I64, false),
                ],
                0,
                I64,
                vec![
                    Op::Int { dst: 0, value: 42 },
                    Op::Borrow {
                        dst: 1,
                        place: Place::local(0),
                        mutable: false,
                    },
                    Op::Int { dst: 2, value: 0 },
                    Op::Struct {
                        dst: 3,
                        fields: vec![1, 2],
                    },
                    Op::Drop {
                        place: Place::local(0),
                    },
                    Op::Int { dst: 4, value: 0 },
                    Op::Return { src: 4 },
                ],
            ),
            "E_BORROW",
        );
    }

    pub fn by_value_view_can_return_its_stored_reference() {
        assert!(
            check(program(
                view_types(false),
                vec![local(8, false), local(7, false)],
                1,
                7,
                vec![
                    Op::Read {
                        dst: 1,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Field(0)]
                        },
                        moving: false
                    },
                    Op::Drop {
                        place: Place::local(0)
                    },
                    Op::Return { src: 1 },
                ]
            ))
            .is_ok()
        );
    }

    pub fn by_value_view_cannot_return_its_owned_cache() {
        rejection(
            program(
                view_types(false),
                vec![local(8, false), local(7, false)],
                1,
                7,
                vec![
                    Op::Borrow {
                        dst: 1,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Field(1)],
                        },
                        mutable: false,
                    },
                    Op::Return { src: 1 },
                ],
            ),
            "E_BORROW",
        );
    }

    pub fn by_value_view_cannot_return_address_of_its_reference_slot() {
        let mut types = view_types(false);
        types.push(Type {
            name: "&&i64".to_string(),
            kind: TypeKind::Ref {
                inner: 7,
                mutable: false,
            },
        });
        rejection(
            program(
                types,
                vec![local(8, false), local(9, false)],
                1,
                9,
                vec![
                    Op::Borrow {
                        dst: 1,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Field(0)],
                        },
                        mutable: false,
                    },
                    Op::Return { src: 1 },
                ],
            ),
            "E_BORROW",
        );
    }

    pub fn reborrow_freezes_the_view_that_holds_its_mutable_parent() {
        let mut types = view_types(true);
        types.push(Type {
            name: "&i64".to_string(),
            kind: TypeKind::Ref {
                inner: I64,
                mutable: false,
            },
        });
        types.push(Type {
            name: "&mut View".to_string(),
            kind: TypeKind::Ref {
                inner: 8,
                mutable: true,
            },
        });
        rejection(
            program(
                types,
                vec![
                    local(I64, true),
                    local(7, false),
                    local(I64, false),
                    local(8, true),
                    local(9, false),
                    local(10, false),
                    local(I64, false),
                ],
                0,
                I64,
                vec![
                    Op::Int { dst: 0, value: 42 },
                    Op::Borrow {
                        dst: 1,
                        place: Place::local(0),
                        mutable: true,
                    },
                    Op::Int { dst: 2, value: 0 },
                    Op::Struct {
                        dst: 3,
                        fields: vec![1, 2],
                    },
                    Op::Borrow {
                        dst: 4,
                        place: Place {
                            local: 3,
                            projections: vec![Projection::Field(0), Projection::Deref],
                        },
                        mutable: false,
                    },
                    Op::Borrow {
                        dst: 5,
                        place: Place::local(3),
                        mutable: true,
                    },
                    Op::Int { dst: 6, value: 0 },
                    Op::Return { src: 6 },
                ],
            ),
            "E_BORROW",
        );
    }

    pub fn exclusive_view_borrow_also_blocks_direct_use_of_reference_fields() {
        let mut types = view_types(true);
        types.push(Type {
            name: "&mut View".to_string(),
            kind: TypeKind::Ref {
                inner: 8,
                mutable: true,
            },
        });
        types.push(Type {
            name: "&i64".to_string(),
            kind: TypeKind::Ref {
                inner: I64,
                mutable: false,
            },
        });
        rejection(
            program(
                types,
                vec![
                    local(I64, true),
                    local(7, false),
                    local(I64, false),
                    local(8, true),
                    local(9, false),
                    local(10, false),
                    local(I64, false),
                ],
                0,
                I64,
                vec![
                    Op::Int { dst: 0, value: 42 },
                    Op::Borrow {
                        dst: 1,
                        place: Place::local(0),
                        mutable: true,
                    },
                    Op::Int { dst: 2, value: 0 },
                    Op::Struct {
                        dst: 3,
                        fields: vec![1, 2],
                    },
                    Op::Borrow {
                        dst: 4,
                        place: Place::local(3),
                        mutable: true,
                    },
                    Op::Borrow {
                        dst: 5,
                        place: Place {
                            local: 3,
                            projections: vec![Projection::Field(0), Projection::Deref],
                        },
                        mutable: false,
                    },
                    Op::Int { dst: 6, value: 0 },
                    Op::Return { src: 6 },
                ],
            ),
            "E_BORROW",
        );
    }

    pub fn declared_return_source_is_verified_against_the_body() {
        let mut p = program(
            with_ref(false),
            vec![local(7, false), local(7, false), local(7, false)],
            2,
            7,
            vec![
                Op::Read {
                    dst: 2,
                    place: Place::local(1),
                    moving: false,
                },
                Op::Drop {
                    place: Place::local(0),
                },
                Op::Drop {
                    place: Place::local(1),
                },
                Op::Return { src: 2 },
            ],
        );
        p.functions[0].return_source = Some(ReturnSource::Parameter {
            index: 0,
            projections: Vec::new(),
        });
        rejection(p, "E_BORROW");
    }

    pub fn broad_return_contract_cannot_hide_a_second_parameter_root() {
        let mut types = with_ref(false);
        types.push(Type {
            name: "PairView".to_string(),
            kind: TypeKind::Struct {
                copy: true,
                fields: vec![
                    Field {
                        name: "left".to_string(),
                        ty: 7,
                    },
                    Field {
                        name: "right".to_string(),
                        ty: 7,
                    },
                ],
            },
        });
        rejection(
            program(
                types,
                vec![local(7, false), local(7, false), local(8, false)],
                2,
                8,
                vec![
                    Op::Struct {
                        dst: 2,
                        fields: vec![0, 1],
                    },
                    Op::Return { src: 2 },
                ],
            ),
            "E_BORROW",
        );
    }

    pub fn custom_drop_forbids_partial_moves() {
        let mut types = primitives();
        types.push(Type {
            name: "Box<i64>".to_string(),
            kind: TypeKind::Box { inner: I64 },
        });
        types.push(Type {
            name: "Guard".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![Field {
                    name: "value".to_string(),
                    ty: 7,
                }],
            },
        });
        types.push(Type {
            name: "&mut Guard".to_string(),
            kind: TypeKind::Ref {
                inner: 8,
                mutable: true,
            },
        });
        let mut p = program(
            types,
            vec![local(8, false), local(7, false)],
            1,
            7,
            vec![
                Op::Read {
                    dst: 1,
                    place: Place {
                        local: 0,
                        projections: vec![Projection::Field(0)],
                    },
                    moving: true,
                },
                Op::Return { src: 1 },
            ],
        );
        p.destructors.push((8, 1));
        p.functions.push(Function {
            file: 0,
            name: "drop".to_string(),
            params: 1,
            result: UNIT,
            return_source: None,
            test: false,
            locals: vec![local(9, false), local(UNIT, false)],
            body: vec![
                op(Op::Drop {
                    place: Place::local(0),
                }),
                op(Op::Unit { dst: 1 }),
                op(Op::Return { src: 1 }),
            ],
        });
        rejection(p, "E_MOVE");
    }

    fn enum_payload_operations(replace_field: bool, mutation: Option<bool>) -> Program {
        let mut types = primitives();
        types.push(Type {
            name: "PayloadBox".to_string(),
            kind: TypeKind::Box { inner: I64 },
        });
        types.push(Type {
            name: "Packet".to_string(),
            kind: TypeKind::Enum {
                copy: false,
                variants: vec![
                    Variant {
                        name: "Data".to_string(),
                        fields: vec![
                            Field {
                                name: "payload".to_string(),
                                ty: 7,
                            },
                            Field {
                                name: "tag".to_string(),
                                ty: I64,
                            },
                        ],
                    },
                    Variant {
                        name: "End".to_string(),
                        fields: Vec::new(),
                    },
                ],
            },
        });
        types.push(Type {
            name: "PacketBorrow".to_string(),
            kind: TypeKind::Ref {
                inner: 8,
                mutable: true,
            },
        });
        let mut body = vec![
            Op::TagTest {
                dst: 1,
                place: Place::local(0),
                variant: 0,
            },
            Op::If { condition: 1 },
        ];
        if replace_field {
            for extra in vec![
                Op::Int { dst: 7, value: 42 },
                Op::Assign {
                    place: Place {
                        local: 0,
                        projections: vec![Projection::VariantField(0, 1)],
                    },
                    src: 7,
                },
            ] {
                body.push(extra);
            }
        }
        if let Some(before_branch) = mutation {
            let at = if before_branch { 1 } else { 2 };
            let mut replaced = Vec::new();
            let mut index = 0usize;
            for old in body {
                if index == at {
                    for extra in vec![
                        Op::Borrow {
                            dst: 5,
                            place: Place::local(0),
                            mutable: true,
                        },
                        Op::Call {
                            dst: 6,
                            function: 1,
                            args: vec![5],
                        },
                        Op::Drop {
                            place: Place::local(6),
                        },
                    ] {
                        replaced.push(extra);
                    }
                }
                replaced.push(old);
                index += 1;
            }
            if index == at {
                replaced.push(Op::Borrow {
                    dst: 5,
                    place: Place::local(0),
                    mutable: true,
                });
                replaced.push(Op::Call {
                    dst: 6,
                    function: 1,
                    args: vec![5],
                });
                replaced.push(Op::Drop {
                    place: Place::local(6),
                });
            }
            body = replaced;
        }
        for extra in vec![
            Op::Read {
                dst: 2,
                place: Place {
                    local: 0,
                    projections: vec![Projection::VariantField(0, 0)],
                },
                moving: true,
            },
            Op::Read {
                dst: 3,
                place: Place {
                    local: 0,
                    projections: vec![Projection::VariantField(0, 1)],
                },
                moving: false,
            },
            Op::Drop {
                place: Place::local(2),
            },
            Op::Drop {
                place: Place::local(0),
            },
            Op::Return { src: 3 },
            Op::Else,
            Op::Drop {
                place: Place::local(0),
            },
            Op::Int { dst: 4, value: 0 },
            Op::Return { src: 4 },
            Op::End,
        ] {
            body.push(extra);
        }
        let mut p = program(
            types,
            vec![
                local(8, true),
                local(BOOL, false),
                local(7, false),
                local(I64, false),
                local(I64, false),
                local(9, false),
                local(UNIT, false),
                local(I64, false),
            ],
            1,
            I64,
            body,
        );
        p.functions.push(Function {
            file: 0,
            name: "mutate_packet".to_string(),
            params: 1,
            result: UNIT,
            return_source: None,
            locals: vec![local(9, false), local(UNIT, false)],
            body: vec![
                op(Op::Drop {
                    place: Place::local(0),
                }),
                op(Op::Unit { dst: 1 }),
                op(Op::Return { src: 1 }),
            ],
            test: false,
        });
        p
    }

    pub fn payload_moves_and_field_writes_preserve_the_enclosing_enum_tag() {
        for replace in vec![false, true] {
            check(enum_payload_operations(replace, None)).unwrap();
        }
        let mut dropped = enum_payload_operations(false, None);
        for index in 0usize..dropped.functions[0].body.len() {
            if matches!(dropped.functions[0].body[index].op, Op::Read { dst: 2, .. }) {
                dropped.functions[0].body[index].op = Op::Drop {
                    place: Place {
                        local: 0,
                        projections: vec![Projection::VariantField(0, 0)],
                    },
                };
                break;
            }
        }
        dropped.functions[0].body.retain(|instruction| {
            !matches!(&instruction.op, Op::Drop { place } if *place == Place::local(2))
        });
        check(dropped).unwrap();
    }

    pub fn whole_enum_mutation_invalidates_both_saved_predicates_and_proven_tags() {
        for before_branch in vec![false, true] {
            let failure = check(enum_payload_operations(false, Some(before_branch)))
                .err()
                .unwrap();
            assert_eq!(failure.code, "E_TYPE");
            assert!(
                failure.message.contains("matching checked tag"),
                "{} at {}:{}: {}",
                failure.code,
                failure.line,
                failure.column,
                failure.message
            );
        }
    }

    pub fn tag_proof_cannot_follow_a_reused_index_into_another_reference() {
        let mut types = primitives();
        types.push(Type {
            name: "Either".to_string(),
            kind: TypeKind::Enum {
                copy: false,
                variants: vec![
                    Variant {
                        name: "Integer".to_string(),
                        fields: vec![Field {
                            name: "0".to_string(),
                            ty: I64,
                        }],
                    },
                    Variant {
                        name: "Empty".to_string(),
                        fields: Vec::new(),
                    },
                ],
            },
        });
        types.push(Type {
            name: "Vec<Either>".to_string(),
            kind: TypeKind::Vec { inner: 7 },
        });
        types.push(Type {
            name: "&Either".to_string(),
            kind: TypeKind::Ref {
                inner: 7,
                mutable: false,
            },
        });
        rejection(
            program(
                types,
                vec![
                    local(8, false),
                    local(U64, true),
                    local(9, false),
                    local(BOOL, false),
                    local(U64, false),
                    local(9, false),
                    local(I64, false),
                ],
                1,
                I64,
                vec![
                    Op::Int { dst: 1, value: 0 },
                    Op::Borrow {
                        dst: 2,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Index(1)],
                        },
                        mutable: false,
                    },
                    Op::TagTest {
                        dst: 3,
                        place: Place::local(2),
                        variant: 0,
                    },
                    Op::Int { dst: 4, value: 1 },
                    Op::Assign {
                        place: Place::local(1),
                        src: 4,
                    },
                    Op::Borrow {
                        dst: 5,
                        place: Place {
                            local: 0,
                            projections: vec![Projection::Index(1)],
                        },
                        mutable: false,
                    },
                    Op::If { condition: 3 },
                    Op::Read {
                        dst: 6,
                        place: Place {
                            local: 5,
                            projections: vec![Projection::Deref, Projection::VariantField(0, 0)],
                        },
                        moving: false,
                    },
                    Op::Return { src: 6 },
                    Op::End,
                ],
            ),
            "E_TYPE",
        );
    }
}
mod generics_tests {
    use crate::ast::*;
    use crate::generics::*;
    use std::collections::BTreeMap;
    fn validate_text(text: &str) -> Result<(), Diagnostic> {
        validate(&crate::syntax::parse(text).unwrap())
    }
    pub fn unused_expanding_recursion() {
        let e =
            validate_text("fn f<T>(x:T){ f::<Vec<T>>(vec![x]); } fn probe()->i64{42}").unwrap_err();
        assert_eq!(e.code, "E_RECURSION");
    }
    pub fn mutual_expansion() {
        let e = validate_text("fn f<T>(x:T){g::<Vec<T>>(vec![x]);}fn g<U>(x:U){f::<U>(x);}")
            .unwrap_err();
        assert_eq!(e.code, "E_RECURSION");
    }
    pub fn inferred_expansion() {
        let e = validate_text("fn f<T>(x:T){f(vec![x]);}").unwrap_err();
        assert_eq!(e.code, "E_RECURSION");
    }
    pub fn nonexpanding_recursion() {
        validate_text("type Identity<T> = T; fn f<T>(x:T){f::<Identity<T>>(x);}").unwrap();
        validate_text("fn f<T>(x:T){g::<T>(x);}fn g<U>(x:U){f::<U>(x);}").unwrap();
    }
    pub fn expansion_outside_component() {
        validate_text("fn f<T>(x:T){g::<Vec<T>>(vec![x]);}fn g<U>(x:U){}").unwrap();
    }
    pub fn reset_breaks_type_cycle() {
        validate_text("fn f<T>(x:T){g::<Vec<T>>(vec![x]);}fn g<U>(x:U){f::<i64>(1i64);}").unwrap();
    }
    pub fn generic_local_flow() {
        assert_eq!(
            validate_text("fn f<T>(x:T){let y: Vec<T> = vec![x];f(y);}")
                .unwrap_err()
                .code,
            "E_RECURSION"
        );
    }
    pub fn blanket_overlap() {
        assert_eq!(
            validate_text("trait A {} struct S<T>{x:T} impl<T> A for S<T>{} impl A for S<i64>{}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
    pub fn disjoint_impls() {
        validate_text("trait A {} impl A for i64{} impl A for bool{}").unwrap();
    }
    pub fn occurs_check_excludes_infinite_overlap() {
        validate_text("trait A<T> {} impl<T> A<T> for T {} impl<U> A<Vec<U>> for U {}").unwrap();
    }
    pub fn required_method_and_type() {
        assert_eq!(
            validate_text("trait A {fn f(&self);type Item;}impl A for i64{}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
}
mod generics_bound_tests {
    use crate::ast::*;
    use crate::generics::*;
    use crate::ir::{self, TypeKind};
    use std::collections::BTreeMap;
    pub fn missing_unused_bound_is_false() {
        let source = crate::syntax::parse(
            "trait Score {fn score(&self)->i64;} fn ignored<T:Score>(x:T)->i64{0}",
        )
        .unwrap();
        assert!(!satisfies(
            &source,
            &ir::primitives(),
            &BTreeMap::new(),
            ir::I64,
            "Score"
        ));
    }
    pub fn prerequisite_is_checked() {
        let source = crate::syntax::parse(
            "trait A {} trait B {} struct S<T>{x:T} impl A for i64 {} impl<T:A> B for S<T> {}",
        )
        .unwrap();
        let mut types = ir::primitives();
        let si = types.len();
        types.push(ir::Type {
            name: "S<i64>".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![ir::Field {
                    name: "x".to_string(),
                    ty: ir::I64,
                }],
            },
        });
        let sb = types.len();
        types.push(ir::Type {
            name: "S<bool>".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![ir::Field {
                    name: "x".to_string(),
                    ty: ir::BOOL,
                }],
            },
        });
        let mut apps = BTreeMap::new();
        apps.insert(si, ("S".to_string(), vec![ir::I64]));
        apps.insert(sb, ("S".to_string(), vec![ir::BOOL]));
        assert!(satisfies(&source, &types, &apps, si, "B"));
        assert!(!satisfies(&source, &types, &apps, sb, "B"));
    }
    pub fn user_prerequisite_cycle_is_false() {
        let source =
            crate::syntax::parse("trait A {} trait B {} impl<T:B> A for T {} impl<T:A> B for T {}")
                .unwrap();
        assert!(!satisfies(
            &source,
            &ir::primitives(),
            &BTreeMap::new(),
            ir::I64,
            "A"
        ));
    }
    pub fn recursive_structural_traits_have_no_depth_limit() {
        let source = crate::syntax::parse("struct Link {next:Box<Link>}").unwrap();
        let mut types = ir::primitives();
        let link = types.len();
        let bx = link + 1;
        types.push(ir::Type {
            name: "Link".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![ir::Field {
                    name: "next".to_string(),
                    ty: bx,
                }],
            },
        });
        types.push(ir::Type {
            name: "Box<Link>".to_string(),
            kind: TypeKind::Box { inner: link },
        });
        assert!(satisfies(&source, &types, &BTreeMap::new(), link, "Send"));
        assert!(satisfies(&source, &types, &BTreeMap::new(), link, "Sync"));
        assert!(satisfies(&source, &types, &BTreeMap::new(), link, "Data"));
        assert!(!satisfies(&source, &types, &BTreeMap::new(), link, "Copy"));
    }
    pub fn derive_lookup_unions_matching_names() {
        let source = crate::syntax::parse(
            "#[derive(Clone)] #[derive(PartialEq)] struct Same; #[derive(Ord)] struct Same; #[derive(Eq)] enum Same{Value} #[other(Send)] struct Same; trait Marker{} impl Marker for Same{}",
        ).unwrap();
        let types = vec![ir::Type {
            name: "Same".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: Vec::new(),
            },
        }];
        let index = ImplementationIndex::new(&source).unwrap();
        for bound in vec![
            "Clone".to_string(),
            "PartialEq".to_string(),
            "Ord".to_string(),
            "Eq".to_string(),
        ] {
            assert!(
                index.derived(&types, &BTreeMap::new(), 0, bound.as_str()),
                "{bound}"
            );
        }
        for bound in vec!["Copy".to_string(), "Send".to_string(), "clone".to_string()] {
            assert!(
                !index.derived(&types, &BTreeMap::new(), 0, bound.as_str()),
                "{bound}"
            );
        }
    }

    pub fn derive_lookup_preserves_qualified_names_and_application_bases() {
        let source = crate::syntax::parse(
            "#[derive(Clone)] struct Value<T>{value:T} mod other{#[derive(Ord)] enum Value<T>{Some(T)}}",
        ).unwrap();
        let index = ImplementationIndex::new(&source).unwrap();
        let types = vec![
            ir::Type {
                name: "Value<i64>".to_string(),
                kind: TypeKind::Struct {
                    copy: false,
                    fields: Vec::new(),
                },
            },
            ir::Type {
                name: "other::Value".to_string(),
                kind: TypeKind::Enum {
                    copy: false,
                    variants: Vec::new(),
                },
            },
        ];
        let mut applications: BTreeMap<usize, (String, Vec<usize>)> = BTreeMap::new();
        applications.insert(0, ("Value".to_string(), vec![ir::I64]));
        assert!(index.derived(&types, &applications, 0, "Clone"));
        assert!(!index.derived(&types, &applications, 0, "Ord"));
        assert!(!index.derived(&types, &BTreeMap::new(), 0, "Clone"));
        assert!(index.derived(&types, &applications, 1, "Ord"));
        assert!(!index.derived(&types, &applications, 1, "Clone"));
    }

    pub fn indexed_derive_still_requires_the_fields_trait() {
        let source = crate::syntax::parse(
            "#[derive(Clone,PartialEq,Eq,PartialOrd,Ord)] struct Holder<T>{value:T}",
        )
        .unwrap();
        let mut types = ir::primitives();
        let holder = types.len();
        types.push(ir::Type {
            name: "Holder<f64>".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![ir::Field {
                    name: "value".to_string(),
                    ty: ir::F64,
                }],
            },
        });
        let mut applications: BTreeMap<usize, (String, Vec<usize>)> = BTreeMap::new();
        applications.insert(holder, ("Holder".to_string(), vec![ir::F64]));
        assert!(satisfies(&source, &types, &applications, holder, "Clone"));
        assert!(satisfies(
            &source,
            &types,
            &applications,
            holder,
            "PartialOrd"
        ));
        assert!(!satisfies(&source, &types, &applications, holder, "Ord"));
        assert!(!satisfies(&source, &types, &applications, holder, "Eq"));
    }

    pub fn ord_and_clone_on_collections() {
        let source = Program::default();
        let mut types = ir::primitives();
        let vi = types.len();
        types.push(ir::Type {
            name: "Vec<i64>".to_string(),
            kind: TypeKind::Vec { inner: ir::I64 },
        });
        let vf = types.len();
        types.push(ir::Type {
            name: "Vec<f64>".to_string(),
            kind: TypeKind::Vec { inner: ir::F64 },
        });
        assert!(satisfies(&source, &types, &BTreeMap::new(), vi, "Ord"));
        assert!(!satisfies(&source, &types, &BTreeMap::new(), vf, "Ord"));
        assert!(satisfies(&source, &types, &BTreeMap::new(), vf, "Clone"));
    }
}
mod generics_signature_tests {
    use crate::ast::*;
    use crate::generics::*;
    use std::collections::BTreeMap;
    fn check(text: &str) -> Result<(), Diagnostic> {
        validate(&crate::syntax::parse(text).unwrap())
    }
    pub fn mismatched_unused_result() {
        assert_eq!(check("trait A {fn f(&self)->i64;}impl A for bool{fn f(&self)->bool{true}}fn probe()->i64{42}").unwrap_err().code,"E_TRAIT");
    }
    pub fn receiver_convention_matters() {
        assert_eq!(
            check("trait A {fn f(&self);}impl A for i64{fn f(&mut self){}}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
    pub fn associated_substitution_and_alias() {
        check("type Alias<T> = Vec<T>;trait A {type Out;fn f(&self)->Self::Out;}impl A for i64{type Out=Alias<i64>;fn f(&self)->Vec<i64>{vec![1]}}").unwrap();
    }
    pub fn renamed_method_generics() {
        check("trait A {fn f<T>(&self,x:T)->T;}impl A for i64{fn f<U>(&self,x:U)->U{x}}").unwrap();
    }
    pub fn stronger_method_bound_rejected() {
        assert_eq!(
            check("trait A {fn f<T>(&self,x:T);}impl A for i64{fn f<U:Copy>(&self,x:U){}}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
    pub fn omitted_inherited_bound_accepted() {
        check("trait A {fn f<T:Copy>(&self,x:T);}impl A for i64{fn f<U>(&self,x:U){}}").unwrap();
    }
    pub fn implied_method_bound() {
        check("trait A {fn f<T:Ord>(&self,x:T);}impl A for i64{fn f<U:Eq>(&self,x:U){}}").unwrap();
    }
    pub fn trait_argument_substitution() {
        check("trait A<T>{fn f(&self,x:T)->T;}impl<U> A<U> for Vec<U>{fn f(&self,x:U)->U{x}}")
            .unwrap();
    }
    pub fn missing_trait_argument_bound() {
        assert_eq!(
            check("trait A<T:Copy>{}impl<U> A<U> for Vec<U>{}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
    pub fn associated_bound_checked() {
        assert_eq!(
            check("trait A {type Out:Copy;}impl A for i64{type Out=Vec<i64>;}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
        check("trait A{type Out:Copy;}impl<T:Copy> A for Vec<T>{type Out=T;}").unwrap();
    }
    pub fn alias_cycle_rejected() {
        check("mod std {pub mod result {pub enum Result<T,E> {Ok(T),Err(E)}}} type Result<T> = std::result::Result<T, i64>; trait A {fn f(&self)->Result<bool>;} impl A for i64 {fn f(&self)->std::result::Result<bool,i64>{std::result::Result::Ok(true)}}").unwrap();
        assert_eq!(check("type A=B;type B=Vec<A>;").unwrap_err().code, "E_TYPE");
    }
}
mod generics_declaration_tests {
    use crate::ast::*;
    use crate::generics::*;
    use std::collections::BTreeMap;
    fn check(text: &str) -> Result<(), Diagnostic> {
        validate(&crate::syntax::parse(text).unwrap())
    }
    pub fn duplicate_functions() {
        assert_eq!(check("fn f(){} fn f(){}").unwrap_err().code, "E_NAME");
    }
    pub fn type_and_value_namespaces() {
        check("struct S{x:i64} fn S()->i64{42}").unwrap();
        assert_eq!(
            check("struct S(i64);fn S()->i64{42}").unwrap_err().code,
            "E_NAME"
        );
    }
    pub fn duplicate_fields_and_parameters() {
        assert_eq!(check("struct S{x:i64,x:bool}").unwrap_err().code, "E_NAME");
        assert_eq!(check("fn f(x:i64,x:i64){}").unwrap_err().code, "E_NAME");
    }
    pub fn direct_and_mutual_constant_cycles() {
        assert_eq!(check("const A:i64=A;").unwrap_err().code, "E_TYPE");
        assert_eq!(
            check("const A:i64=B;const B:i64=A;").unwrap_err().code,
            "E_TYPE"
        );
        check("const A:i64=1;const B:i64=A+1;").unwrap();
    }
    pub fn external_module_loading_is_explicit() {
        assert_eq!(check("mod missing;").unwrap_err().code, "E_UNSUPPORTED");
        check("mod a { fn f(){} } mod b {fn f(){}}").unwrap();
    }
    pub fn expanding_data_recursion() {
        assert_eq!(
            check("struct Node<T>{value:T,next:Box<Node<Vec<T>>>}")
                .unwrap_err()
                .code,
            "E_RECURSION"
        );
    }
    pub fn mutual_data_expansion() {
        assert_eq!(
            check("struct A<T>{x:Box<B<Vec<T>>>}struct B<U>{x:Box<A<U>>}")
                .unwrap_err()
                .code,
            "E_RECURSION"
        );
    }
    pub fn alias_data_expansion() {
        assert_eq!(
            check("type Nest<T>=Node<Vec<T>>;struct Node<T>{next:Box<Nest<T>>}")
                .unwrap_err()
                .code,
            "E_RECURSION"
        );
    }
    pub fn regular_data_recursion() {
        check("struct Node<T>{value:T,next:Box<Node<T>>}").unwrap();
        check("enum List<T>{Nil,Cons(T,Box<List<T>>)}").unwrap();
    }
}
mod runtime_memory_tests {
    use crate::runtime_memory::*;

    pub fn mappings_are_disjoint_and_release_invalidates_raw_addresses() {
        let mut memory = Memory::new();
        let left = memory.invoke(Primitive::Map, &[4096]).unwrap();
        let right = memory.invoke(Primitive::Map, &[4096]).unwrap();
        assert!(right > left + 4096);
        memory.write(left, &[1, 2, 3, 4, 5, 6]).unwrap();
        memory
            .invoke(Primitive::MemoryCopy, &[left + 2, left, 4])
            .unwrap();
        assert_eq!(
            memory.read(left, 6).unwrap().to_vec(),
            vec![1, 2, 1, 2, 3, 4]
        );
        memory
            .invoke(Primitive::MemoryCopy, &[right, left + 2, 4])
            .unwrap();
        assert_eq!(memory.read(right, 4).unwrap().to_vec(), vec![1, 2, 3, 4]);
        assert!(matches!(
            memory.read(left + 4095, 2),
            Err(MemoryFault::Bounds)
        ));
        memory.invoke(Primitive::Unmap, &[left, 4096]).unwrap();
        assert!(matches!(memory.read(left, 1), Err(MemoryFault::Bounds)));
        assert_eq!(memory.read(right, 4).unwrap().to_vec(), vec![1, 2, 3, 4]);
    }

    pub fn remapping_preserves_contents_and_refusal_keeps_the_old_mapping_live() {
        let mut memory = Memory::new();
        let original = memory.invoke(Primitive::Map, &[17]).unwrap();
        memory.write(original, &[1, 2, 3, 4]).unwrap();
        memory.write(original + 4095, &[173]).unwrap();
        memory.refuse_next_mapping();
        assert_eq!(
            memory.invoke(Primitive::Remap, &[original, 17, 8192]),
            Ok(0u64)
        );
        assert_eq!(memory.read(original, 4).unwrap().to_vec(), vec![1, 2, 3, 4]);
        let grown = memory
            .invoke(Primitive::Remap, &[original, 4096, 8192])
            .unwrap();
        assert_ne!(grown, 0);
        assert!(matches!(memory.read(original, 1), Err(MemoryFault::Bounds)));
        assert_eq!(memory.read(grown, 4).unwrap().to_vec(), vec![1, 2, 3, 4]);
        assert_eq!(memory.read(grown + 4095, 2).unwrap().to_vec(), vec![173, 0]);
        assert_eq!(memory.read(grown + 8191, 1).unwrap().to_vec(), vec![0]);
        assert_eq!(memory.mapping_count(), 2);
        assert_eq!(
            memory.invoke(Primitive::Remap, &[grown + 1, 4096, 8192]),
            Err(MemoryFault::Bounds)
        );
    }
}
mod x86_tests {
    use crate::x86::*;
    fn finish(a: Assembler) -> Vec<u8> {
        match a.finish() {
            Ok(bytes) => bytes,
            Err(problem) => panic!("{}", problem.message),
        }
    }
    pub fn immediates_preserve_flags_and_choose_the_smallest_mov_form() {
        let mut a = Assembler::new();
        a.immediate(A, 0);
        a.immediate(R8, 0xffff_ffff);
        a.immediate(C, (-1i64) as u64);
        a.immediate(D, 0x0123_4567_89ab_cdef);
        assert_eq!(
            finish(a),
            vec![
                0xb8, 0, 0, 0, 0, 0x41, 0xb8, 0xff, 0xff, 0xff, 0xff, 0x48, 0xc7, 0xc1, 0xff, 0xff,
                0xff, 0xff, 0x48, 0xba, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01
            ]
        );
    }
    pub fn relative_calls_branches_and_rip_addresses_are_patched() {
        let mut a = Assembler::new();
        let start = a.label();
        let end = a.label();
        a.bind(start);
        a.call(end);
        a.condition(4, start);
        a.address(R8, end);
        a.bind(end);
        a.ret();
        assert_eq!(
            finish(a),
            vec![
                0xe8, 13, 0, 0, 0, 0x0f, 0x84, 0xf5, 0xff, 0xff, 0xff, 0x4c, 0x8d, 0x05, 0, 0, 0,
                0, 0xc3
            ]
        );

        let mut a = Assembler::new();
        a.enable_profile();
        let callee = a.label();
        a.immediate(A, 0);
        a.mov(R8, A);
        a.lea(DI, BP, -8);
        a.rr(0x01, A, C, true);
        a.load(A, BP, -8);
        a.store(DI, 0, A);
        a.scope_kind(KIND_ARGUMENT);
        a.mov(DI, A);
        a.call(callee);
        a.mov(SI, A);
        a.scope_kind(KIND_PROLOGUE);
        a.push(BP);
        a.ret();
        a.region_kind(KIND_RUNTIME);
        a.mov(A, DI);
        a.bind(callee);
        a.ret();
        assert_eq!(a.take_kinds(), vec![
            (0, KIND_CONSTANT), (5, KIND_MOVE), (8, KIND_ADDRESS),
            (12, KIND_ARITHMETIC), (15, KIND_FRAME), (19, KIND_MEMORY),
            (22, KIND_ARGUMENT), (25, KIND_CALL), (30, KIND_ARGUMENT),
            (33, KIND_PROLOGUE), (34, KIND_RETURN), (35, KIND_RUNTIME),
        ]);
        finish(a);
    }
    pub fn memory_widths_handle_low_byte_registers_and_stack_bases() {
        let mut a = Assembler::new();
        a.store_width(SP, 0, SI, 1);
        a.load_width(R9, R13, 0, 1, true);
        a.store_width(R12, -8, R10, 2);
        a.load(A, BP, -256);
        assert_eq!(
            finish(a),
            vec![
                0x40, 0x88, 0x34, 0x24, 0x4d, 0x0f, 0xbe, 0x4d, 0, 0x66, 0x45, 0x89, 0x54, 0x24,
                0xf8, 0x48, 0x8b, 0x85, 0, 0xff, 0xff, 0xff
            ]
        );
    }
    pub fn encoding_errors_retain_the_ir_location() {
        let mut a = Assembler::new();
        let far = a.label();
        a.location(7, 42, 9);
        a.jump(far);
        a.data_label(far, 2147483653usize);
        let problem = match a.finish() {
            Ok(_) => panic!("far branch accepted"),
            Err(problem) => problem,
        };
        assert_eq!((problem.file, problem.line, problem.column), (7, 42, 9));
        assert_eq!(problem.code, "E_EMIT");
    }
}
mod ir_text_tests {
    use crate::diagnostic::Diagnostic;
    use crate::ir::*;
    use crate::ir_text::{read, write};

    fn assert_same_program(actual: &Program, expected: &Program) {
        assert!(
            actual == expected,
            "IR structures differ:\nactual:\n{}\nexpected:\n{}",
            write(actual),
            write(expected)
        );
    }

    pub fn text_hexadecimal_payloads_preserve_bytes_and_reject_malformed_input() {
        for (encoded, expected) in vec![
            ("".to_string(), vec![]),
            ("00ffC3a9".to_string(), vec![0u8, 255, 195, 169]),
        ] {
            let text = format!("lkjscript-ir 1\nfn \"x\" 0 0\n@1 text 0 \"{encoded}\"\nendfn\n");
            assert!(
                read(text.as_str()).unwrap().functions[0].body[0].op
                    == Op::Text {
                        dst: 0,
                        bytes: expected
                    },
                "decoded text payload differs for {encoded}"
            );
        }
        for encoded in vec![
            "0".to_string(),
            "000".to_string(),
            "gg".to_string(),
            "é".to_string(),
            "0x".to_string(),
            "-1".to_string(),
            " 0".to_string(),
        ] {
            let text = format!("lkjscript-ir 1\nfn \"x\" 0 0\n@1 text 0 \"{encoded}\"\nendfn\n");
            assert_eq!(
                read(text.as_str())
                    .err()
                    .expect("malformed text payload must be rejected")
                    .code,
                "E_IR_TEXT",
                "{encoded}"
            );
        }
    }

    pub fn format_header_is_required_and_versioned() {
        for text in vec![
            "".to_string(),
            "type x unit\n".to_string(),
            "lkjscript-ir 0\n".to_string(),
            "lkjscript-ir 2\n".to_string(),
            "\nlkjscript-ir 1\n".to_string(),
            "lkjscript-ir 1 extra\n".to_string(),
        ] {
            let problem = read(text.as_str()).err().unwrap();
            assert_eq!(problem.code, "E_IR_TEXT");
            assert_eq!(problem.line, 1);
        }
        let program = read("lkjscript-ir 1\n").unwrap();
        assert_eq!(write(&program), "lkjscript-ir 1\n");
        let problem = read("lkjscript-ir 1\ntype\n").err().unwrap();
        assert_eq!(problem.line, 2);
    }

    pub fn source_round_trip_preserves_meaning_and_locations() {
        let source = "struct Pair { left:i64, right:i64 } pub fn probe()->i64 { let p=Pair {left:20,right:22}; p.left+p.right }";
        let syntax = crate::syntax::parse(source).unwrap();
        let checked = crate::check::check(crate::elaborate::elaborate(&syntax).unwrap()).unwrap();
        let text = write(checked.program());
        let decoded = read(text.as_str()).unwrap();
        assert_same_program(&decoded, checked.program());
        assert!(crate::check::check(decoded).is_ok());
    }

    pub fn text_can_supply_ir_the_elaborator_never_emits() {
        let mut text = write(&Program {
            destructors: Vec::new(),
            types: primitives(),
            functions: Vec::new(),
        });
        text.push_str("fn \"probe\" 0 3\nlocal \"value\" 3 let\n@1 int 0 42\n@1 return 0\n@2 drop 999\nendfn\n");
        let unchecked = read(text.as_str()).unwrap();
        assert_eq!(crate::check::check(unchecked).err().unwrap().code, "E_TYPE");
    }

    pub fn no_allocation_uses_declared_parameter_count() {
        let text = "lkjscript-ir 1\nfn \"bad\" 18446744073709551615 3\nendfn\n".to_string();
        let mut decoded = read(text.as_str()).unwrap();
        decoded.types = primitives();
        assert_eq!(crate::check::check(decoded).err().unwrap().code, "E_IR");
    }

    pub fn malformed_text_gets_a_diagnostic() {
        for text in vec![
            "type".to_string(),
            "type \"x\" struct".to_string(),
            "fn \"x\" 0 3".to_string(),
            "type \"x\" enum move".to_string(),
            "type \"x\" int 256 signed".to_string(),
            "fn \"x\" 0 3\n@1 read 0 move\nendfn".to_string(),
            "type \"x\\uD800\" unit".to_string(),
            "type \"x\" unit trailing".to_string(),
        ] {
            assert_eq!(
                read(&format!("lkjscript-ir 1\n{text}")).err().unwrap().code,
                "E_IR_TEXT",
                "{text}"
            );
        }
    }

    pub fn unknown_operation_spellings_are_rejected_before_ir_exists() {
        for instruction in vec![
            "unary 0 negate 1".to_string(),
            "binary 0 add 1 2".to_string(),
            "intrinsic 0 arbitrary 1".to_string(),
        ] {
            let text = format!("lkjscript-ir 1\nfn \"probe\" 0 3\n@7 {instruction}\nendfn\n");
            let problem = read(text.as_str()).err().unwrap();
            assert_eq!(problem.code, "E_IR_TEXT");
            assert_eq!(problem.line, 3);
        }
    }

    pub fn names_and_messages_use_json_string_escapes() {
        let name = "quote\" slash\\ newline\n and 雪";
        let p = Program {
            destructors: Vec::new(),
            types: vec![Type {
                name: name.to_string(),
                kind: TypeKind::World,
            }],
            functions: Vec::new(),
        };
        assert_same_program(&read(&write(&p)).unwrap(), &p);
        let decoded = read("lkjscript-ir 1\ntype \"\\uD83D\\uDE00\" world\n").unwrap();
        assert_eq!(decoded.types[0].name, "😀");
    }

    pub fn all_instruction_shapes_round_trip() {
        let ops = vec![
            Op::ViewRange {
                dst: 3,
                place: Place::local(0),
                start: 1,
                end: 2,
                inclusive: true,
                mutable: false,
            },
            Op::Int { dst: 0, value: -1 },
            Op::Float { dst: 0, bits: 42 },
            Op::Unit { dst: 0 },
            Op::Read {
                dst: 1,
                place: Place {
                    local: 0,
                    projections: vec![
                        Projection::Deref,
                        Projection::Field(1),
                        Projection::Index(2),
                        Projection::VariantField(0, 1),
                    ],
                },
                moving: true,
            },
            Op::Borrow {
                dst: 1,
                place: Place::local(0),
                mutable: true,
            },
            Op::Assign {
                place: Place::local(0),
                src: 1,
            },
            Op::Drop {
                place: Place::local(0),
            },
            Op::DropIf {
                place: Place::local(0),
            },
            Op::Unary {
                dst: 0,
                op: UnaryOp::Neg,
                src: 1,
            },
            Op::Binary {
                dst: 0,
                op: BinaryOp::Add,
                left: 1,
                right: 2,
            },
            Op::Call {
                dst: 0,
                function: 1,
                args: vec![2, 3],
            },
            Op::Struct {
                dst: 0,
                fields: vec![1, 2],
            },
            Op::Variant {
                dst: 0,
                variant: 1,
                fields: vec![2],
            },
            Op::Discriminant { dst: 0, src: 1 },
            Op::TagTest {
                dst: 0,
                place: Place::local(1),
                variant: 2,
            },
            Op::Intrinsic {
                dst: 0,
                name: IntrinsicOp::VecNew,
                args: Vec::new(),
            },
            Op::If { condition: 0 },
            Op::Else,
            Op::Loop,
            Op::Continue,
            Op::Break,
            Op::End,
            Op::Return { src: 0 },
            Op::Trap {
                message: "bad\nvalue".to_string(),
            },
        ];
        let p = Program {
            destructors: Vec::new(),
            types: primitives(),
            functions: vec![Function {
                file: 7,
                return_source: None,
                name: "probe".to_string(),
                params: 0,
                result: I64,
                locals: Vec::new(),
                body: super::instructions(ops, 7),
                test: true,
            }],
        };
        assert_same_program(&read(&write(&p)).unwrap(), &p);
    }
    pub fn arrays_slices_views_and_destructors_round_trip() {
        let source = "struct Guard<'a>{ count:&'a mut i64 } impl<'a> Drop for Guard<'a>{ fn drop(&mut self){*self.count=*self.count+1;} } fn first(values:&[u8])->&u8 { &values[0] } pub fn probe()->i64 { let mut count:i64=0; {let _guard=Guard{count:&mut count};} let bytes:[u8;2]=[20,22]; count+*first(&bytes) as i64 }";
        let syntax = crate::syntax::parse(source).unwrap();
        let checked = crate::check::check(crate::elaborate::elaborate(&syntax).unwrap()).unwrap();
        assert!(!checked.program().destructors.is_empty());
        assert!(
            checked
                .program()
                .functions
                .iter()
                .any(|function| function.return_source.is_some())
        );
        let text = write(checked.program());
        assert!(text.contains("slice-borrow"));
        assert!(text.contains("destructor"));
        let decoded = read(text.as_str()).unwrap();
        assert_same_program(&decoded, checked.program());
        assert!(crate::check::check(decoded).is_ok());
    }
}
pub fn run(world: &mut World, trace: bool) -> usize {
    if trace {
        let mut output = world.stderr();
        output.write(b"syntax_tests::precedence_and_arena\n");
    }
    syntax_tests::precedence_and_arena();
    if trace {
        let mut output = world.stderr();
        output.write(b"syntax_tests::numeric_suffixes_preserve_operators_and_decimal_exponents\n");
    }
    syntax_tests::numeric_suffixes_preserve_operators_and_decimal_exponents();
    if trace {
        let mut output = world.stderr();
        output.write(b"syntax_tests::declarations_and_borrows\n");
    }
    syntax_tests::declarations_and_borrows();
    if trace {
        let mut output = world.stderr();
        output.write(b"syntax_tests::control_and_patterns\n");
    }
    syntax_tests::control_and_patterns();
    if trace {
        let mut output = world.stderr();
        output.write(b"syntax_tests::nested_generics_and_parentheses\n");
    }
    syntax_tests::nested_generics_and_parentheses();
    if trace {
        let mut output = world.stderr();
        output.write(b"syntax_tests::deep_grouping_is_iterative\n");
    }
    syntax_tests::deep_grouping_is_iterative();
    if trace {
        let mut output = world.stderr();
        output.write(b"syntax_tests::diagnostics_name_rejected_constructs\n");
    }
    syntax_tests::diagnostics_name_rejected_constructs();
    if trace {
        let mut output = world.stderr();
        output.write(b"syntax_tests::views_and_closure_syntax\n");
    }
    syntax_tests::views_and_closure_syntax();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::accepts_scalar\n");
    }
    check_tests::accepts_scalar();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::rejects_forged_scalar\n");
    }
    check_tests::rejects_forged_scalar();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::rejects_bad_operand_in_dead_code\n");
    }
    check_tests::rejects_bad_operand_in_dead_code();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::rejects_bad_later_signature_without_panicking\n");
    }
    check_tests::rejects_bad_later_signature_without_panicking();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::consuming_a_borrowed_owner_is_rejected\n");
    }
    check_tests::consuming_a_borrowed_owner_is_rejected();
    if trace {
        let mut output = world.stderr();
        output.write(
            b"check_tests::immutable_reference_cannot_become_mutable_through_a_mut_binding\n",
        );
    }
    check_tests::immutable_reference_cannot_become_mutable_through_a_mut_binding();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::returned_parameter_view_preserves_external_origin\n");
    }
    check_tests::returned_parameter_view_preserves_external_origin();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::returning_address_of_parameter_slot_is_rejected\n");
    }
    check_tests::returning_address_of_parameter_slot_is_rejected();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::recursive_containers_preserve_the_reference_storage_rule\n");
    }
    check_tests::recursive_containers_preserve_the_reference_storage_rule();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::heap_storage_rejects_views\n");
    }
    check_tests::heap_storage_rejects_views();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::inactive_variant_cannot_forge_a_capability\n");
    }
    check_tests::inactive_variant_cannot_forge_a_capability();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::unknown_variant_field_needs_tag_branch\n");
    }
    check_tests::unknown_variant_field_needs_tag_branch();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::missing_explicit_cleanup_is_rejected\n");
    }
    check_tests::missing_explicit_cleanup_is_rejected();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::by_value_recursive_types_are_rejected\n");
    }
    check_tests::by_value_recursive_types_are_rejected();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::bool_cannot_cast_directly_to_float\n");
    }
    check_tests::bool_cannot_cast_directly_to_float();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::partially_moved_aggregate_cannot_be_consumed_whole\n");
    }
    check_tests::partially_moved_aggregate_cannot_be_consumed_whole();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::field_assignment_cannot_resurrect_a_moved_owner\n");
    }
    check_tests::field_assignment_cannot_resurrect_a_moved_owner();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::indexing_requires_an_initialized_index\n");
    }
    check_tests::indexing_requires_an_initialized_index();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::checked_tag_branch_allows_a_variant_field\n");
    }
    check_tests::checked_tag_branch_allows_a_variant_field();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::shared_reborrow_of_mutable_parameter_is_usable\n");
    }
    check_tests::shared_reborrow_of_mutable_parameter_is_usable();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::mutable_parent_cannot_be_called_while_reborrow_is_live\n");
    }
    check_tests::mutable_parent_cannot_be_called_while_reborrow_is_live();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::moved_local_cannot_be_dropped_twice\n");
    }
    check_tests::moved_local_cannot_be_dropped_twice();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::view_field_keeps_its_owner_borrowed\n");
    }
    check_tests::view_field_keeps_its_owner_borrowed();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::by_value_view_can_return_its_stored_reference\n");
    }
    check_tests::by_value_view_can_return_its_stored_reference();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::by_value_view_cannot_return_its_owned_cache\n");
    }
    check_tests::by_value_view_cannot_return_its_owned_cache();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::by_value_view_cannot_return_address_of_its_reference_slot\n");
    }
    check_tests::by_value_view_cannot_return_address_of_its_reference_slot();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::reborrow_freezes_the_view_that_holds_its_mutable_parent\n");
    }
    check_tests::reborrow_freezes_the_view_that_holds_its_mutable_parent();
    if trace {
        let mut output = world.stderr();
        output.write(
            b"check_tests::exclusive_view_borrow_also_blocks_direct_use_of_reference_fields\n",
        );
    }
    check_tests::exclusive_view_borrow_also_blocks_direct_use_of_reference_fields();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::declared_return_source_is_verified_against_the_body\n");
    }
    check_tests::declared_return_source_is_verified_against_the_body();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::broad_return_contract_cannot_hide_a_second_parameter_root\n");
    }
    check_tests::broad_return_contract_cannot_hide_a_second_parameter_root();
    if trace {
        let mut output = world.stderr();
        output.write(b"check_tests::custom_drop_forbids_partial_moves\n");
    }
    check_tests::custom_drop_forbids_partial_moves();
    if trace {
        let mut output = world.stderr();
        output.write(
            b"check_tests::payload_moves_and_field_writes_preserve_the_enclosing_enum_tag\n",
        );
    }
    check_tests::payload_moves_and_field_writes_preserve_the_enclosing_enum_tag();
    if trace {
        let mut output = world.stderr();
        output.write(
            b"check_tests::whole_enum_mutation_invalidates_both_saved_predicates_and_proven_tags\n",
        );
    }
    check_tests::whole_enum_mutation_invalidates_both_saved_predicates_and_proven_tags();
    if trace {
        let mut output = world.stderr();
        output
            .write(b"check_tests::tag_proof_cannot_follow_a_reused_index_into_another_reference\n");
    }
    check_tests::tag_proof_cannot_follow_a_reused_index_into_another_reference();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::unused_expanding_recursion\n");
    }
    generics_tests::unused_expanding_recursion();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::mutual_expansion\n");
    }
    generics_tests::mutual_expansion();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::inferred_expansion\n");
    }
    generics_tests::inferred_expansion();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::nonexpanding_recursion\n");
    }
    generics_tests::nonexpanding_recursion();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::expansion_outside_component\n");
    }
    generics_tests::expansion_outside_component();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::reset_breaks_type_cycle\n");
    }
    generics_tests::reset_breaks_type_cycle();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::generic_local_flow\n");
    }
    generics_tests::generic_local_flow();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::blanket_overlap\n");
    }
    generics_tests::blanket_overlap();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::disjoint_impls\n");
    }
    generics_tests::disjoint_impls();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::occurs_check_excludes_infinite_overlap\n");
    }
    generics_tests::occurs_check_excludes_infinite_overlap();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_tests::required_method_and_type\n");
    }
    generics_tests::required_method_and_type();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_bound_tests::missing_unused_bound_is_false\n");
    }
    generics_bound_tests::missing_unused_bound_is_false();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_bound_tests::prerequisite_is_checked\n");
    }
    generics_bound_tests::prerequisite_is_checked();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_bound_tests::user_prerequisite_cycle_is_false\n");
    }
    generics_bound_tests::user_prerequisite_cycle_is_false();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_bound_tests::recursive_structural_traits_have_no_depth_limit\n");
    }
    generics_bound_tests::recursive_structural_traits_have_no_depth_limit();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_bound_tests::derive_lookup_unions_matching_names\n");
    }
    generics_bound_tests::derive_lookup_unions_matching_names();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_bound_tests::derive_lookup_preserves_qualified_names_and_application_bases\n");
    }
    generics_bound_tests::derive_lookup_preserves_qualified_names_and_application_bases();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_bound_tests::indexed_derive_still_requires_the_fields_trait\n");
    }
    generics_bound_tests::indexed_derive_still_requires_the_fields_trait();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_bound_tests::ord_and_clone_on_collections\n");
    }
    generics_bound_tests::ord_and_clone_on_collections();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::mismatched_unused_result\n");
    }
    generics_signature_tests::mismatched_unused_result();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::receiver_convention_matters\n");
    }
    generics_signature_tests::receiver_convention_matters();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::associated_substitution_and_alias\n");
    }
    generics_signature_tests::associated_substitution_and_alias();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::renamed_method_generics\n");
    }
    generics_signature_tests::renamed_method_generics();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::stronger_method_bound_rejected\n");
    }
    generics_signature_tests::stronger_method_bound_rejected();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::omitted_inherited_bound_accepted\n");
    }
    generics_signature_tests::omitted_inherited_bound_accepted();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::implied_method_bound\n");
    }
    generics_signature_tests::implied_method_bound();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::trait_argument_substitution\n");
    }
    generics_signature_tests::trait_argument_substitution();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::missing_trait_argument_bound\n");
    }
    generics_signature_tests::missing_trait_argument_bound();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::associated_bound_checked\n");
    }
    generics_signature_tests::associated_bound_checked();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_signature_tests::alias_cycle_rejected\n");
    }
    generics_signature_tests::alias_cycle_rejected();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::duplicate_functions\n");
    }
    generics_declaration_tests::duplicate_functions();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::type_and_value_namespaces\n");
    }
    generics_declaration_tests::type_and_value_namespaces();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::duplicate_fields_and_parameters\n");
    }
    generics_declaration_tests::duplicate_fields_and_parameters();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::direct_and_mutual_constant_cycles\n");
    }
    generics_declaration_tests::direct_and_mutual_constant_cycles();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::external_module_loading_is_explicit\n");
    }
    generics_declaration_tests::external_module_loading_is_explicit();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::expanding_data_recursion\n");
    }
    generics_declaration_tests::expanding_data_recursion();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::mutual_data_expansion\n");
    }
    generics_declaration_tests::mutual_data_expansion();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::alias_data_expansion\n");
    }
    generics_declaration_tests::alias_data_expansion();
    if trace {
        let mut output = world.stderr();
        output.write(b"generics_declaration_tests::regular_data_recursion\n");
    }
    generics_declaration_tests::regular_data_recursion();
    if trace {
        let mut output = world.stderr();
        output.write(
            b"runtime_memory_tests::mappings_are_disjoint_and_release_invalidates_raw_addresses\n",
        );
    }
    runtime_memory_tests::mappings_are_disjoint_and_release_invalidates_raw_addresses();
    if trace {
        let mut output = world.stderr();
        output.write(b"runtime_memory_tests::remapping_preserves_contents_and_refusal_keeps_the_old_mapping_live\n");
    }
    runtime_memory_tests::remapping_preserves_contents_and_refusal_keeps_the_old_mapping_live();
    if trace {
        let mut output = world.stderr();
        output.write(b"x86_tests::immediates_preserve_flags_and_choose_the_smallest_mov_form\n");
    }
    x86_tests::immediates_preserve_flags_and_choose_the_smallest_mov_form();
    if trace {
        let mut output = world.stderr();
        output.write(b"x86_tests::relative_calls_branches_and_rip_addresses_are_patched\n");
    }
    x86_tests::relative_calls_branches_and_rip_addresses_are_patched();
    if trace {
        let mut output = world.stderr();
        output.write(b"x86_tests::memory_widths_handle_low_byte_registers_and_stack_bases\n");
    }
    x86_tests::memory_widths_handle_low_byte_registers_and_stack_bases();
    if trace {
        let mut output = world.stderr();
        output.write(b"x86_tests::encoding_errors_retain_the_ir_location\n");
    }
    x86_tests::encoding_errors_retain_the_ir_location();
    if trace {
        let mut output = world.stderr();
        output.write(
            b"ir_text_tests::text_hexadecimal_payloads_preserve_bytes_and_reject_malformed_input\n",
        );
    }
    ir_text_tests::text_hexadecimal_payloads_preserve_bytes_and_reject_malformed_input();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::format_header_is_required_and_versioned\n");
    }
    ir_text_tests::format_header_is_required_and_versioned();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::source_round_trip_preserves_meaning_and_locations\n");
    }
    ir_text_tests::source_round_trip_preserves_meaning_and_locations();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::text_can_supply_ir_the_elaborator_never_emits\n");
    }
    ir_text_tests::text_can_supply_ir_the_elaborator_never_emits();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::no_allocation_uses_declared_parameter_count\n");
    }
    ir_text_tests::no_allocation_uses_declared_parameter_count();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::malformed_text_gets_a_diagnostic\n");
    }
    ir_text_tests::malformed_text_gets_a_diagnostic();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::unknown_operation_spellings_are_rejected_before_ir_exists\n");
    }
    ir_text_tests::unknown_operation_spellings_are_rejected_before_ir_exists();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::names_and_messages_use_json_string_escapes\n");
    }
    ir_text_tests::names_and_messages_use_json_string_escapes();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::all_instruction_shapes_round_trip\n");
    }
    ir_text_tests::all_instruction_shapes_round_trip();
    if trace {
        let mut output = world.stderr();
        output.write(b"ir_text_tests::arrays_slices_views_and_destructors_round_trip\n");
    }
    ir_text_tests::arrays_slices_views_and_destructors_round_trip();
    97usize
}
