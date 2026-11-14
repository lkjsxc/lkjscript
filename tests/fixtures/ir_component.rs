//! Execution probes for the unchanged production IR representation module.
mod ir;

fn flag(value: &bool) -> i64 {
    if *value { 1 } else { 0 }
}

fn place(value: usize) -> ir::Place {
    let mut result = ir::Place::local(value);
    result.projections.push(ir::Projection::Deref);
    result.projections.push(ir::Projection::Field(value + 1));
    result
}

fn operation(which: usize, changed: bool) -> ir::Op {
    let n: usize = if changed { 43 } else { 42 };
    if changed && which >= 19 && which <= 23 {
        return ir::Op::Trap {
            message: "different tag".to_string(),
        };
    }
    match which {
        0 => ir::Op::Text {
            dst: n,
            bytes: vec![0u8, 65u8, 255u8],
        },
        1 => ir::Op::Int { dst: n, value: -17 },
        2 => ir::Op::Float {
            dst: n,
            bits: 4607182418800017408u64,
        },
        3 => ir::Op::Unit { dst: n },
        4 => ir::Op::Read {
            dst: n,
            place: place(7),
            moving: true,
        },
        5 => ir::Op::Borrow {
            dst: n,
            place: place(8),
            mutable: false,
        },
        6 => ir::Op::SliceBorrow {
            dst: n,
            place: place(9),
            mutable: true,
        },
        7 => ir::Op::Array {
            dst: n,
            elements: vec![3usize, 5usize],
        },
        8 => ir::Op::Assign {
            place: place(n),
            src: 11,
        },
        9 => ir::Op::Drop { place: place(n) },
        10 => ir::Op::Unary {
            dst: n,
            op: ir::UnaryOp::Not,
            src: 12,
        },
        11 => ir::Op::Binary {
            dst: n,
            op: ir::BinaryOp::Add,
            left: 13,
            right: 14,
        },
        12 => ir::Op::Call {
            dst: n,
            function: 4,
            args: vec![15usize, 16usize],
        },
        13 => ir::Op::Struct {
            dst: n,
            fields: vec![17usize, 18usize],
        },
        14 => ir::Op::Variant {
            dst: n,
            variant: 2,
            fields: vec![19usize, 20usize],
        },
        15 => ir::Op::Discriminant { dst: n, src: 21 },
        16 => ir::Op::TagTest {
            dst: n,
            place: ir::Place::local(22),
            variant: 3,
        },
        17 => ir::Op::Intrinsic {
            dst: n,
            name: ir::IntrinsicOp::VecNew,
            args: vec![23usize, 24usize],
        },
        18 => ir::Op::If { condition: n },
        19 => ir::Op::Else,
        20 => ir::Op::End,
        21 => ir::Op::Loop,
        22 => ir::Op::Break,
        23 => ir::Op::Continue,
        24 => ir::Op::Return { src: n },
        26 => ir::Op::DropIf { place: place(n) },
        27 => ir::Op::BreakTo { depth: n },
        28 => ir::Op::ContinueTo { depth: n },
        29 => ir::Op::ViewRange {
            dst: n,
            place: place(6),
            start: 7,
            end: 8,
            inclusive: true,
            mutable: false,
        },
        _ => ir::Op::Trap {
            message: if changed {
                "changed".to_string()
            } else {
                "trap\0é水😀".to_string()
            },
        },
    }
}

fn operation_value(value: &ir::Op) -> i64 {
    match value {
        ir::Op::Text { dst, bytes } => *dst as i64 + bytes.len() as i64 + bytes[2] as i64,
        ir::Op::Int { dst, value } => *dst as i64 + *value,
        ir::Op::Float { dst, bits } => *dst as i64 + (*bits / 65536) as i64,
        ir::Op::Unit { dst } => *dst as i64,
        ir::Op::Read { dst, place, moving } => {
            *dst as i64 + place.local as i64 + place.projections.len() as i64 + flag(moving)
        }
        ir::Op::Borrow {
            dst,
            place,
            mutable,
        } => *dst as i64 + place.local as i64 + flag(mutable),
        ir::Op::SliceBorrow {
            dst,
            place,
            mutable,
        } => *dst as i64 + place.local as i64 + flag(mutable),
        ir::Op::ViewRange {
            dst,
            place,
            start,
            end,
            inclusive,
            mutable,
        } => {
            *dst as i64
                + place.local as i64
                + *start as i64
                + *end as i64
                + flag(inclusive)
                + flag(mutable)
        }
        ir::Op::Array { dst, elements } => *dst as i64 + elements[0] as i64 + elements[1] as i64,
        ir::Op::Assign { place, src } => place.local as i64 + *src as i64,
        ir::Op::Drop { place } => place.local as i64,
        ir::Op::DropIf { place } => place.local as i64 + 1,
        ir::Op::Unary { dst, op, src } => {
            *dst as i64 + op.as_str().as_bytes()[0] as i64 + *src as i64
        }
        ir::Op::Binary {
            dst,
            op,
            left,
            right,
        } => *dst as i64 + op.as_str().as_bytes()[0] as i64 + *left as i64 + *right as i64,
        ir::Op::Call {
            dst,
            function,
            args,
        } => *dst as i64 + *function as i64 + args[0] as i64 + args[1] as i64,
        ir::Op::Struct { dst, fields } => *dst as i64 + fields[0] as i64 + fields[1] as i64,
        ir::Op::Variant {
            dst,
            variant,
            fields,
        } => *dst as i64 + *variant as i64 + fields[0] as i64 + fields[1] as i64,
        ir::Op::Discriminant { dst, src } => *dst as i64 + *src as i64,
        ir::Op::TagTest {
            dst,
            place,
            variant,
        } => *dst as i64 + place.local as i64 + *variant as i64,
        ir::Op::Intrinsic { dst, name, args } => {
            *dst as i64 + name.as_str().len() as i64 + args[0] as i64 + args[1] as i64
        }
        ir::Op::If { condition } => *condition as i64,
        ir::Op::Else => 1001,
        ir::Op::End => 1002,
        ir::Op::Loop => 1003,
        ir::Op::Break => 1004,
        ir::Op::Continue => 1005,
        ir::Op::BreakTo { depth } => 1006 + *depth as i64,
        ir::Op::ContinueTo { depth } => 1007 + *depth as i64,
        ir::Op::Return { src } => *src as i64,
        ir::Op::Trap { message } => message.len() as i64,
    }
}

fn kind_value(kind: &ir::TypeKind) -> i64 {
    match kind {
        ir::TypeKind::Unit => 1,
        ir::TypeKind::Bool => 2,
        ir::TypeKind::Char => 5,
        ir::TypeKind::Int { bits, signed } => *bits as i64 * 2 + flag(signed),
        ir::TypeKind::F64 => 3,
        ir::TypeKind::Struct { fields, copy } => {
            fields.len() as i64 + fields[0].name.len() as i64 + fields[0].ty as i64 + flag(copy)
        }
        ir::TypeKind::Enum { variants, copy } => {
            variants.len() as i64
                + variants[0].name.len() as i64
                + variants[1].fields[0].ty as i64
                + flag(copy)
        }
        ir::TypeKind::Ref { inner, mutable } => *inner as i64 + flag(mutable),
        ir::TypeKind::Slice { inner, mutable } => *inner as i64 + flag(mutable),
        ir::TypeKind::Array { inner, len } => *inner as i64 + *len as i64,
        ir::TypeKind::Vec { inner } => *inner as i64,
        ir::TypeKind::Box { inner } => *inner as i64,
        ir::TypeKind::Str => 4,
        ir::TypeKind::World => 5,
        ir::TypeKind::Stdout => 6,
        ir::TypeKind::Stdin => 7,
        ir::TypeKind::Stderr => 8,
        ir::TypeKind::Directory => 9,
        ir::TypeKind::File => 10,
        ir::TypeKind::Process => 11,
        ir::TypeKind::Clock => 12,
        ir::TypeKind::Child => 13,
    }
}

fn model() -> ir::Program {
    let mut types = ir::primitives();
    types.push(ir::Type {
        name: "record".to_string(),
        kind: ir::TypeKind::Struct {
            fields: vec![ir::Field {
                name: "field\0水".to_string(),
                ty: ir::I64,
            }],
            copy: false,
        },
    });
    types.push(ir::Type {
        name: "choice".to_string(),
        kind: ir::TypeKind::Enum {
            variants: vec![
                ir::Variant {
                    name: "Empty".to_string(),
                    fields: Vec::new(),
                },
                ir::Variant {
                    name: "Full".to_string(),
                    fields: vec![ir::Field {
                        name: "value".to_string(),
                        ty: ir::I64,
                    }],
                },
            ],
            copy: false,
        },
    });
    types.push(ir::Type {
        name: "reference".to_string(),
        kind: ir::TypeKind::Ref {
            inner: ir::I64,
            mutable: true,
        },
    });
    types.push(ir::Type {
        name: "slice".to_string(),
        kind: ir::TypeKind::Slice {
            inner: ir::U8,
            mutable: false,
        },
    });
    types.push(ir::Type {
        name: "array".to_string(),
        kind: ir::TypeKind::Array {
            inner: ir::BOOL,
            len: 17,
        },
    });
    types.push(ir::Type {
        name: "vector".to_string(),
        kind: ir::TypeKind::Vec { inner: ir::I32 },
    });
    types.push(ir::Type {
        name: "box".to_string(),
        kind: ir::TypeKind::Box { inner: ir::U64 },
    });
    types.push(ir::Type {
        name: "str".to_string(),
        kind: ir::TypeKind::Str,
    });
    types.push(ir::Type {
        name: "world".to_string(),
        kind: ir::TypeKind::World,
    });
    types.push(ir::Type {
        name: "stdout".to_string(),
        kind: ir::TypeKind::Stdout,
    });
    types.push(ir::Type {
        name: "stdin".to_string(),
        kind: ir::TypeKind::Stdin,
    });
    types.push(ir::Type {
        name: "stderr".to_string(),
        kind: ir::TypeKind::Stderr,
    });
    types.push(ir::Type {
        name: "directory".to_string(),
        kind: ir::TypeKind::Directory,
    });
    types.push(ir::Type {
        name: "file".to_string(),
        kind: ir::TypeKind::File,
    });
    let mut body: Vec<ir::Instruction> = Vec::new();
    let mut index: usize = 0;
    while index < 30 {
        body.push(ir::Instruction {
            column: 1,
            op: operation(index, false),
            line: 100 + index,
        });
        index = index + 1;
    }
    let function = ir::Function {
        file: 2,
        name: "function\0é水😀".to_string(),
        params: 1,
        result: ir::I64,
        return_source: Option::Some(ir::ReturnSource::Parameter {
            index: 0,
            projections: vec![
                ir::Projection::Deref,
                ir::Projection::Field(3),
                ir::Projection::VariantField(2, 4),
                ir::Projection::Index(9),
            ],
        }),
        locals: vec![ir::Local {
            name: "parameter".to_string(),
            ty: ir::I64,
            mutable: true,
        }],
        body,
        test: true,
    };
    ir::Program {
        types,
        functions: vec![function],
        destructors: vec![(7usize, 0usize)],
    }
}

pub fn primitive_snapshot(index: usize, field: usize) -> i64 {
    let original = ir::primitives();
    let copied = original.clone();
    drop(original);
    if field == 0 {
        return kind_value(&copied[index].kind);
    }
    if field == 1 {
        return copied[index].name.len() as i64;
    }
    let bytes = copied[index].name.as_bytes();
    if field - 2 < bytes.len() {
        bytes[field - 2] as i64
    } else {
        -1
    }
}

pub fn capability_snapshot(which: usize) -> i64 {
    let original = match which {
        0 => ir::TypeKind::Process,
        1 => ir::TypeKind::Clock,
        _ => ir::TypeKind::Child,
    };
    let copied = original.clone();
    assert!(copied == original);
    drop(original);
    let different = if which == 0 {
        ir::TypeKind::Clock
    } else {
        ir::TypeKind::Process
    };
    assert!(copied != different);
    kind_value(&copied)
}

pub fn operation_snapshot(which: usize) -> i64 {
    let original = operation(which, false);
    let copied = original.clone();
    assert!(original == copied);
    drop(original);
    operation_value(&copied)
}

pub fn operation_equality(which: usize, changed: bool) -> i64 {
    let original = operation(which, false);
    let candidate = operation(which, changed);
    let result = original == candidate;
    assert!((original != candidate) == !result);
    if result { 1 } else { 0 }
}

pub fn program_snapshot(field: usize) -> i64 {
    let mut original = model();
    let copied = original.clone();
    assert!(copied == original);
    original.functions[0].name.push_str(" changed");
    original.functions[0].body[0].op = operation(0, true);
    original.destructors[0].0 = 99;
    assert!(copied != original);
    drop(original);
    let checked = ir::CheckedProgram { program: copied };
    let retained: ir::Program = checked.program().clone();
    drop(checked);
    assert!(retained == model());
    if field == 0 {
        let mut total: i64 = 0;
        let mut index: usize = 0;
        while index < retained.types.len() {
            total = total + kind_value(&retained.types[index].kind);
            index = index + 1;
        }
        return total;
    }
    if field == 1 {
        let mut total: i64 = 0;
        let mut index: usize = 0;
        while index < retained.functions[0].body.len() {
            total = total + operation_value(&retained.functions[0].body[index].op);
            total = total + retained.functions[0].body[index].line as i64;
            index = index + 1;
        }
        return total;
    }
    if field == 2 {
        return retained.functions[0].name.len() as i64;
    }
    if field == 3 {
        return retained.destructors[0].0 as i64;
    }
    if field == 4 {
        return retained.functions[0].locals[0].name.len() as i64;
    }
    match &retained.functions[0].return_source {
        Option::None => -1,
        Option::Some(source) => match source {
            ir::ReturnSource::Static => -2,
            ir::ReturnSource::Leaves(leaves) => leaves.len() as i64,
            ir::ReturnSource::Parameter { index, projections } => {
                *index as i64 + projections.len() as i64
            }
        },
    }
}

fn projections_value(projections: &Vec<ir::Projection>) -> i64 {
    let mut result: i64 = 0;
    let mut cursor: usize = 0;
    while cursor < projections.len() {
        result = result
            + match &projections[cursor] {
                ir::Projection::Deref => 1,
                ir::Projection::Field(value) => *value as i64,
                ir::Projection::VariantField(tag, value) => *tag as i64 + *value as i64,
                ir::Projection::Index(value) => *value as i64,
            };
        cursor = cursor + 1;
    }
    result
}

pub fn return_source_snapshot(which: usize) -> i64 {
    let original: Option<ir::ReturnSource> = if which == 0 {
        Option::None
    } else if which == 1 {
        Option::Some(ir::ReturnSource::Static)
    } else if which == 2 {
        Option::Some(ir::ReturnSource::Parameter {
            index: 17,
            projections: vec![
                ir::Projection::Deref,
                ir::Projection::Field(2),
                ir::Projection::VariantField(3, 4),
                ir::Projection::Index(5),
            ],
        })
    } else {
        let origin = if which == 5 {
            ir::LeafOrigin::Static
        } else {
            ir::LeafOrigin::Inputs {
                inputs: vec![
                    ir::InputLeaf {
                        index: 11,
                        projections: vec![ir::Projection::Deref, ir::Projection::Field(7)],
                    },
                    ir::InputLeaf {
                        index: 13,
                        projections: vec![ir::Projection::Field(2)],
                    },
                ],
                exact: which == 3,
            }
        };
        Option::Some(ir::ReturnSource::Leaves(vec![ir::ReturnLeaf {
            projections: vec![ir::Projection::VariantField(2, 9)],
            origin,
        }]))
    };
    let copied = original.clone();
    assert!(copied == original);
    drop(original);
    match copied {
        Option::None => 100,
        Option::Some(source) => match source {
            ir::ReturnSource::Static => 101,
            ir::ReturnSource::Parameter { index, projections } => {
                index as i64 + projections_value(&projections)
            }
            ir::ReturnSource::Leaves(leaves) => {
                let mut result: i64 = 0;
                let mut cursor: usize = 0;
                while cursor < leaves.len() {
                    result = result + projections_value(&leaves[cursor].projections);
                    result = result
                        + match &leaves[cursor].origin {
                            ir::LeafOrigin::Static => 103,
                            ir::LeafOrigin::Inputs { inputs, exact } => {
                                let mut value = flag(exact);
                                let mut index: usize = 0;
                                while index < inputs.len() {
                                    value = value
                                        + inputs[index].index as i64
                                        + projections_value(&inputs[index].projections);
                                    index = index + 1;
                                }
                                value
                            }
                        };
                    cursor = cursor + 1;
                }
                result
            }
        },
    }
}

pub fn production_place_local() -> usize {
    ir::Place::local(42).local
}
