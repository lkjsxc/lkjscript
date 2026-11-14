//! Reference-bearing values must preserve every live loan across transfers.
#[path = "support/native.rs"]
mod native;
use lkjscript::{check, elaborate, ir, source, syntax};

fn source_program(source: &str) -> ir::Program {
    let parsed = syntax::parse(source).expect("regression source must parse");
    elaborate::elaborate(&parsed).expect("regression source must elaborate")
}

fn reject(program: ir::Program) {
    match check::check(program) {
        Ok(_) => panic!("IR checker accepted a reference to dead local storage"),
        Err(diagnostic) => assert_eq!(diagnostic.code, "E_BORROW", "{diagnostic:?}"),
    }
}

fn accept(program: ir::Program) {
    if let Err(diagnostic) = check::check(program) {
        panic!("valid reference transfer was rejected: {diagnostic:?}");
    }
}

fn run(text: &str, expected: i64) {
    native::assert_bundle(
        &source::SourceBundle {
            root: "provenance.rs".to_string(),
            library: None,
            files: vec![source::SourceInput {
                path: "provenance.rs".to_string(),
                text: text.to_string(),
            }],
        },
        &[("probe", expected)],
    );
}

fn local(ty: usize, mutable: bool) -> ir::Local {
    ir::Local {
        name: String::new(),
        ty,
        mutable,
    }
}

fn drop_local(index: usize) -> ir::Op {
    ir::Op::Drop {
        place: ir::Place::local(index),
    }
}

fn reference_types(copy: bool, mutable_view_borrow: bool) -> Vec<ir::Type> {
    let mut types = ir::primitives();
    types.push(ir::Type {
        name: "&i64".to_string(),
        kind: ir::TypeKind::Ref {
            inner: ir::I64,
            mutable: false,
        },
    });
    types.push(ir::Type {
        name: "View".to_string(),
        kind: ir::TypeKind::Struct {
            copy,
            fields: vec![ir::Field {
                name: "value".to_string(),
                ty: 7,
            }],
        },
    });
    types.push(ir::Type {
        name: "ViewBorrow".to_string(),
        kind: ir::TypeKind::Ref {
            inner: 8,
            mutable: mutable_view_borrow,
        },
    });
    types
}

fn program(types: Vec<ir::Type>, locals: Vec<ir::Local>, ops: Vec<ir::Op>) -> ir::Program {
    ir::Program {
        types,
        functions: vec![ir::Function {
            file: 0,
            name: "transfer".to_string(),
            params: 1,
            result: 8,
            return_source: Some(ir::ReturnSource::Parameter {
                index: 0,
                projections: Vec::new(),
            }),
            locals,
            body: ops
                .into_iter()
                .enumerate()
                .map(|(line, op)| ir::Instruction {
                    column: 1,
                    op,
                    line: line + 1,
                })
                .collect(),
            test: false,
        }],
        destructors: Vec::new(),
    }
}

fn reference(dst: usize, from_local: bool) -> ir::Op {
    if from_local {
        ir::Op::Borrow {
            dst,
            place: ir::Place::local(1),
            mutable: false,
        }
    } else {
        ir::Op::Read {
            dst,
            place: ir::Place::local(0),
            moving: false,
        }
    }
}

// These programs are built directly as IR; the source frontend cannot reject or
// repair them before the checker sees the replacement operation.
fn replacement(old_local: bool, new_local: bool, return_installed: bool) -> ir::Program {
    let mut ops = vec![
        ir::Op::Int { dst: 1, value: 7 },
        reference(2, old_local),
        ir::Op::Struct {
            dst: 3,
            fields: vec![2],
        },
        reference(4, new_local),
        ir::Op::Struct {
            dst: 5,
            fields: vec![4],
        },
        ir::Op::Borrow {
            dst: 6,
            place: ir::Place::local(3),
            mutable: true,
        },
        ir::Op::Intrinsic {
            dst: 7,
            name: ir::IntrinsicOp::Replace,
            args: vec![6, 5],
        },
    ];
    let result = if return_installed {
        ops.push(drop_local(7));
        ops.push(ir::Op::Read {
            dst: 8,
            place: ir::Place::local(3),
            moving: true,
        });
        8
    } else {
        ops.push(drop_local(3));
        7
    };
    ops.push(drop_local(1));
    ops.push(drop_local(0));
    ops.push(ir::Op::Return { src: result });
    program(
        reference_types(false, true),
        vec![
            local(7, false),
            local(ir::I64, false),
            local(7, false),
            local(8, true),
            local(7, false),
            local(8, false),
            local(9, false),
            local(8, false),
            local(8, false),
        ],
        ops,
    )
}

#[test]
fn direct_ir_replacement_return_keeps_the_old_value_source() {
    reject(replacement(true, false, false));
    accept(replacement(false, true, false));
}

#[test]
fn direct_ir_replacement_installs_the_new_value_source() {
    reject(replacement(false, true, true));
    accept(replacement(true, false, true));
}

fn cloned_view(from_local: bool) -> ir::Program {
    program(
        reference_types(true, false),
        vec![
            local(7, false),
            local(ir::I64, false),
            local(7, false),
            local(8, false),
            local(9, false),
            local(8, false),
        ],
        vec![
            ir::Op::Int { dst: 1, value: 7 },
            reference(2, from_local),
            ir::Op::Struct {
                dst: 3,
                fields: vec![2],
            },
            ir::Op::Borrow {
                dst: 4,
                place: ir::Place::local(3),
                mutable: false,
            },
            ir::Op::Intrinsic {
                dst: 5,
                name: ir::IntrinsicOp::Clone,
                args: vec![4],
            },
            drop_local(3),
            drop_local(1),
            drop_local(0),
            ir::Op::Return { src: 5 },
        ],
    )
}

#[test]
fn direct_ir_clone_cannot_erase_aggregate_provenance() {
    reject(cloned_view(true));
    accept(cloned_view(false));
}

#[test]
fn replacement_cannot_return_a_view_of_a_dead_local() {
    reject(source_program(
        r#"
        struct View<'a> { value: &'a i64 }
        fn leak<'a>(external: &'a i64) -> View<'a> {
            let local: i64 = 7;
            let mut view = View { value: &local };
            std::mem::replace(&mut view, View { value: external })
        }
    "#,
    ));
}

#[test]
fn clone_cannot_return_a_view_of_a_dead_local() {
    reject(source_program(
        r#"
        #[derive(Copy, Clone)]
        struct View<'a> { value: &'a i64 }
        fn leak<'a>(external: &'a i64) -> View<'a> {
            let local: i64 = 7;
            let view = View { value: &local };
            view.clone()
        }
    "#,
    ));
}

#[test]
fn replacement_cannot_install_a_local_view_in_a_parameter() {
    reject(source_program(
        r#"
        struct View<'a> { value: &'a i64 }
        fn overwrite<'a>(slot: &mut View<'a>) {
            let local: i64 = 7;
            let old = std::mem::replace(slot, View { value: &local });
        }
    "#,
    ));
}

#[test]
fn replacement_keeps_old_and_installed_views_usable() {
    run(
        r#"
        struct View<'a> { value: &'a i64 }
        fn probe() -> i64 {
            let left: i64 = 7;
            let right: i64 = 42;
            let mut view = View { value: &left };
            let old = std::mem::replace(&mut view, View { value: &right });
            *old.value + *view.value
        }
    "#,
        49,
    );
}

#[test]
fn replacement_of_a_field_preserves_disjoint_borrows() {
    run(
        r#"
        struct View<'a> { value: &'a i64 }
        struct Data { a: i64, b: i64 }
        struct Pair<'a> { left: View<'a>, right: View<'a> }
        fn probe() -> i64 {
            let data = Data { a: 7, b: 35 };
            let mut pair = Pair {
                left: View { value: &data.a },
                right: View { value: &data.b }
            };
            let right = &pair.right;
            let old = std::mem::replace(&mut pair.left, View { value: &data.b });
            *old.value + *right.value + *pair.left.value
        }
    "#,
        77,
    );
}

#[test]
fn replacement_transfers_exclusive_reference_fields() {
    run(
        r#"
        struct View<'a> { value: &'a mut i64 }
        struct Data { a: i64, b: i64 }
        fn probe() -> i64 {
            let mut data = Data { a: 7, b: 35 };
            let mut view = View { value: &mut data.a };
            let old = std::mem::replace(&mut view, View { value: &mut data.b });
            *old.value = 11;
            *view.value = 31;
            *old.value + *view.value
        }
    "#,
        42,
    );
}

#[test]
fn replacement_preserves_reference_free_enum_variants() {
    run(
        r#"
        enum Maybe<'a> { None, Some(&'a i64) }
        struct Wrapped<'a> { choice: Maybe<'a> }
        fn empty<'a>(external: &'a i64) -> Wrapped<'a> {
            let mut view = Wrapped { choice: Maybe::Some(external) };
            let old = std::mem::replace(&mut view, Wrapped { choice: Maybe::None });
            view
        }
        fn probe() -> i64 {
            let value: i64 = 42;
            let view = empty(&value);
            match view.choice { Maybe::None => 42, Maybe::Some(found) => *found }
        }
    "#,
        42,
    );
}

#[test]
fn clone_preserves_valid_shared_reference_leaves() {
    run(
        r#"
        #[derive(Copy, Clone)]
        struct View<'a> { value: &'a i64 }
        fn copied<'a>(external: &'a i64) -> View<'a> {
            let view = View { value: external };
            view.clone()
        }
        fn probe() -> i64 {
            let value: i64 = 42;
            *copied(&value).value
        }
    "#,
        42,
    );
}

#[test]
fn a_call_cannot_hide_a_reference_installed_from_a_shorter_lifetime() {
    reject(source_program(
        r#"
        struct View<'a> { value: &'a i64 }
        fn install<'a>(slot: &mut View<'a>, replacement: &'a i64) {
            let old = std::mem::replace(slot, View { value: replacement });
        }
        fn probe() -> i64 {
            let external: i64 = 42;
            let mut view = View { value: &external };
            {
                let local: i64 = 7;
                install(&mut view, &local);
            };
            *view.value
        }
    "#,
    ));
}

#[test]
fn a_parameter_root_does_not_justify_borrowing_its_owned_cache() {
    reject(source_program(
        r#"
        struct View<'a> { value: &'a i64 }
        struct Data<'a> { view: View<'a>, cache: i64 }
        fn install<'a>(data: &mut Data<'a>) {
            let old = std::mem::replace(&mut data.view, View { value: &data.cache });
        }
        fn probe() -> i64 {
            let external: i64 = 42;
            let mut data = Data { view: View { value: &external }, cache: 7 };
            install(&mut data);
            *data.view.value
        }
    "#,
    ));
}

fn field_assignment(old_local: bool, new_local: bool, borrowed: bool) -> ir::Program {
    let mut ops = vec![
        ir::Op::Int { dst: 1, value: 7 },
        reference(2, old_local),
        ir::Op::Struct {
            dst: 3,
            fields: vec![2],
        },
        reference(4, new_local),
    ];
    let place = if borrowed {
        ops.push(ir::Op::Borrow {
            dst: 6,
            place: ir::Place::local(3),
            mutable: true,
        });
        ir::Place {
            local: 6,
            projections: vec![ir::Projection::Deref, ir::Projection::Field(0)],
        }
    } else {
        ir::Place {
            local: 3,
            projections: vec![ir::Projection::Field(0)],
        }
    };
    ops.push(ir::Op::Assign { place, src: 4 });
    if borrowed {
        ops.push(drop_local(6));
    }
    ops.extend([
        ir::Op::Read {
            dst: 5,
            place: ir::Place::local(3),
            moving: true,
        },
        drop_local(1),
        drop_local(0),
        ir::Op::Return { src: 5 },
    ]);
    program(
        reference_types(false, true),
        vec![
            local(7, false),
            local(ir::I64, false),
            local(7, false),
            local(8, true),
            local(7, false),
            local(8, false),
            local(9, false),
        ],
        ops,
    )
}

#[test]
fn direct_ir_reference_field_assignment_updates_canonical_storage() {
    for borrowed in [false, true] {
        reject(field_assignment(false, true, borrowed));
        accept(field_assignment(true, false, borrowed));
    }
}

#[test]
fn direct_reference_replacement_preserves_exclusive_owners() {
    run(
        r#"
        struct Data { a: i64, b: i64 }
        fn probe() -> i64 {
            let mut data = Data { a: 1, b: 2 };
            let mut pointer = &mut data.a;
            let old = std::mem::replace(&mut pointer, &mut data.b);
            *old = 19;
            *pointer = 23;
            *old + *pointer
        }
    "#,
        42,
    );
}

#[test]
fn direct_reference_replacement_cannot_erase_an_escape() {
    reject(source_program(
        r#"
        fn leak<'a>(external: &'a i64) -> &'a i64 {
            let local: i64 = 7;
            let mut pointer = &local;
            std::mem::replace(&mut pointer, external)
        }
    "#,
    ));
}

#[test]
fn reference_field_assignment_cannot_bypass_external_write_contracts() {
    reject(source_program(
        r#"
        struct View<'a> { value: &'a i64 }
        fn install<'a>(slot: &mut View<'a>, replacement: &'a i64) {
            slot.value = replacement;
        }
        fn probe() -> i64 {
            let external: i64 = 42;
            let mut view = View { value: &external };
            {
                let local: i64 = 7;
                install(&mut view, &local);
            };
            *view.value
        }
    "#,
    ));
}

#[test]
fn absent_enum_leaves_survive_calls_clone_and_assignment() {
    run(
        r#"
        #[derive(Copy, Clone)]
        enum Maybe<'a> { None, Some(&'a i64) }
        fn inspect(value: &mut Maybe<'_>) {}
        fn empty<'a>(external: &'a i64) -> Maybe<'a> {
            let mut value: Maybe<'a> = Maybe::None;
            inspect(&mut value);
            let copied = value.clone();
            value = copied;
            value
        }
        fn probe() -> i64 {
            let external: i64 = 42;
            match empty(&external) {
                Maybe::None => 42,
                Maybe::Some(found) => *found
            }
        }
    "#,
        42,
    );
}

#[test]
fn reinitializing_an_absent_enum_leaf_restores_its_loan() {
    reject(source_program(
        r#"
        enum Maybe<'a> { None, Some(&'a i64) }
        fn inspect(value: &mut Maybe<'_>) {}
        fn leak<'a>(external: &'a i64) -> Maybe<'a> {
            let mut value: Maybe<'_> = Maybe::None;
            inspect(&mut value);
            let local: i64 = 7;
            value = Maybe::Some(&local);
            value
        }
    "#,
    ));
}
