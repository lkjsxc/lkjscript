//! Execution probes for the unchanged production K representation module.
mod k;

fn flag(value: &bool) -> i64 {
    if *value { 1 } else { 0 }
}

fn place(value: usize) -> k::Place {
    let mut result = k::Place::local(value);
    result.projections.push(k::Projection::Deref);
    result.projections.push(k::Projection::Field(value + 1));
    result
}

fn operation(which: usize, changed: bool) -> k::Op {
    let n: usize = if changed { 43 } else { 42 };
    if changed && which >= 19 && which <= 23 {
        return k::Op::Trap {
            message: "different tag".to_string(),
        };
    }
    match which {
        0 => k::Op::Text {
            dst: n,
            bytes: vec![0u8, 65u8, 255u8],
        },
        1 => k::Op::Int { dst: n, value: -17 },
        2 => k::Op::Float {
            dst: n,
            bits: 4607182418800017408u64,
        },
        3 => k::Op::Unit { dst: n },
        4 => k::Op::Read {
            dst: n,
            place: place(7),
            moving: true,
        },
        5 => k::Op::Borrow {
            dst: n,
            place: place(8),
            mutable: false,
        },
        6 => k::Op::SliceBorrow {
            dst: n,
            place: place(9),
            mutable: true,
        },
        7 => k::Op::Array {
            dst: n,
            elements: vec![3usize, 5usize],
        },
        8 => k::Op::Assign {
            place: place(n),
            src: 11,
        },
        9 => k::Op::Drop { place: place(n) },
        10 => k::Op::Unary {
            dst: n,
            op: "!".to_string(),
            src: 12,
        },
        11 => k::Op::Binary {
            dst: n,
            op: "+".to_string(),
            left: 13,
            right: 14,
        },
        12 => k::Op::Call {
            dst: n,
            function: 4,
            args: vec![15usize, 16usize],
        },
        13 => k::Op::Struct {
            dst: n,
            fields: vec![17usize, 18usize],
        },
        14 => k::Op::Variant {
            dst: n,
            variant: 2,
            fields: vec![19usize, 20usize],
        },
        15 => k::Op::Discriminant { dst: n, src: 21 },
        16 => k::Op::TagTest {
            dst: n,
            place: k::Place::local(22),
            variant: 3,
        },
        17 => k::Op::Intrinsic {
            dst: n,
            name: "fixture\0水".to_string(),
            args: vec![23usize, 24usize],
        },
        18 => k::Op::If { condition: n },
        19 => k::Op::Else,
        20 => k::Op::End,
        21 => k::Op::Loop,
        22 => k::Op::Break,
        23 => k::Op::Continue,
        24 => k::Op::Return { src: n },
        26 => k::Op::DropIf { place: place(n) },
        27 => k::Op::BreakTo { depth: n },
        28 => k::Op::ContinueTo { depth: n },
        _ => k::Op::Trap {
            message: if changed {
                "changed".to_string()
            } else {
                "trap\0é水😀".to_string()
            },
        },
    }
}

fn operation_value(value: &k::Op) -> i64 {
    match value {
        k::Op::Text { dst, bytes } => *dst as i64 + bytes.len() as i64 + bytes[2] as i64,
        k::Op::Int { dst, value } => *dst as i64 + *value,
        k::Op::Float { dst, bits } => *dst as i64 + (*bits / 65536) as i64,
        k::Op::Unit { dst } => *dst as i64,
        k::Op::Read { dst, place, moving } => {
            *dst as i64 + place.local as i64 + place.projections.len() as i64 + flag(moving)
        }
        k::Op::Borrow {
            dst,
            place,
            mutable,
        } => *dst as i64 + place.local as i64 + flag(mutable),
        k::Op::SliceBorrow {
            dst,
            place,
            mutable,
        } => *dst as i64 + place.local as i64 + flag(mutable),
        k::Op::Array { dst, elements } => *dst as i64 + elements[0] as i64 + elements[1] as i64,
        k::Op::Assign { place, src } => place.local as i64 + *src as i64,
        k::Op::Drop { place } => place.local as i64,
        k::Op::DropIf { place } => place.local as i64 + 1,
        k::Op::Unary { dst, op, src } => *dst as i64 + op.as_bytes()[0] as i64 + *src as i64,
        k::Op::Binary {
            dst,
            op,
            left,
            right,
        } => *dst as i64 + op.as_bytes()[0] as i64 + *left as i64 + *right as i64,
        k::Op::Call {
            dst,
            function,
            args,
        } => *dst as i64 + *function as i64 + args[0] as i64 + args[1] as i64,
        k::Op::Struct { dst, fields } => *dst as i64 + fields[0] as i64 + fields[1] as i64,
        k::Op::Variant {
            dst,
            variant,
            fields,
        } => *dst as i64 + *variant as i64 + fields[0] as i64 + fields[1] as i64,
        k::Op::Discriminant { dst, src } => *dst as i64 + *src as i64,
        k::Op::TagTest { dst, place, variant } => *dst as i64 + place.local as i64 + *variant as i64,
        k::Op::Intrinsic { dst, name, args } => {
            *dst as i64 + name.len() as i64 + args[0] as i64 + args[1] as i64
        }
        k::Op::If { condition } => *condition as i64,
        k::Op::Else => 1001,
        k::Op::End => 1002,
        k::Op::Loop => 1003,
        k::Op::Break => 1004,
        k::Op::Continue => 1005,
        k::Op::BreakTo { depth } => 1006 + *depth as i64,
        k::Op::ContinueTo { depth } => 1007 + *depth as i64,
        k::Op::Return { src } => *src as i64,
        k::Op::Trap { message } => message.len() as i64,
    }
}

fn kind_value(kind: &k::TypeKind) -> i64 {
    match kind {
        k::TypeKind::Unit => 1,
        k::TypeKind::Bool => 2,
        k::TypeKind::Int { bits, signed } => *bits as i64 * 2 + flag(signed),
        k::TypeKind::F64 => 3,
        k::TypeKind::Struct { fields, copy } => {
            fields.len() as i64 + fields[0].name.len() as i64 + fields[0].ty as i64 + flag(copy)
        }
        k::TypeKind::Enum { variants, copy } => {
            variants.len() as i64
                + variants[0].name.len() as i64
                + variants[1].fields[0].ty as i64
                + flag(copy)
        }
        k::TypeKind::Ref { inner, mutable } => *inner as i64 + flag(mutable),
        k::TypeKind::Slice { inner, mutable } => *inner as i64 + flag(mutable),
        k::TypeKind::Array { inner, len } => *inner as i64 + *len as i64,
        k::TypeKind::Vec { inner } => *inner as i64,
        k::TypeKind::Box { inner } => *inner as i64,
        k::TypeKind::Str => 4,
        k::TypeKind::World => 5,
        k::TypeKind::Stdout => 6,
    }
}

fn model() -> k::Program {
    let mut types = k::primitives();
    types.push(k::Type {
        name: "record".to_string(),
        kind: k::TypeKind::Struct {
            fields: vec![k::Field {
                name: "field\0水".to_string(),
                ty: k::I64,
            }],
            copy: false,
        },
    });
    types.push(k::Type {
        name: "choice".to_string(),
        kind: k::TypeKind::Enum {
            variants: vec![
                k::Variant {
                    name: "Empty".to_string(),
                    fields: Vec::new(),
                },
                k::Variant {
                    name: "Full".to_string(),
                    fields: vec![k::Field {
                        name: "value".to_string(),
                        ty: k::I64,
                    }],
                },
            ],
            copy: false,
        },
    });
    types.push(k::Type {
        name: "reference".to_string(),
        kind: k::TypeKind::Ref {
            inner: k::I64,
            mutable: true,
        },
    });
    types.push(k::Type {
        name: "slice".to_string(),
        kind: k::TypeKind::Slice {
            inner: k::U8,
            mutable: false,
        },
    });
    types.push(k::Type {
        name: "array".to_string(),
        kind: k::TypeKind::Array {
            inner: k::BOOL,
            len: 17,
        },
    });
    types.push(k::Type {
        name: "vector".to_string(),
        kind: k::TypeKind::Vec { inner: k::I32 },
    });
    types.push(k::Type {
        name: "box".to_string(),
        kind: k::TypeKind::Box { inner: k::U64 },
    });
    types.push(k::Type {
        name: "str".to_string(),
        kind: k::TypeKind::Str,
    });
    types.push(k::Type {
        name: "world".to_string(),
        kind: k::TypeKind::World,
    });
    let mut body: Vec<k::Instruction> = Vec::new();
    let mut index: usize = 0;
    while index < 29 {
        body.push(k::Instruction { column: 1,
            op: operation(index, false),
            line: 100 + index,
        });
        index = index + 1;
    }
    let function = k::Function {
        file: 2,
        name: "function\0é水😀".to_string(),
        params: 1,
        result: k::I64,
        return_source: Option::Some(k::ReturnSource::Parameter {
            index: 0,
            projections: vec![
                k::Projection::Deref,
                k::Projection::Field(3),
                k::Projection::VariantField(2, 4),
                k::Projection::Index(9),
            ],
        }),
        locals: vec![k::Local {
            name: "parameter".to_string(),
            ty: k::I64,
            mutable: true,
        }],
        body,
        test: true,
    };
    k::Program {
        types,
        functions: vec![function],
        destructors: vec![(7usize, 0usize)],
    }
}

pub fn primitive_snapshot(index: usize, field: usize) -> i64 {
    let original = k::primitives();
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
    let checked = k::CheckedProgram { program: copied };
    let retained: k::Program = checked.program().clone();
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
            k::ReturnSource::Static => -2,
            k::ReturnSource::Leaves(leaves) => leaves.len() as i64,
            k::ReturnSource::Parameter { index, projections } => {
                *index as i64 + projections.len() as i64
            }
        },
    }
}

fn projections_value(projections: &Vec<k::Projection>) -> i64 {
    let mut result: i64 = 0;
    let mut cursor: usize = 0;
    while cursor < projections.len() {
        result = result + match &projections[cursor] {
            k::Projection::Deref => 1,
            k::Projection::Field(value) => *value as i64,
            k::Projection::VariantField(tag, value) => *tag as i64 + *value as i64,
            k::Projection::Index(value) => *value as i64,
        };
        cursor = cursor + 1;
    }
    result
}

pub fn return_source_snapshot(which: usize) -> i64 {
    let original: Option<k::ReturnSource> = if which == 0 {
        Option::None
    } else if which == 1 {
        Option::Some(k::ReturnSource::Static)
    } else if which == 2 {
        Option::Some(k::ReturnSource::Parameter {
            index: 17,
            projections: vec![
                k::Projection::Deref,
                k::Projection::Field(2),
                k::Projection::VariantField(3, 4),
                k::Projection::Index(5),
            ],
        })
    } else {
        let origin = if which == 5 {
            k::LeafOrigin::Static
        } else {
            k::LeafOrigin::Parameter {
                index: 11,
                projections: vec![k::Projection::Deref, k::Projection::Field(7)],
                exact: which == 3,
            }
        };
        Option::Some(k::ReturnSource::Leaves(vec![k::ReturnLeaf {
            projections: vec![k::Projection::VariantField(2, 9)],
            origin,
        }]))
    };
    let copied = original.clone();
    assert!(copied == original);
    drop(original);
    match copied {
        Option::None => 100,
        Option::Some(source) => match source {
            k::ReturnSource::Static => 101,
            k::ReturnSource::Parameter { index, projections } => {
                index as i64 + projections_value(&projections)
            }
            k::ReturnSource::Leaves(leaves) => {
                let mut result: i64 = 0;
                let mut cursor: usize = 0;
                while cursor < leaves.len() {
                    result = result + projections_value(&leaves[cursor].projections);
                    result = result + match &leaves[cursor].origin {
                        k::LeafOrigin::Static => 103,
                        k::LeafOrigin::Parameter { index, projections, exact } => {
                            *index as i64 + projections_value(projections) + flag(exact)
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
    k::Place::local(42).local
}
