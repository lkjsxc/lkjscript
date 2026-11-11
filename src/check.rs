//! The single gate for explicit K. Each structured edge has an ownership state.
use crate::ast::{Diagnostic, Span};
use crate::k::*;

fn error(line: usize, code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(
        Span {
            line,
            column: 1,
            start: 0,
            end: 0,
        },
        code,
        message,
    )
}

pub fn is_copy(types: &[Type], ty: usize) -> bool {
    let mut pending = vec![ty];
    let mut seen = vec![false; types.len()];
    while let Some(current) = pending.pop() {
        if current >= types.len() {
            return false;
        }
        if seen[current] {
            continue;
        }
        seen[current] = true;
        match &types[current].kind {
            TypeKind::Unit | TypeKind::Bool | TypeKind::Int { .. } | TypeKind::F64 => {}
            TypeKind::Ref { mutable, .. } | TypeKind::Slice { mutable, .. } => {
                if *mutable {
                    return false;
                }
            }
            TypeKind::Struct { fields, copy } => {
                if !copy {
                    return false;
                }
                for field in fields {
                    pending.push(field.ty);
                }
            }
            TypeKind::Enum { variants, copy } => {
                if !copy {
                    return false;
                }
                for variant in variants {
                    for field in &variant.fields {
                        pending.push(field.ty);
                    }
                }
            }
            _ => return false,
        }
    }
    true
}

pub fn is_data(types: &[Type], ty: usize) -> bool {
    let mut pending = vec![ty];
    let mut seen = vec![false; types.len()];
    while let Some(current) = pending.pop() {
        if current >= types.len() {
            return false;
        }
        if seen[current] {
            continue;
        }
        seen[current] = true;
        match &types[current].kind {
            TypeKind::World => return false,
            TypeKind::Ref { inner, .. }
            | TypeKind::Slice { inner, .. }
            | TypeKind::Vec { inner }
            | TypeKind::Box { inner } => pending.push(*inner),
            TypeKind::Struct { fields, .. } => {
                for field in fields {
                    pending.push(field.ty);
                }
            }
            TypeKind::Enum { variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        pending.push(field.ty);
                    }
                }
            }
            _ => {}
        }
    }
    true
}

pub fn place_type(types: &[Type], function: &Function, place: &Place) -> Option<usize> {
    let mut ty = function.locals.get(place.local)?.ty;
    for projection in &place.projections {
        ty = match (projection, &types.get(ty)?.kind) {
            (Projection::Deref, TypeKind::Ref { inner, .. } | TypeKind::Box { inner }) => *inner,
            (Projection::Field(index), TypeKind::Struct { fields, .. }) => fields.get(*index)?.ty,
            (Projection::VariantField(tag, index), TypeKind::Enum { variants, .. }) => {
                variants.get(*tag)?.fields.get(*index)?.ty
            }
            (Projection::Index(index), TypeKind::Vec { inner } | TypeKind::Slice { inner, .. }) => {
                if function.locals.get(*index)?.ty != U64 {
                    return None;
                }
                *inner
            }
            _ => return None,
        };
    }
    Some(ty)
}

fn heap_type(kind: &TypeKind) -> bool {
    matches!(
        kind,
        TypeKind::Struct { .. }
            | TypeKind::Enum { .. }
            | TypeKind::Vec { .. }
            | TypeKind::Box { .. }
            | TypeKind::String
    )
}

fn path_overlaps(a: &[Projection], b: &[Projection]) -> bool {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return !matches!((&a[i], &b[i]), (Projection::Field(_), Projection::Field(_)));
        }
    }
    true
}

fn is_prefix(a: &[Projection], b: &[Projection]) -> bool {
    a.len() <= b.len() && a == &b[..a.len()]
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Root {
    Local(usize),
    Parameter(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Origin {
    root: Root,
    projections: Vec<Projection>,
}

impl Origin {
    fn local(local: usize) -> Origin {
        Origin {
            root: Root::Local(local),
            projections: Vec::new(),
        }
    }
}

fn overlaps(a: &Origin, b: &Origin) -> bool {
    a.root == b.root && path_overlaps(&a.projections, &b.projections)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Loan {
    local: usize,
    target: Origin,
    mutable: bool,
    parents: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Tag {
    target: Origin,
    variant: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct State {
    live: Vec<bool>,
    moved: Vec<Place>,
    loans: Vec<Loan>,
    tags: Vec<Tag>,
    predicates: Vec<Option<Tag>>,
    reachable: bool,
}

#[derive(Clone)]
struct Control {
    entry: State,
    then_state: Option<State>,
    is_loop: bool,
    breaks: Vec<State>,
}

fn origin(state: &State, place: &Place) -> (Origin, Vec<usize>) {
    let mut result = Origin {
        root: Root::Local(place.local),
        projections: place.projections.clone(),
    };
    let mut parents = Vec::new();
    loop {
        if !matches!(result.projections.first(), Some(Projection::Deref)) {
            return (result, parents);
        }
        let local = match result.root {
            Root::Local(local) => local,
            Root::Parameter(_) => return (result, parents),
        };
        if parents.contains(&local) {
            return (result, parents);
        }
        match state.loans.iter().find(|loan| loan.local == local) {
            Some(loan) => {
                parents.push(local);
                for parent in &loan.parents {
                    if !parents.contains(parent) {
                        parents.push(*parent);
                    }
                }
                let mut next = loan.target.clone();
                next.projections.extend_from_slice(&result.projections[1..]);
                result = next;
            }
            None => return (result, parents),
        }
    }
}

fn affects_tag(target: &Origin, tag: &Tag) -> bool {
    target.projections.len() <= tag.target.projections.len() && overlaps(target, &tag.target)
}

fn invalidate(state: &mut State, target: &Origin) {
    state.tags.retain(|tag| !affects_tag(target, tag));
    for predicate in &mut state.predicates {
        if predicate
            .as_ref()
            .is_some_and(|tag| affects_tag(target, tag))
        {
            *predicate = None;
        }
    }
}

fn tag_at(state: &State, target: &Origin) -> Option<usize> {
    state
        .tags
        .iter()
        .find(|tag| tag.target == *target)
        .map(|tag| tag.variant)
}

fn set_tag(state: &mut State, target: Origin, variant: usize) {
    state.tags.retain(|tag| tag.target != target);
    state.tags.push(Tag { target, variant });
}

fn place_writable(types: &[Type], function: &Function, place: &Place) -> bool {
    let mut writable = function.locals[place.local].mutable;
    let mut blocked = false;
    let mut ty = function.locals[place.local].ty;
    let mut prefix = Place::local(place.local);
    for projection in &place.projections {
        match (projection, &types[ty].kind) {
            (Projection::Deref, TypeKind::Ref { mutable, .. }) => {
                if !mutable {
                    blocked = true;
                }
                writable = *mutable && !blocked;
            }
            (Projection::Index(_), TypeKind::Slice { mutable, .. }) => {
                if !mutable {
                    blocked = true;
                }
                writable = *mutable && !blocked;
            }
            _ => {}
        }
        prefix.projections.push(projection.clone());
        ty = place_type(types, function, &prefix).unwrap();
    }
    writable && !blocked
}

fn borrowed_projection(types: &[Type], function: &Function, place: &Place) -> bool {
    let mut ty = function.locals[place.local].ty;
    let mut prefix = Place::local(place.local);
    for projection in &place.projections {
        if matches!(
            (projection, &types[ty].kind),
            (Projection::Deref, TypeKind::Ref { .. }) | (Projection::Index(_), _)
        ) {
            return true;
        }
        prefix.projections.push(projection.clone());
        ty = place_type(types, function, &prefix).unwrap();
    }
    false
}

// mode 0 reads the whole place; mode 1 replaces/drops its remaining fields.
fn access(
    state: &State,
    types: &[Type],
    function: &Function,
    place: &Place,
    write: bool,
    mode: u8,
    line: usize,
) -> Result<(), Diagnostic> {
    if !state.live[place.local] {
        return Err(error(line, "E_MOVE", "value is uninitialized or moved"));
    }
    let mut prefix = Place::local(place.local);
    for projection in &place.projections {
        if let Projection::Index(index) = projection {
            if !state.live[*index] {
                return Err(error(line, "E_MOVE", "index is uninitialized or moved"));
            }
        }
        if let Projection::VariantField(variant, _) = projection {
            let target = origin(state, &prefix).0;
            if tag_at(state, &target) != Some(*variant) {
                return Err(error(
                    line,
                    "E_TYPE",
                    "enum field needs a matching checked tag branch",
                ));
            }
        }
        prefix.projections.push(projection.clone());
    }
    let (target, parents) = origin(state, place);
    if let Root::Local(local) = target.root {
        if !state.live[local] {
            return Err(error(line, "E_MOVE", "borrow points to a moved owner"));
        }
        for moved in &state.moved {
            if moved.local == local && path_overlaps(&moved.projections, &target.projections) {
                let ancestor = is_prefix(&moved.projections, &target.projections)
                    && moved.projections.len() < target.projections.len();
                if mode == 0 || ancestor {
                    return Err(error(line, "E_MOVE", "place contains a moved field"));
                }
            }
        }
    }
    for loan in &state.loans {
        if loan.local == place.local || parents.contains(&loan.local) {
            continue;
        }
        if overlaps(&loan.target, &target) && (write || loan.mutable) {
            return Err(error(
                line,
                "E_BORROW",
                "place conflicts with a live borrow",
            ));
        }
    }
    let _ = (types, function);
    Ok(())
}

fn consume(
    state: &mut State,
    types: &[Type],
    function: &Function,
    local: usize,
    line: usize,
) -> Result<(), Diagnostic> {
    access(state, types, function, &Place::local(local), true, 0, line)?;
    state.live[local] = false;
    state.loans.retain(|loan| loan.local != local);
    state.moved.retain(|place| place.local != local);
    invalidate(state, &Origin::local(local));
    state.predicates[local] = None;
    Ok(())
}

fn put(state: &mut State, local: usize, line: usize) -> Result<(), Diagnostic> {
    if state.live[local] {
        return Err(error(line, "E_K", "destination still owns a value"));
    }
    state.live[local] = true;
    state.predicates[local] = None;
    invalidate(state, &Origin::local(local));
    Ok(())
}

fn same_set<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len() && a.iter().all(|item| b.contains(item))
}

fn merge(mut a: State, b: State, line: usize) -> Result<State, Diagnostic> {
    if !a.reachable {
        return Ok(b);
    }
    if !b.reachable {
        return Ok(a);
    }
    if a.live != b.live || !same_set(&a.moved, &b.moved) || !same_set(&a.loans, &b.loans) {
        return Err(error(
            line,
            "E_MOVE",
            "branches must leave the same owners and borrows",
        ));
    }
    a.tags.retain(|tag| b.tags.contains(tag));
    for i in 0..a.predicates.len() {
        if a.predicates[i] != b.predicates[i] {
            a.predicates[i] = None;
        }
    }
    Ok(a)
}

fn local_type(function: &Function, local: usize, line: usize) -> Result<usize, Diagnostic> {
    function
        .locals
        .get(local)
        .map(|l| l.ty)
        .ok_or_else(|| error(line, "E_K", "local index is not declared"))
}

fn enum_place(types: &[Type], function: &Function, local: usize) -> Place {
    let mut place = Place::local(local);
    if matches!(types[function.locals[local].ty].kind, TypeKind::Ref { .. }) {
        place.projections.push(Projection::Deref);
    }
    place
}

fn check_intrinsic(
    types: &[Type],
    function: &Function,
    dst: usize,
    name: &str,
    args: &[usize],
    line: usize,
) -> Result<(), Diagnostic> {
    let dt = local_type(function, dst, line)?;
    let mut ts = Vec::new();
    for arg in args {
        ts.push(local_type(function, *arg, line)?);
    }
    let inner = |t: usize| -> usize {
        match types[t].kind {
            TypeKind::Ref { inner, .. } => inner,
            _ => t,
        }
    };
    let valid = match name {
        "assert" => ts == vec![BOOL] && dt == UNIT,
        "vec_new" => ts.is_empty() && matches!(types[dt].kind, TypeKind::Vec { .. }),
        "vec_len" => {
            ts.len() == 1
                && dt == U64
                && matches!(types[ts[0]].kind, TypeKind::Ref { .. })
                && matches!(
                    types[inner(ts[0])].kind,
                    TypeKind::Vec { .. } | TypeKind::Slice { .. } | TypeKind::String
                )
        }
        "vec_push" => {
            ts.len() == 2
                && dt == UNIT
                && match types[ts[0]].kind {
                    TypeKind::Ref {
                        inner: container,
                        mutable: true,
                    } => {
                        matches!(types[container].kind, TypeKind::Vec { inner } if inner == ts[1])
                    }
                    _ => false,
                }
        }
        "vec_pop" => {
            ts.len() == 1
                && match types[ts[0]].kind {
                    TypeKind::Ref {
                        inner: container,
                        mutable: true,
                    } => match types[container].kind {
                        TypeKind::Vec { inner } => {
                            matches!(&types[dt].kind, TypeKind::Enum { variants, .. }
                    if variants.len() == 2 && variants[0].name == "None" && variants[0].fields.is_empty()
                    && variants[1].name == "Some" && variants[1].fields.len() == 1 && variants[1].fields[0].ty == inner)
                        }
                        _ => false,
                    },
                    _ => false,
                }
        }
        "replace" => {
            ts.len() == 2
                && ts[1] == dt
                && !matches!(
                    types[dt].kind,
                    TypeKind::Ref { .. } | TypeKind::Slice { .. }
                )
                && matches!(types[ts[0]].kind, TypeKind::Ref { inner, mutable: true } if inner == dt)
        }
        "box_new" => {
            ts.len() == 1 && matches!(types[dt].kind, TypeKind::Box { inner } if inner == ts[0])
        }
        "cast" => {
            ts.len() == 1
                && match (&types[ts[0]].kind, &types[dt].kind) {
                    (TypeKind::Bool, TypeKind::Int { .. }) => true,
                    (
                        TypeKind::Int { .. } | TypeKind::F64,
                        TypeKind::Int { .. } | TypeKind::F64,
                    ) => true,
                    _ => false,
                }
        }
        "clone" => {
            ts.len() == 1
                && dt == inner(ts[0])
                && matches!(types[ts[0]].kind, TypeKind::Ref { .. })
                && !matches!(
                    types[dt].kind,
                    TypeKind::Ref { .. } | TypeKind::Slice { .. }
                )
                && is_copy(types, dt)
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(error(line, "E_TYPE", "invalid intrinsic or signature"))
    }
}

// Validate instruction shape even when its control path is unreachable.
fn check_shape(
    program: &Program,
    function: &Function,
    instruction: &Instruction,
) -> Result<(), Diagnostic> {
    let types = &program.types;
    let line = instruction.line;
    let ty = |local: usize| local_type(function, local, line);
    let place_ty = |place: &Place| {
        place_type(types, function, place)
            .ok_or_else(|| error(line, "E_TYPE", "invalid place projection"))
    };
    match &instruction.op {
        Op::Int { dst, value } => match &types[ty(*dst)?].kind {
            TypeKind::Bool if *value == 0 || *value == 1 => {}
            TypeKind::Int { bits, signed } => {
                if *bits < 64 {
                    let min = if *signed { -(1_i64 << (*bits - 1)) } else { 0 };
                    let max = if *signed {
                        (1_i64 << (*bits - 1)) - 1
                    } else {
                        (1_i64 << *bits) - 1
                    };
                    if *value < min || *value > max {
                        return Err(error(
                            line,
                            "E_TYPE",
                            "integer literal does not fit its type",
                        ));
                    }
                }
            }
            _ => return Err(error(line, "E_TYPE", "integer cannot construct this type")),
        },
        Op::Float { dst, bits } => {
            if ty(*dst)? != F64 {
                return Err(error(line, "E_TYPE", "float destination is not f64"));
            }
            if bits & 0x7ff0000000000000 == 0x7ff0000000000000
                && bits & 0x000fffffffffffff != 0
                && *bits != 0x7ff8000000000000
            {
                return Err(error(
                    line,
                    "E_TYPE",
                    "NaN literal must use the canonical representation",
                ));
            }
        }
        Op::Unit { dst } => {
            if ty(*dst)? != UNIT {
                return Err(error(line, "E_TYPE", "unit destination has a value type"));
            }
        }
        Op::Read { dst, place, moving } => {
            let source = place_ty(place)?;
            if ty(*dst)? != source {
                return Err(error(line, "E_TYPE", "read destination type differs"));
            }
            if !moving && !is_copy(types, source) {
                return Err(error(line, "E_MOVE", "non-Copy value must move"));
            }
            if *moving && borrowed_projection(types, function, place) {
                return Err(error(
                    line,
                    "E_BORROW",
                    "cannot move out of a borrow or indexed collection",
                ));
            }
        }
        Op::Borrow {
            dst,
            place,
            mutable,
        } => {
            let source = place_ty(place)?;
            if !matches!(types[ty(*dst)?].kind,
                TypeKind::Ref { inner, mutable: kind_mut } if inner == source && kind_mut == *mutable)
            {
                return Err(error(line, "E_TYPE", "borrow type differs"));
            }
            if *mutable && !place_writable(types, function, place) {
                return Err(error(
                    line,
                    "E_BORROW",
                    "mutable borrow through an immutable place",
                ));
            }
        }
        Op::Assign { place, src } => {
            if place_ty(place)? != ty(*src)? {
                return Err(error(line, "E_TYPE", "assignment type differs"));
            }
            if !place.projections.is_empty()
                && matches!(
                    types[ty(*src)?].kind,
                    TypeKind::Ref { .. } | TypeKind::Slice { .. }
                )
            {
                return Err(error(
                    line,
                    "E_UNSUPPORTED",
                    "assigning a reference through a projection needs view provenance support",
                ));
            }
            if !place.projections.is_empty() && !place_writable(types, function, place) {
                return Err(error(
                    line,
                    "E_BORROW",
                    "assignment through an immutable place",
                ));
            }
        }
        Op::Drop { place } => {
            let _ = place_ty(place)?;
            if borrowed_projection(types, function, place) {
                return Err(error(
                    line,
                    "E_BORROW",
                    "cannot drop through a borrow or indexed collection",
                ));
            }
        }
        Op::Unary { dst, op, src } => {
            let t = ty(*src)?;
            let valid = match (op.as_str(), &types[t].kind) {
                ("-", TypeKind::Int { signed: true, .. } | TypeKind::F64) => true,
                ("!", TypeKind::Int { .. } | TypeKind::Bool) => true,
                _ => false,
            };
            if ty(*dst)? != t || !valid {
                return Err(error(line, "E_TYPE", "invalid unary operation"));
            }
        }
        Op::Binary {
            dst,
            op,
            left,
            right,
        } => {
            let t = ty(*left)?;
            if t != ty(*right)? {
                return Err(error(line, "E_TYPE", "binary operand types differ"));
            }
            let comparison = ["==", "!=", "<", "<=", ">", ">="].contains(&op.as_str());
            if ty(*dst)? != if comparison { BOOL } else { t } {
                return Err(error(line, "E_TYPE", "binary result type differs"));
            }
            let allowed = match types[t].kind {
                TypeKind::Bool => ["==", "!=", "&", "|", "^", "&&", "||"].contains(&op.as_str()),
                TypeKind::Int { .. } => [
                    "+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", "==", "!=", "<", "<=", ">",
                    ">=",
                ]
                .contains(&op.as_str()),
                TypeKind::F64 => {
                    ["+", "-", "*", "/", "==", "!=", "<", "<=", ">", ">="].contains(&op.as_str())
                }
                _ => false,
            };
            if !allowed {
                return Err(error(
                    line,
                    "E_TYPE",
                    "operator is not defined for this type",
                ));
            }
        }
        Op::Call {
            dst,
            function: called,
            args,
        } => {
            let target = program
                .functions
                .get(*called)
                .ok_or_else(|| error(line, "E_K", "call target is not declared"))?;
            if target.params != args.len() || target.result != ty(*dst)? {
                return Err(error(line, "E_TYPE", "call signature differs"));
            }
            for i in 0..args.len() {
                if ty(args[i])? != target.locals[i].ty {
                    return Err(error(
                        line,
                        "E_TYPE",
                        &format!(
                            "call {} argument {} expects {}, found {}",
                            target.name,
                            i + 1,
                            types[target.locals[i].ty].name,
                            types[ty(args[i])?].name
                        ),
                    ));
                }
            }
        }
        Op::Struct { dst, fields } | Op::Variant { dst, fields, .. } => {
            let expected = match (&instruction.op, &types[ty(*dst)?].kind) {
                (Op::Struct { .. }, TypeKind::Struct { fields, .. }) => fields,
                (Op::Variant { variant, .. }, TypeKind::Enum { variants, .. }) => {
                    &variants
                        .get(*variant)
                        .ok_or_else(|| error(line, "E_TYPE", "invalid enum variant"))?
                        .fields
                }
                _ => return Err(error(line, "E_TYPE", "constructor has wrong type")),
            };
            if fields.len() != expected.len() {
                return Err(error(line, "E_TYPE", "constructor field count differs"));
            }
            for i in 0..fields.len() {
                if ty(fields[i])? != expected[i].ty {
                    return Err(error(line, "E_TYPE", "constructor field type differs"));
                }
            }
        }
        Op::Discriminant { dst, src } | Op::TagTest { dst, src, .. } => {
            let st = ty(*src)?;
            let inner = match types[st].kind {
                TypeKind::Ref { inner, .. } => inner,
                _ => st,
            };
            let variants = match &types[inner].kind {
                TypeKind::Enum { variants, .. } => variants,
                _ => return Err(error(line, "E_TYPE", "tag read requires an enum")),
            };
            match instruction.op {
                Op::TagTest { variant, .. } => {
                    if ty(*dst)? != BOOL || variant >= variants.len() {
                        return Err(error(line, "E_TYPE", "invalid enum tag test"));
                    }
                }
                _ => {
                    if ty(*dst)? != U64 || inner != st {
                        return Err(error(line, "E_TYPE", "invalid enum tag read"));
                    }
                }
            }
        }
        Op::Intrinsic { dst, name, args } => {
            check_intrinsic(types, function, *dst, name, args, line)?
        }
        Op::If { condition } => {
            if ty(*condition)? != BOOL {
                return Err(error(line, "E_TYPE", "if requires bool"));
            }
        }
        Op::Return { src } => {
            if ty(*src)? != function.result {
                return Err(error(line, "E_TYPE", "returned type differs"));
            }
        }
        Op::Else | Op::End | Op::Loop | Op::Break | Op::Continue | Op::Trap { .. } => {}
    }
    Ok(())
}

fn call_borrows(
    state: &mut State,
    types: &[Type],
    function: &Function,
    args: &[usize],
    line: usize,
) -> Result<(), Diagnostic> {
    for arg in args {
        if let TypeKind::Ref { mutable, .. } = types[function.locals[*arg].ty].kind {
            let place = Place {
                local: *arg,
                projections: vec![Projection::Deref],
            };
            access(state, types, function, &place, mutable, 0, line)?;
        }
    }
    for arg in args {
        if matches!(
            types[function.locals[*arg].ty].kind,
            TypeKind::Ref { mutable: true, .. }
        ) {
            let target = origin(
                state,
                &Place {
                    local: *arg,
                    projections: vec![Projection::Deref],
                },
            )
            .0;
            invalidate(state, &target);
        }
    }
    Ok(())
}

fn check_function(program: &Program, function: &Function) -> Result<(), Diagnostic> {
    let types = &program.types;
    let mut state = State {
        live: vec![false; function.locals.len()],
        moved: Vec::new(),
        loans: Vec::new(),
        tags: Vec::new(),
        predicates: vec![None; function.locals.len()],
        reachable: true,
    };
    for i in 0..function.params {
        state.live[i] = true;
        if let TypeKind::Ref { mutable, .. } = types[function.locals[i].ty].kind {
            state.loans.push(Loan {
                local: i,
                target: Origin {
                    root: Root::Parameter(i),
                    projections: Vec::new(),
                },
                mutable,
                parents: Vec::new(),
            });
        }
    }
    let mut controls: Vec<Control> = Vec::new();
    for instruction in &function.body {
        let line = instruction.line;
        check_shape(program, function, instruction)?;
        match &instruction.op {
            Op::Else => {
                let control = controls
                    .last_mut()
                    .ok_or_else(|| error(line, "E_K", "else outside if"))?;
                if control.is_loop || control.then_state.is_some() {
                    return Err(error(line, "E_K", "invalid else"));
                }
                control.then_state = Some(state.clone());
                state = control.entry.clone();
                continue;
            }
            Op::End => {
                let control = controls
                    .pop()
                    .ok_or_else(|| error(line, "E_K", "end outside control"))?;
                if control.is_loop {
                    if state.reachable {
                        let _ = merge(control.entry.clone(), state.clone(), line)?;
                    }
                    state.reachable = false;
                    for exit in control.breaks {
                        state = merge(state, exit, line)?;
                    }
                } else {
                    state = merge(state, control.then_state.unwrap_or(control.entry), line)?;
                }
                continue;
            }
            Op::If { condition } => {
                let predicate = state.predicates[*condition].clone();
                if state.reachable {
                    consume(&mut state, types, function, *condition, line)?;
                }
                controls.push(Control {
                    entry: state.clone(),
                    then_state: None,
                    is_loop: false,
                    breaks: Vec::new(),
                });
                if let Some(tag) = predicate {
                    set_tag(&mut state, tag.target, tag.variant);
                }
                continue;
            }
            Op::Loop => {
                controls.push(Control {
                    entry: state.clone(),
                    then_state: None,
                    is_loop: true,
                    breaks: Vec::new(),
                });
                continue;
            }
            Op::Break | Op::Continue => {
                if !controls.iter().any(|control| control.is_loop) {
                    return Err(error(line, "E_K", "loop transfer outside loop"));
                }
            }
            _ => {}
        }
        if !state.reachable {
            continue;
        }
        match &instruction.op {
            Op::Int { dst, .. } | Op::Float { dst, .. } | Op::Unit { dst } => {
                put(&mut state, *dst, line)?
            }
            Op::Read { dst, place, moving } => {
                access(&state, types, function, place, *moving, 0, line)?;
                let target = origin(&state, place).0;
                let known_tag = tag_at(&state, &target);
                let predicate = if place.projections.is_empty() {
                    state.predicates[place.local].clone()
                } else {
                    None
                };
                let loan = if matches!(types[function.locals[*dst].ty].kind, TypeKind::Ref { .. }) {
                    let mut deref = place.clone();
                    deref.projections.push(Projection::Deref);
                    let (target, parents) = origin(&state, &deref);
                    Some(Loan {
                        local: *dst,
                        target,
                        parents,
                        mutable: matches!(
                            types[function.locals[*dst].ty].kind,
                            TypeKind::Ref { mutable: true, .. }
                        ),
                    })
                } else {
                    None
                };
                if *moving {
                    if place.projections.is_empty() {
                        consume(&mut state, types, function, place.local, line)?;
                    } else {
                        invalidate(&mut state, &target);
                        state.moved.push(place.clone());
                    }
                }
                put(&mut state, *dst, line)?;
                state.predicates[*dst] = predicate;
                if let Some(loan) = loan {
                    state.loans.push(loan);
                }
                if let Some(variant) = known_tag {
                    set_tag(&mut state, Origin::local(*dst), variant);
                }
            }
            Op::Borrow {
                dst,
                place,
                mutable,
            } => {
                access(&state, types, function, place, *mutable, 0, line)?;
                let (target, parents) = origin(&state, place);
                put(&mut state, *dst, line)?;
                state.loans.push(Loan {
                    local: *dst,
                    target,
                    mutable: *mutable,
                    parents,
                });
            }
            Op::Assign { place, src } => {
                if !place.projections.is_empty() || state.live[place.local] {
                    access(&state, types, function, place, true, 1, line)?;
                }
                if state.live[place.local]
                    && place.projections.is_empty()
                    && !function.locals[place.local].mutable
                {
                    return Err(error(line, "E_TYPE", "assignment to an immutable local"));
                }
                let target = origin(&state, place).0;
                let absent = if place.projections.is_empty() {
                    !state.live[place.local]
                } else {
                    state.moved.iter().any(|moved| moved == place)
                };
                let pt = place_type(types, function, place).unwrap();
                if !absent && heap_type(&types[pt].kind) {
                    return Err(error(
                        line,
                        "E_K",
                        "replacement of an owned value needs an explicit drop",
                    ));
                }
                let inherited = state.loans.iter().find(|loan| loan.local == *src).cloned();
                let known_tag = tag_at(&state, &Origin::local(*src));
                let predicate = state.predicates[*src].clone();
                consume(&mut state, types, function, *src, line)?;
                invalidate(&mut state, &target);
                state.live[place.local] = true;
                state.moved.retain(|moved| {
                    !(moved.local == place.local
                        && is_prefix(&place.projections, &moved.projections))
                });
                if place.projections.is_empty() {
                    state.loans.retain(|old| old.local != place.local);
                    if let Some(mut loan) = inherited {
                        loan.local = place.local;
                        state.loans.push(loan);
                    }
                    state.predicates[place.local] = predicate;
                }
                if let Some(variant) = known_tag {
                    set_tag(&mut state, target, variant);
                }
            }
            Op::Drop { place } => {
                if !state.live[place.local] {
                    return Err(error(
                        line,
                        "E_MOVE",
                        "drop of an uninitialized or moved local",
                    ));
                }
                access(&state, types, function, place, true, 1, line)?;
                if place.projections.is_empty() {
                    // Dropping an aggregate destroys only fields that remain after partial moves.
                    state.moved.retain(|moved| moved.local != place.local);
                    consume(&mut state, types, function, place.local, line)?;
                } else {
                    let target = origin(&state, place).0;
                    invalidate(&mut state, &target);
                    state.moved.retain(|moved| {
                        !(moved.local == place.local
                            && is_prefix(&place.projections, &moved.projections))
                    });
                    state.moved.push(place.clone());
                }
            }
            Op::Unary { dst, src, .. } => {
                consume(&mut state, types, function, *src, line)?;
                put(&mut state, *dst, line)?;
            }
            Op::Binary {
                dst, left, right, ..
            } => {
                consume(&mut state, types, function, *left, line)?;
                consume(&mut state, types, function, *right, line)?;
                put(&mut state, *dst, line)?;
            }
            Op::Call {
                dst,
                function: called,
                args,
            } => {
                let target = &program.functions[*called];
                let mut returned = None;
                if matches!(types[target.result].kind, TypeKind::Ref { .. }) {
                    let mut count = 0;
                    for i in 0..target.params {
                        if matches!(types[target.locals[i].ty].kind, TypeKind::Ref { .. }) {
                            count += 1;
                            returned = state
                                .loans
                                .iter()
                                .find(|loan| loan.local == args[i])
                                .cloned();
                        }
                    }
                    if count != 1 || returned.is_none() {
                        return Err(error(
                            line,
                            "E_BORROW",
                            "returned view needs one source parameter",
                        ));
                    }
                }
                call_borrows(&mut state, types, function, args, line)?;
                for arg in args {
                    consume(&mut state, types, function, *arg, line)?;
                }
                put(&mut state, *dst, line)?;
                if let Some(mut loan) = returned {
                    loan.local = *dst;
                    state.loans.push(loan);
                }
            }
            Op::Struct { dst, fields } | Op::Variant { dst, fields, .. } => {
                for field in fields {
                    consume(&mut state, types, function, *field, line)?;
                }
                put(&mut state, *dst, line)?;
                if let Op::Variant { variant, .. } = instruction.op {
                    set_tag(&mut state, Origin::local(*dst), variant);
                }
            }
            Op::Discriminant { dst, src } | Op::TagTest { dst, src, .. } => {
                let place = enum_place(types, function, *src);
                access(&state, types, function, &place, false, 0, line)?;
                let target = origin(&state, &place).0;
                put(&mut state, *dst, line)?;
                if let Op::TagTest { variant, .. } = instruction.op {
                    state.predicates[*dst] = Some(Tag { target, variant });
                }
            }
            Op::Intrinsic { dst, args, .. } => {
                call_borrows(&mut state, types, function, args, line)?;
                for arg in args {
                    consume(&mut state, types, function, *arg, line)?;
                }
                put(&mut state, *dst, line)?;
            }
            Op::Return { src } => {
                if matches!(types[function.result].kind, TypeKind::Ref { .. }) {
                    let loan = state
                        .loans
                        .iter()
                        .find(|loan| loan.local == *src)
                        .ok_or_else(|| {
                            error(line, "E_BORROW", "returned reference has no source")
                        })?;
                    if !matches!(loan.target.root, Root::Parameter(_)) {
                        return Err(error(
                            line,
                            "E_BORROW",
                            "returned view does not project from a borrowed parameter",
                        ));
                    }
                }
                consume(&mut state, types, function, *src, line)?;
                if state.live.iter().any(|live| *live) {
                    return Err(error(
                        line,
                        "E_K",
                        "return leaves a value without an explicit drop",
                    ));
                }
                state.reachable = false;
            }
            Op::Break | Op::Continue => {
                let control = controls
                    .iter_mut()
                    .rev()
                    .find(|control| control.is_loop)
                    .unwrap();
                if matches!(instruction.op, Op::Continue) {
                    let _ = merge(control.entry.clone(), state.clone(), line)?;
                } else {
                    control.breaks.push(state.clone());
                }
                state.reachable = false;
            }
            Op::Trap { .. } => state.reachable = false,
            Op::If { .. } | Op::Else | Op::End | Op::Loop => {}
        }
    }
    if !controls.is_empty() {
        return Err(error(1, "E_K", "unclosed control block"));
    }
    if state.reachable {
        return Err(error(1, "E_K", "function reaches end without return"));
    }
    Ok(())
}

fn direct_children(kind: &TypeKind) -> Vec<usize> {
    let mut children = Vec::new();
    match kind {
        TypeKind::Struct { fields, .. } => {
            for field in fields {
                children.push(field.ty);
            }
        }
        TypeKind::Enum { variants, .. } => {
            for variant in variants {
                for field in &variant.fields {
                    children.push(field.ty);
                }
            }
        }
        _ => {}
    }
    children
}

fn contains_view(types: &[Type], initial: usize) -> bool {
    let mut seen = vec![false; types.len()];
    let mut pending = vec![initial];
    while let Some(ty) = pending.pop() {
        if seen[ty] {
            continue;
        }
        seen[ty] = true;
        match &types[ty].kind {
            TypeKind::Ref { .. } | TypeKind::Slice { .. } => return true,
            TypeKind::Box { inner } | TypeKind::Vec { inner } => pending.push(*inner),
            kind => pending.extend(direct_children(kind)),
        }
    }
    false
}

fn check_types(types: &[Type]) -> Result<(), Diagnostic> {
    let expected = primitives();
    if types.len() < expected.len() {
        return Err(error(1, "E_K", "primitive types are missing"));
    }
    for i in 0..expected.len() {
        if types[i] != expected[i] {
            return Err(error(1, "E_K", "primitive type definition differs"));
        }
    }
    for ty in types {
        let mut children = direct_children(&ty.kind);
        match &ty.kind {
            TypeKind::Int { bits, .. } => {
                if ![8, 16, 32, 64].contains(bits) {
                    return Err(error(1, "E_K", "unsupported integer representation"));
                }
            }
            TypeKind::Ref { inner, .. }
            | TypeKind::Slice { inner, .. }
            | TypeKind::Vec { inner }
            | TypeKind::Box { inner } => children.push(*inner),
            _ => {}
        }
        for child in children {
            if child >= types.len() {
                return Err(error(1, "E_K", "unknown type reference"));
            }
        }
    }
    let mut colors = vec![0u8; types.len()];
    for start in 0..types.len() {
        if colors[start] != 0 {
            continue;
        }
        let mut pending = vec![(start, false)];
        while let Some((ty, leaving)) = pending.pop() {
            if leaving {
                colors[ty] = 2;
                continue;
            }
            if colors[ty] == 2 {
                continue;
            }
            if colors[ty] == 1 {
                return Err(error(
                    1,
                    "E_TYPE",
                    "recursive value type needs Box or another indirection",
                ));
            }
            colors[ty] = 1;
            pending.push((ty, true));
            for child in direct_children(&types[ty].kind) {
                pending.push((child, false));
            }
        }
    }
    for i in 0..types.len() {
        if heap_type(&types[i].kind) && contains_view(types, i) {
            return Err(error(
                1,
                "E_UNSUPPORTED",
                "references inside owning aggregates need view provenance support",
            ));
        }
        if matches!(
            types[i].kind,
            TypeKind::Struct { copy: true, .. } | TypeKind::Enum { copy: true, .. }
        ) && !is_copy(types, i)
        {
            return Err(error(1, "E_TYPE", "Copy type contains an affine field"));
        }
    }
    Ok(())
}

pub fn check(program: Program) -> Result<CheckedProgram, Diagnostic> {
    check_types(&program.types)?;
    // Check all signatures before any call instruction can index a callee.
    for function in &program.functions {
        if function.params > function.locals.len() || function.result >= program.types.len() {
            return Err(error(1, "E_K", "invalid function signature"));
        }
        for local in &function.locals {
            if local.ty >= program.types.len() {
                return Err(error(1, "E_K", "unknown local type"));
            }
            if matches!(program.types[local.ty].kind, TypeKind::Slice { .. }) {
                return Err(error(
                    1,
                    "E_UNSUPPORTED",
                    "bare slice values need view provenance support",
                ));
            }
        }
        if function.test && (function.params != 0 || function.result != UNIT) {
            return Err(error(
                1,
                "E_TYPE",
                "test function must take no arguments and return unit",
            ));
        }
        if matches!(program.types[function.result].kind, TypeKind::Ref { .. }) {
            let borrowed = function.locals[..function.params]
                .iter()
                .filter(|local| matches!(program.types[local.ty].kind, TypeKind::Ref { .. }))
                .count();
            if borrowed != 1 {
                return Err(error(
                    1,
                    "E_BORROW",
                    "returned view needs one source parameter",
                ));
            }
        }
    }
    for function in &program.functions {
        check_function(&program, function)?;
    }
    Ok(CheckedProgram { program })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(ty: usize, mutable: bool) -> Local {
        Local {
            name: String::new(),
            ty,
            mutable,
        }
    }
    fn op(op: Op) -> Instruction {
        Instruction { op, line: 1 }
    }
    fn program(
        types: Vec<Type>,
        locals: Vec<Local>,
        params: usize,
        result: usize,
        ops: Vec<Op>,
    ) -> Program {
        Program {
            types,
            functions: vec![Function {
                name: "probe".to_string(),
                params,
                result,
                locals,
                body: ops.into_iter().map(op).collect(),
                test: false,
            }],
        }
    }
    fn rejection(p: Program, code: &str) {
        let result = check(p);
        assert!(result.is_err(), "malformed K was accepted");
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

    #[test]
    fn accepts_scalar() {
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

    #[test]
    fn rejects_forged_scalar() {
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

    #[test]
    fn rejects_bad_operand_in_dead_code() {
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

    #[test]
    fn rejects_bad_later_signature_without_panicking() {
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
            name: "bad".to_string(),
            params: 1,
            result: I64,
            locals: Vec::new(),
            body: Vec::new(),
            test: false,
        });
        rejection(p, "E_K");
    }

    #[test]
    fn consuming_a_borrowed_owner_is_rejected() {
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
            name: "take".to_string(),
            params: 1,
            result: I64,
            locals: vec![local(I64, false)],
            body: vec![op(Op::Return { src: 0 })],
            test: false,
        });
        rejection(p, "E_BORROW");
    }

    #[test]
    fn immutable_reference_cannot_become_mutable_through_a_mut_binding() {
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

    #[test]
    fn returned_parameter_view_preserves_external_origin() {
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

    #[test]
    fn returning_address_of_parameter_slot_is_rejected() {
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

    #[test]
    fn reference_aggregate_requires_provenance_support() {
        let mut types = with_ref(false);
        types.push(Type {
            name: "View".to_string(),
            kind: TypeKind::Struct {
                fields: vec![Field {
                    name: "value".to_string(),
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
            "E_UNSUPPORTED",
        );
    }

    #[test]
    fn inactive_variant_cannot_forge_a_capability() {
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

    #[test]
    fn unknown_variant_field_needs_tag_branch() {
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

    #[test]
    fn missing_explicit_cleanup_is_rejected() {
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
            "E_K",
        );
    }

    #[test]
    fn by_value_recursive_types_are_rejected() {
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

    #[test]
    fn bool_cannot_cast_directly_to_float() {
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
                        name: "cast".to_string(),
                        args: vec![0],
                    },
                    Op::Return { src: 1 },
                ],
            ),
            "E_TYPE",
        );
    }
    #[test]
    fn partially_moved_aggregate_cannot_be_consumed_whole() {
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

    #[test]
    fn field_assignment_cannot_resurrect_a_moved_owner() {
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

    #[test]
    fn indexing_requires_an_initialized_index() {
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

    #[test]
    fn checked_tag_branch_allows_a_variant_field() {
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
                        src: 0,
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

    #[test]
    fn shared_reborrow_of_mutable_parameter_is_usable() {
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

    #[test]
    fn mutable_parent_cannot_be_called_while_reborrow_is_live() {
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

    #[test]
    fn moved_local_cannot_be_dropped_twice() {
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
}
