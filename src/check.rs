//! The single gate for explicit K. Each structured edge has an ownership state.
use crate::ast::{Diagnostic, Span};
use crate::k::*;
use std::collections::{BTreeMap, BTreeSet};

fn error(line: usize, code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(
        Span {
            file: 0,
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
            TypeKind::Array { inner, .. } => pending.push(*inner),
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
            | TypeKind::Box { inner }
            | TypeKind::Array { inner, .. } => pending.push(*inner),
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
            (
                Projection::Index(index),
                TypeKind::Vec { inner }
                | TypeKind::Slice { inner, .. }
                | TypeKind::Array { inner, .. },
            ) => {
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
            | TypeKind::Array { .. }
            | TypeKind::Box { .. }
    )
}

fn path_overlaps(a: &[Projection], b: &[Projection]) -> bool {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            // Enum field access separately requires a checked tag. Distinct
            // fields of that same variant are disjoint; different variants
            // may reuse their payload storage.
            return match (&a[i], &b[i]) {
                (Projection::Field(_), Projection::Field(_)) => false,
                (Projection::VariantField(left, _), Projection::VariantField(right, _))
                    if left == right =>
                {
                    false
                }
                _ => true,
            };
        }
    }
    true
}

fn is_prefix(a: &[Projection], b: &[Projection]) -> bool {
    a.len() <= b.len() && a == &b[..a.len()]
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Root {
    /// Immutable storage that lives for the entire module instance.
    Static,
    Local(usize),
    Parameter(usize),
}

#[derive(Clone, PartialEq, Eq)]
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

#[derive(Clone, PartialEq, Eq)]
struct Loan {
    holder: Place,
    target: Origin,
    mutable: bool,
    parents: Vec<Place>,
}
#[derive(Clone, PartialEq, Eq)]
struct Tag {
    target: Origin,
    witness: Place,
    variant: usize,
}
#[derive(Clone, PartialEq)]
struct State {
    live: Vec<bool>,
    moved: Vec<Place>,
    loans: Vec<Loan>,
    absent_references: Vec<Place>,
    tags: Vec<Tag>,
    predicates: BTreeMap<usize, Tag>,
    reachable: bool,
}
#[derive(Clone)]
struct Control {
    entry: State,
    then_state: Option<State>,
    is_loop: bool,
    breaks: Vec<State>,
}

fn within(place: &Place, base: &Place) -> bool {
    place.local == base.local && is_prefix(&base.projections, &place.projections)
}
fn rebase(place: &Place, from: &Place, to: &Place) -> Place {
    if !within(place, from) {
        return place.clone();
    }
    let mut next = to.clone();
    next.projections
        .extend_from_slice(&place.projections[from.projections.len()..]);
    next
}
fn holder_origin(place: &Place) -> Origin {
    Origin {
        root: Root::Local(place.local),
        projections: place.projections.clone(),
    }
}
fn origin(state: &State, place: &Place) -> (Origin, Vec<Place>) {
    let mut result = holder_origin(place);
    let mut parents = Vec::new();
    loop {
        let local = match result.root {
            Root::Local(local) => local,
            Root::Parameter(_) | Root::Static => return (result, parents),
        };
        let mut found = None;
        for i in 0..result.projections.len() {
            if !matches!(result.projections[i], Projection::Deref) {
                continue;
            }
            let holder = Place {
                local,
                projections: result.projections[..i].to_vec(),
            };
            if parents.contains(&holder) {
                continue;
            }
            if let Some(loan) = state.loans.iter().find(|loan| loan.holder == holder) {
                found = Some((i, loan.clone()));
                break;
            }
        }
        match found {
            Some((at, loan)) => {
                parents.push(loan.holder);
                for parent in loan.parents {
                    if !parents.contains(&parent) {
                        parents.push(parent);
                    }
                }
                let mut next = loan.target;
                next.projections
                    .extend_from_slice(&result.projections[at + 1..]);
                result = next;
            }
            None => return (result, parents),
        }
    }
}
fn affects_tag(target: &Origin, tag: &Tag) -> bool {
    // Changing payload storage leaves its enclosing discriminant intact. Both
    // the enum target and the reference carrier witness are invalidated only by
    // an operation on that storage or one of its ancestors.
    (target.projections.len() <= tag.target.projections.len() && overlaps(target, &tag.target))
        || (target.projections.len() <= tag.witness.projections.len()
            && overlaps(target, &holder_origin(&tag.witness)))
}
fn invalidate(state: &mut State, target: &Origin) {
    state.tags.retain(|tag| !affects_tag(target, tag));
    state.predicates.retain(|_, tag| !affects_tag(target, tag));
}
fn tag_at(state: &State, target: &Origin, witness: &Place) -> Option<usize> {
    state
        .tags
        .iter()
        .find(|tag| tag.target == *target && tag.witness == *witness)
        .map(|tag| tag.variant)
}
fn set_tag(state: &mut State, tag: Tag) {
    state.tags.retain(|old| old.witness != tag.witness);
    state.tags.push(tag);
}

/// Reference leaves of a view. Owned heap containers cannot contain views.
fn reference_paths(types: &[Type], ty: usize) -> Vec<(Vec<Projection>, bool)> {
    let mut result = Vec::new();
    let mut pending = vec![(ty, Vec::new())];
    while let Some((ty, path)) = pending.pop() {
        match &types[ty].kind {
            TypeKind::Ref { mutable, .. } => result.push((path, *mutable)),
            TypeKind::Struct { fields, .. } => {
                for (i, field) in fields.iter().enumerate() {
                    let mut child = path.clone();
                    child.push(Projection::Field(i));
                    pending.push((field.ty, child));
                }
            }
            TypeKind::Enum { variants, .. } => {
                for (v, variant) in variants.iter().enumerate() {
                    for (i, field) in variant.fields.iter().enumerate() {
                        let mut child = path.clone();
                        child.push(Projection::VariantField(v, i));
                        pending.push((field.ty, child));
                    }
                }
            }
            _ => {}
        }
    }
    result
}
pub fn is_view(types: &[Type], ty: usize) -> bool {
    !reference_paths(types, ty).is_empty()
}
pub(crate) fn shared_view(types: &[Type], ty: usize) -> bool {
    let paths = reference_paths(types, ty);
    !paths.is_empty() && paths.iter().all(|(_, mutable)| !mutable)
}

/// A projected source selects stored shared references, not the temporary
/// descriptor used to borrow them. Validate every step before allocating it.
fn parameter_source(
    types: &[Type],
    function: &Function,
    index: usize,
    projections: &[Projection],
) -> Option<Place> {
    if index >= function.params {
        return None;
    }
    let mut ty = function.locals.get(index)?.ty;
    for projection in projections {
        if !matches!(projection, Projection::Deref) {
            return None;
        }
        match types.get(ty)?.kind {
            TypeKind::Ref { inner, .. } => ty = inner,
            _ => return None,
        }
    }
    if !is_view(types, ty) || (!projections.is_empty() && !shared_view(types, ty)) {
        return None;
    }
    Some(Place {
        local: index,
        projections: projections.to_vec(),
    })
}
fn exclusive_variants(a: &Place, b: &Place) -> bool {
    if a.local != b.local {
        return false;
    }
    for i in 0..a.projections.len().min(b.projections.len()) {
        if a.projections[i] == b.projections[i] {
            continue;
        }
        return matches!((&a.projections[i], &b.projections[i]),
            (Projection::VariantField(a, _), Projection::VariantField(b, _)) if a != b);
    }
    false
}
fn common_origin(loans: &[Loan], line: usize) -> Result<Option<(Origin, Vec<Place>)>, Diagnostic> {
    // Static leaves can share any shorter dynamic lifetime. Keep every leaf in
    // the provenance inventory while computing the dynamic root of a view.
    let first = loans
        .iter()
        .find(|loan| loan.target.root != Root::Static)
        .or_else(|| loans.first());
    let Some(first) = first else {
        return Ok(None);
    };
    let mut common = first.target.clone();
    let mut parents = first.parents.clone();
    for loan in loans {
        if loan.target.root == Root::Static {
            if loan.mutable {
                return Err(error(line, "E_BORROW", "static storage is immutable"));
            }
            continue;
        }
        if loan.target.root != common.root {
            return Err(error(
                line,
                "E_BORROW",
                "view must have one source lifetime",
            ));
        }
        let mut count = 0;
        while count < common.projections.len()
            && count < loan.target.projections.len()
            && common.projections[count] == loan.target.projections[count]
        {
            count += 1;
        }
        common.projections.truncate(count);
        for parent in &loan.parents {
            if !parents.contains(parent) {
                parents.push(parent.clone());
            }
        }
    }
    Ok(Some((common, parents)))
}
fn validate_view(loans: &[Loan], line: usize) -> Result<(), Diagnostic> {
    let _ = common_origin(loans, line)?;
    for (i, a) in loans.iter().enumerate() {
        for b in &loans[i + 1..] {
            if !exclusive_variants(&a.holder, &b.holder)
                && overlaps(&a.target, &b.target)
                && (a.mutable || b.mutable)
            {
                return Err(error(
                    line,
                    "E_BORROW",
                    "view contains aliasing mutable references",
                ));
            }
        }
    }
    Ok(())
}
/// Complete provenance of a value, including reference leaves in inactive
/// enum alternatives. Absence is evidence, not a missing loan entry.
struct Provenance {
    loans: Vec<Loan>,
    absent: Vec<Place>,
}
fn carried_provenance(
    state: &State,
    types: &[Type],
    function: &Function,
    from: &Place,
    to: &Place,
    line: usize,
) -> Result<Provenance, Diagnostic> {
    let value_ty = place_type(types, function, from).unwrap();
    let paths = reference_paths(types, value_ty);
    let mut result = Provenance {
        loans: Vec::new(),
        absent: Vec::new(),
    };
    if paths.is_empty() {
        return Ok(result);
    }
    let target = origin(state, from).0;
    for (path, mutable) in paths {
        let mut holder = to.clone();
        holder.projections.extend_from_slice(&path);
        if let Root::Local(local) = target.root {
            let mut source = Place {
                local,
                projections: target.projections.clone(),
            };
            source.projections.extend_from_slice(&path);
            if let Some(loan) = state.loans.iter().find(|loan| loan.holder == source) {
                let mut next = loan.clone();
                next.holder = holder;
                result.loans.push(next);
            } else if path
                .iter()
                .any(|p| matches!(p, Projection::VariantField(_, _)))
                && state.absent_references.contains(&source)
            {
                result.absent.push(holder);
            } else {
                return Err(error(
                    line,
                    "E_BORROW",
                    "reference-bearing value has incomplete source provenance",
                ));
            }
        } else {
            let mut source = from.clone();
            source.projections.extend_from_slice(&path);
            source.projections.push(Projection::Deref);
            let (target, parents) = origin(state, &source);
            result.loans.push(Loan {
                holder,
                target,
                mutable,
                parents,
            });
        }
    }
    Ok(result)
}
fn remove_provenance(state: &mut State, place: &Place) {
    state.loans.retain(|loan| !within(&loan.holder, place));
    state
        .absent_references
        .retain(|absent| !within(absent, place));
}
fn install_provenance(state: &mut State, value: Provenance) {
    state.loans.extend(value.loans);
    state.absent_references.extend(value.absent);
}
fn storage_place(target: &Origin, fallback: &Place) -> Place {
    match target.root {
        Root::Local(local) => Place {
            local,
            projections: target.projections.clone(),
        },
        Root::Parameter(_) | Root::Static => fallback.clone(),
    }
}
/// K does not yet declare cross-parameter write effects. A write into external
/// view storage may clear leaves or retain their exact existing origins, but
/// cannot silently change the caller's lifetime/projection contract.
fn check_external_write(
    target: &Origin,
    destination: &Place,
    value: &Provenance,
    line: usize,
) -> Result<(), Diagnostic> {
    if !matches!(target.root, Root::Parameter(_)) {
        return Ok(());
    }
    for loan in &value.loans {
        let mut expected = target.clone();
        expected
            .projections
            .extend_from_slice(&loan.holder.projections[destination.projections.len()..]);
        expected.projections.push(Projection::Deref);
        if loan.target != expected {
            return Err(error(
                line,
                "E_BORROW",
                "write changes external view provenance without a declared mutation contract",
            ));
        }
    }
    Ok(())
}
fn validate_stored_view(state: &State, target: &Origin, line: usize) -> Result<(), Diagnostic> {
    if let Root::Local(local) = target.root {
        let loans: Vec<Loan> = state
            .loans
            .iter()
            .filter(|loan| loan.holder.local == local)
            .cloned()
            .collect();
        validate_view(&loans, line)?;
    }
    Ok(())
}
fn remap_parents(state: &mut State, from: &Place, to: &Place) {
    for loan in &mut state.loans {
        for parent in &mut loan.parents {
            *parent = rebase(parent, from, to);
        }
    }
}
fn remap_carried(loans: &mut [Loan], from: &Place, to: &Place) {
    for loan in loans {
        for parent in &mut loan.parents {
            *parent = rebase(parent, from, to);
        }
    }
}
fn place_writable(types: &[Type], function: &Function, place: &Place) -> bool {
    let mut writable = function.locals[place.local].mutable;
    let mut blocked = false;
    let mut ty = function.locals[place.local].ty;
    let mut prefix = Place::local(place.local);
    for projection in &place.projections {
        if let (Projection::Deref, TypeKind::Ref { mutable, .. }) = (projection, &types[ty].kind) {
            if !mutable {
                blocked = true;
            }
            writable = *mutable && !blocked;
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
fn loan_use(state: &State, loan: &Loan, mutable: bool, line: usize) -> Result<(), Diagnostic> {
    for other in &state.loans {
        if other.holder == loan.holder
            || loan.parents.contains(&other.holder)
            || exclusive_variants(&loan.holder, &other.holder)
        {
            continue;
        }
        if overlaps(&loan.target, &other.target) && (mutable || other.mutable) {
            return Err(error(
                line,
                "E_BORROW",
                "reference conflicts with a live borrow",
            ));
        }
    }
    Ok(())
}

// mode 0 reads a whole value; mode 1 replaces/drops its remaining fields.
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
        if matches!(projection, Projection::Deref)
            && matches!(
                types[place_type(types, function, &prefix).unwrap()].kind,
                TypeKind::Ref { .. }
            )
        {
            let _ = carried_provenance(state, types, function, &prefix, &prefix, line)?;
        }
        if let Projection::Index(index) = projection {
            if !state.live[*index] {
                return Err(error(line, "E_MOVE", "index is uninitialized or moved"));
            }
        }
        if let Projection::VariantField(variant, _) = projection {
            if tag_at(state, &origin(state, &prefix).0, &prefix) != Some(*variant) {
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
    if write && target.root == Root::Static {
        return Err(error(line, "E_BORROW", "static storage is immutable"));
    }
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
    let value_ty = place_type(types, function, place).unwrap();
    let view_owner =
        is_view(types, value_ty) && !matches!(types[value_ty].kind, TypeKind::Ref { .. });
    for loan in &state.loans {
        if loan.holder == *place || parents.contains(&loan.holder) {
            continue;
        }
        if overlaps(&loan.target, &target) && (write || loan.mutable) {
            return Err(error(
                line,
                "E_BORROW",
                "place conflicts with a live borrow",
            ));
        }
        // Using a reference field also uses the storage containing that field.
        for parent in &parents {
            if overlaps(&loan.target, &holder_origin(parent)) && (write || loan.mutable) {
                return Err(error(
                    line,
                    "E_BORROW",
                    "reference field is inside borrowed storage",
                ));
            }
        }
        // A child reborrow freezes a view that can use or drop its parent field.
        if write && view_owner && !within(&loan.holder, place) {
            for parent in &loan.parents {
                if overlaps(&holder_origin(parent), &target) {
                    return Err(error(
                        line,
                        "E_BORROW",
                        "view is still borrowed through one of its fields",
                    ));
                }
            }
        }
    }
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
    remove_provenance(state, &Place::local(local));
    state.moved.retain(|place| place.local != local);
    invalidate(state, &Origin::local(local));
    state.predicates.remove(&local);
    Ok(())
}
fn put(state: &mut State, local: usize, line: usize) -> Result<(), Diagnostic> {
    if state.live[local] {
        return Err(error(line, "E_K", "destination still owns a value"));
    }
    state.live[local] = true;
    state.predicates.remove(&local);
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
    if a.live != b.live
        || !same_set(&a.moved, &b.moved)
        || !same_set(&a.loans, &b.loans)
        || !same_set(&a.absent_references, &b.absent_references)
    {
        return Err(error(
            line,
            "E_MOVE",
            "branches must leave the same owners and borrows",
        ));
    }
    a.tags.retain(|tag| b.tags.contains(tag));
    a.predicates
        .retain(|local, tag| b.predicates.get(local) == Some(tag));
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
fn has_destructor(program: &Program, ty: usize) -> bool {
    program.destructors.iter().any(|entry| entry.0 == ty)
}
fn crosses_destructor(program: &Program, function: &Function, place: &Place) -> bool {
    let mut prefix = Place::local(place.local);
    for projection in &place.projections {
        if has_destructor(
            program,
            place_type(&program.types, function, &prefix).unwrap(),
        ) {
            return true;
        }
        prefix.projections.push(projection.clone());
    }
    false
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
    let text_ref = |t: usize| {
        matches!(types[t].kind,
        TypeKind::Ref { inner, mutable: false } if matches!(types[inner].kind, TypeKind::Str))
    };
    let bytes_ref = |t: usize| {
        matches!(types[t].kind,
        TypeKind::Ref { inner, mutable: false } if matches!(types[inner].kind,
            TypeKind::Slice { inner: U8, mutable: false }))
    };
    let valid = match name {
        "str_len" => ts.len() == 1 && text_ref(ts[0]) && dt == U64,
        "str_as_bytes" => ts.len() == 1 && text_ref(ts[0]) && bytes_ref(dt),
        "str_from_utf8" => {
            ts.len() == 1
                && bytes_ref(ts[0])
                && match &types[dt].kind {
                    TypeKind::Enum { variants, .. }
                        if variants.len() == 2
                            && variants[0].fields.len() == 1
                            && variants[1].fields.len() == 1 =>
                    {
                        text_ref(variants[0].fields[0].ty)
                            && matches!(&types[variants[1].fields[0].ty].kind,
                        TypeKind::Struct { fields, copy: true } if fields.is_empty())
                    }
                    _ => false,
                }
        }
        "assert" => ts == vec![BOOL] && dt == UNIT,
        "vec_new" => ts.is_empty() && matches!(types[dt].kind, TypeKind::Vec { .. }),
        "vec_len" => {
            ts.len() == 1
                && dt == U64
                && matches!(types[ts[0]].kind, TypeKind::Ref { .. })
                && matches!(
                    types[inner(ts[0])].kind,
                    TypeKind::Array { .. } | TypeKind::Vec { .. } | TypeKind::Slice { .. }
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
        Op::Text { dst, bytes } => {
            if !matches!(types[ty(*dst)?].kind, TypeKind::Ref { inner, mutable: false }
                if matches!(types[inner].kind, TypeKind::Str))
            {
                return Err(error(
                    line,
                    "E_TYPE",
                    "text literal requires an immutable text reference",
                ));
            }
            if std::str::from_utf8(bytes).is_err() {
                return Err(error(line, "E_TYPE", "text literal contains invalid UTF-8"));
            }
        }
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
            if *moving && crosses_destructor(program, function, place) {
                return Err(error(
                    line,
                    "E_MOVE",
                    "cannot partially move a type with custom Drop",
                ));
            }
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
        Op::Array { dst, elements } => {
            let (inner, len) = match types[ty(*dst)?].kind {
                TypeKind::Array { inner, len } => (inner, len),
                _ => return Err(error(line, "E_TYPE", "array constructor has wrong type")),
            };
            if elements.len() != len {
                return Err(error(line, "E_TYPE", "array element count differs"));
            }
            for element in elements {
                if ty(*element)? != inner {
                    return Err(error(line, "E_TYPE", "array element type differs"));
                }
            }
        }
        Op::SliceBorrow {
            dst,
            place,
            mutable,
        } => {
            let inner = match types[place_ty(place)?].kind {
                TypeKind::Array { inner, .. } | TypeKind::Vec { inner } => inner,
                _ => {
                    return Err(error(
                        line,
                        "E_TYPE",
                        "slice borrow requires an array or Vec",
                    ));
                }
            };
            let slice = match types[ty(*dst)?].kind {
                TypeKind::Ref {
                    inner,
                    mutable: kind_mut,
                } if kind_mut == *mutable => inner,
                _ => {
                    return Err(error(
                        line,
                        "E_TYPE",
                        "slice borrow needs a matching reference type",
                    ));
                }
            };
            if !matches!(types[slice].kind, TypeKind::Slice { inner: item, .. } if item == inner) {
                return Err(error(line, "E_TYPE", "slice element type differs"));
            }
            if *mutable && !place_writable(types, function, place) {
                return Err(error(line, "E_BORROW", "mutable slice of immutable place"));
            }
        }
        Op::Assign { place, src } => {
            if place_ty(place)? != ty(*src)? {
                return Err(error(line, "E_TYPE", "assignment type differs"));
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
            if program.destructors.iter().any(|entry| entry.1 == *called) {
                return Err(error(
                    line,
                    "E_TYPE",
                    "destructor functions are invoked only by drop",
                ));
            }
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
    let mut used = Vec::new();
    let mut changed = Vec::new();
    for arg in args {
        let value = Place::local(*arg);
        let _ = carried_provenance(state, types, function, &value, &value, line)?;
        let ty = function.locals[*arg].ty;
        if let TypeKind::Ref { mutable, .. } = types[ty].kind {
            let place = Place {
                local: *arg,
                projections: vec![Projection::Deref],
            };
            access(state, types, function, &place, mutable, 0, line)?;
            let target = origin(state, &place).0;
            if mutable {
                changed.push(target.clone());
            }
            if let Root::Local(local) = target.root {
                let base = Place {
                    local,
                    projections: target.projections.clone(),
                };
                for loan in &state.loans {
                    if within(&loan.holder, &base) {
                        used.push((loan.clone(), mutable && loan.mutable));
                    }
                }
            }
        } else {
            for loan in &state.loans {
                if loan.holder.local == *arg {
                    used.push((loan.clone(), loan.mutable));
                }
            }
        }
    }
    for (loan, mutable) in &used {
        loan_use(state, loan, *mutable, line)?;
        if *mutable {
            changed.push(loan.target.clone());
        }
    }
    for target in changed {
        invalidate(state, &target);
    }
    Ok(())
}

fn check_function(program: &Program, function: &Function) -> Result<(), Diagnostic> {
    let types = &program.types;
    let mut state = State {
        live: vec![false; function.locals.len()],
        moved: Vec::new(),
        loans: Vec::new(),
        absent_references: Vec::new(),
        tags: Vec::new(),
        predicates: BTreeMap::new(),
        reachable: true,
    };
    for i in 0..function.params {
        state.live[i] = true;
        for (path, mutable) in reference_paths(types, function.locals[i].ty) {
            let mut projections = path.clone();
            if !path.is_empty() {
                projections.push(Projection::Deref);
            }
            state.loans.push(Loan {
                holder: Place {
                    local: i,
                    projections: path,
                },
                target: Origin {
                    root: Root::Parameter(i),
                    projections,
                },
                mutable,
                parents: Vec::new(),
            });
        }
    }
    // Keep the exact permitted source leaves. A common root alone would allow
    // a borrow of an unrelated scalar field to impersonate a stored reference.
    let return_origins = match &function.return_source {
        Some(ReturnSource::Parameter { index, projections }) => {
            let place = parameter_source(types, function, *index, projections).unwrap();
            carried_provenance(&state, types, function, &place, &place, 1)?
                .loans
                .into_iter()
                .map(|loan| loan.target)
                .collect::<Vec<_>>()
        }
        _ => Vec::new(),
    };
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
                let predicate = state.predicates.get(condition).cloned();
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
                    set_tag(&mut state, tag);
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
            Op::Text { dst, .. } => {
                put(&mut state, *dst, line)?;
                state.loans.push(Loan {
                    holder: Place::local(*dst),
                    target: Origin {
                        root: Root::Static,
                        projections: Vec::new(),
                    },
                    mutable: false,
                    parents: Vec::new(),
                });
            }
            Op::Int { dst, .. } | Op::Float { dst, .. } | Op::Unit { dst } => {
                put(&mut state, *dst, line)?
            }
            Op::Read { dst, place, moving } => {
                access(&state, types, function, place, *moving, 0, line)?;
                let target = origin(&state, place).0;
                let known_tag = tag_at(&state, &target, place);
                let predicate = if place.projections.is_empty() {
                    state.predicates.get(&place.local).cloned()
                } else {
                    None
                };
                let destination = Place::local(*dst);
                let mut carried =
                    carried_provenance(&state, types, function, place, &destination, line)?;
                if *moving {
                    remap_parents(&mut state, place, &destination);
                    remap_carried(&mut carried.loans, place, &destination);
                    if place.projections.is_empty() {
                        consume(&mut state, types, function, place.local, line)?;
                    } else {
                        remove_provenance(&mut state, place);
                        invalidate(&mut state, &target);
                        state.moved.push(place.clone());
                    }
                }
                put(&mut state, *dst, line)?;
                if let Some(tag) = predicate {
                    state.predicates.insert(*dst, tag);
                }
                install_provenance(&mut state, carried);
                if let Some(variant) = known_tag {
                    set_tag(
                        &mut state,
                        Tag {
                            target: Origin::local(*dst),
                            witness: Place::local(*dst),
                            variant,
                        },
                    );
                }
            }
            Op::Borrow {
                dst,
                place,
                mutable,
            }
            | Op::SliceBorrow {
                dst,
                place,
                mutable,
            } => {
                access(&state, types, function, place, *mutable, 0, line)?;
                let (target, parents) = origin(&state, place);
                put(&mut state, *dst, line)?;
                state.loans.push(Loan {
                    holder: Place::local(*dst),
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
                let source = Place::local(*src);
                let destination = storage_place(&target, place);
                let mut carried =
                    carried_provenance(&state, types, function, &source, &destination, line)?;
                check_external_write(&target, &destination, &carried, line)?;
                let known_tag = tag_at(&state, &Origin::local(*src), &Place::local(*src));
                let predicate = state.predicates.get(src).cloned();
                remap_parents(&mut state, &source, &destination);
                remap_carried(&mut carried.loans, &source, &destination);
                consume(&mut state, types, function, *src, line)?;
                invalidate(&mut state, &target);
                state.live[place.local] = true;
                state.moved.retain(|moved| {
                    !(moved.local == place.local
                        && is_prefix(&place.projections, &moved.projections))
                });
                if matches!(target.root, Root::Local(_)) {
                    remove_provenance(&mut state, &destination);
                    install_provenance(&mut state, carried);
                    validate_stored_view(&state, &target, line)?;
                }
                if place.projections.is_empty() {
                    state.predicates.remove(&place.local);
                    if let Some(tag) = predicate {
                        state.predicates.insert(place.local, tag);
                    }
                }
                if let Some(variant) = known_tag {
                    set_tag(
                        &mut state,
                        Tag {
                            target,
                            witness: place.clone(),
                            variant,
                        },
                    );
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
                let pt = place_type(types, function, place).unwrap();
                if has_destructor(program, pt) {
                    if state.moved.iter().any(|moved| within(moved, place)) {
                        return Err(error(
                            line,
                            "E_MOVE",
                            "custom drop requires every field to be initialized",
                        ));
                    }
                    for loan in &state.loans {
                        if within(&loan.holder, place) {
                            loan_use(&state, loan, loan.mutable, line)?;
                        }
                    }
                }
                remove_provenance(&mut state, place);
                if place.projections.is_empty() {
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
                let callee = &program.functions[*called];
                let mut returned = Provenance {
                    loans: Vec::new(),
                    absent: Vec::new(),
                };
                let source = match &callee.return_source {
                    Some(ReturnSource::Parameter { index, projections }) => {
                        let mut source =
                            parameter_source(types, callee, *index, projections).unwrap();
                        source.local = args[*index];
                        let incoming =
                            carried_provenance(&state, types, function, &source, &source, line)?;
                        common_origin(&incoming.loans, line)?
                    }
                    Some(ReturnSource::Static) => Some((
                        Origin {
                            root: Root::Static,
                            projections: Vec::new(),
                        },
                        Vec::new(),
                    )),
                    None => None,
                };
                for (projections, mutable) in reference_paths(types, callee.result) {
                    let holder = Place {
                        local: *dst,
                        projections,
                    };
                    if let Some((target, parents)) = &source {
                        if mutable && target.root == Root::Static {
                            return Err(error(
                                line,
                                "E_BORROW",
                                "static storage cannot supply an exclusive result",
                            ));
                        }
                        returned.loans.push(Loan {
                            holder,
                            target: target.clone(),
                            mutable,
                            parents: parents.clone(),
                        });
                    } else if holder
                        .projections
                        .iter()
                        .any(|p| matches!(p, Projection::VariantField(_, _)))
                    {
                        returned.absent.push(holder);
                    } else {
                        return Err(error(
                            line,
                            "E_BORROW",
                            "call result reference has no source provenance",
                        ));
                    }
                }
                call_borrows(&mut state, types, function, args, line)?;
                for arg in args {
                    consume(&mut state, types, function, *arg, line)?;
                }
                put(&mut state, *dst, line)?;
                install_provenance(&mut state, returned);
            }
            Op::Struct { dst, fields } | Op::Variant { dst, fields, .. } => {
                let mut carried = Provenance {
                    loans: Vec::new(),
                    absent: Vec::new(),
                };
                let mut transfers = Vec::new();
                for (i, field) in fields.iter().enumerate() {
                    let projection = match instruction.op {
                        Op::Variant { variant, .. } => Projection::VariantField(variant, i),
                        _ => Projection::Field(i),
                    };
                    let to = Place {
                        local: *dst,
                        projections: vec![projection],
                    };
                    let from = Place::local(*field);
                    let field_value =
                        carried_provenance(&state, types, function, &from, &to, line)?;
                    carried.loans.extend(field_value.loans);
                    carried.absent.extend(field_value.absent);
                    transfers.push((from, to));
                }
                if let Op::Variant { variant, .. } = instruction.op {
                    for (projections, _) in reference_paths(types, function.locals[*dst].ty) {
                        if matches!(projections.first(), Some(Projection::VariantField(tag, _)) if *tag != variant)
                        {
                            carried.absent.push(Place {
                                local: *dst,
                                projections,
                            });
                        }
                    }
                }
                validate_view(&carried.loans, line)?;
                for (from, to) in &transfers {
                    remap_parents(&mut state, from, to);
                    remap_carried(&mut carried.loans, from, to);
                }
                for field in fields {
                    consume(&mut state, types, function, *field, line)?;
                }
                put(&mut state, *dst, line)?;
                install_provenance(&mut state, carried);
                if let Op::Variant { variant, .. } = instruction.op {
                    set_tag(
                        &mut state,
                        Tag {
                            target: Origin::local(*dst),
                            witness: Place::local(*dst),
                            variant,
                        },
                    );
                }
            }
            Op::Array { dst, elements } => {
                for value in elements {
                    consume(&mut state, types, function, *value, line)?;
                }
                put(&mut state, *dst, line)?;
            }
            Op::Discriminant { dst, src } | Op::TagTest { dst, src, .. } => {
                let place = enum_place(types, function, *src);
                access(&state, types, function, &place, false, 0, line)?;
                let target = origin(&state, &place).0;
                put(&mut state, *dst, line)?;
                if let Op::TagTest { variant, .. } = instruction.op {
                    state.predicates.insert(
                        *dst,
                        Tag {
                            target,
                            witness: place,
                            variant,
                        },
                    );
                }
            }
            Op::Intrinsic { dst, name, args } => {
                if name == "str_as_bytes" || name == "str_from_utf8" {
                    let source = Place::local(args[0]);
                    let destination = Place {
                        local: *dst,
                        projections: if name == "str_from_utf8" {
                            vec![Projection::VariantField(0, 0)]
                        } else {
                            Vec::new()
                        },
                    };
                    let returned =
                        carried_provenance(&state, types, function, &source, &destination, line)?;
                    call_borrows(&mut state, types, function, args, line)?;
                    consume(&mut state, types, function, args[0], line)?;
                    put(&mut state, *dst, line)?;
                    install_provenance(&mut state, returned);
                } else if name == "replace" || name == "clone" {
                    let place = Place {
                        local: args[0],
                        projections: vec![Projection::Deref],
                    };
                    let target = origin(&state, &place).0;
                    let stored = storage_place(&target, &place);
                    let destination = Place::local(*dst);
                    let mut returned =
                        carried_provenance(&state, types, function, &place, &destination, line)?;
                    let old_tag = tag_at(&state, &target, &stored);
                    let incoming = if name == "replace" {
                        let source = Place::local(args[1]);
                        let value =
                            carried_provenance(&state, types, function, &source, &stored, line)?;
                        check_external_write(&target, &stored, &value, line)?;
                        let tag = tag_at(&state, &Origin::local(args[1]), &source);
                        Some((source, value, tag))
                    } else {
                        None
                    };
                    call_borrows(&mut state, types, function, args, line)?;
                    if let Some((source, mut value, tag)) = incoming {
                        remap_parents(&mut state, &stored, &destination);
                        remap_carried(&mut returned.loans, &stored, &destination);
                        remap_parents(&mut state, &source, &stored);
                        remap_carried(&mut value.loans, &source, &stored);
                        for arg in args {
                            consume(&mut state, types, function, *arg, line)?;
                        }
                        if matches!(target.root, Root::Local(_)) {
                            remove_provenance(&mut state, &stored);
                            install_provenance(&mut state, value);
                            validate_stored_view(&state, &target, line)?;
                        }
                        if let Some(variant) = tag {
                            set_tag(
                                &mut state,
                                Tag {
                                    target: target.clone(),
                                    witness: stored,
                                    variant,
                                },
                            );
                        }
                    } else {
                        for arg in args {
                            consume(&mut state, types, function, *arg, line)?;
                        }
                    }
                    put(&mut state, *dst, line)?;
                    install_provenance(&mut state, returned);
                    if let Some(variant) = old_tag {
                        set_tag(
                            &mut state,
                            Tag {
                                target: Origin::local(*dst),
                                witness: destination,
                                variant,
                            },
                        );
                    }
                } else {
                    call_borrows(&mut state, types, function, args, line)?;
                    for arg in args {
                        consume(&mut state, types, function, *arg, line)?;
                    }
                    put(&mut state, *dst, line)?;
                }
            }
            Op::Return { src } => {
                let returned = Place::local(*src);
                let provenance =
                    carried_provenance(&state, types, function, &returned, &returned, line)?;
                for loan in &provenance.loans {
                    let allowed = match (&function.return_source, &loan.target.root) {
                        (Some(ReturnSource::Parameter { index, .. }), Root::Parameter(actual)) => {
                            index == actual
                                && return_origins.iter().any(|source| {
                                    source.root == loan.target.root
                                        && is_prefix(&source.projections, &loan.target.projections)
                                })
                        }
                        (
                            Some(ReturnSource::Parameter { .. } | ReturnSource::Static),
                            Root::Static,
                        ) => !loan.mutable,
                        _ => false,
                    };
                    if !allowed {
                        return Err(error(
                            line,
                            "E_BORROW",
                            "returned view differs from its declared source",
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
        TypeKind::Array { inner, .. } => children.push(*inner),
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

/// Solve reference containment once over the type graph. Recursive owned types
/// are allowed; a reference seed propagates to every transitive container.
fn reference_containment(types: &[Type]) -> Vec<bool> {
    let mut contains = vec![false; types.len()];
    let mut parents = vec![Vec::new(); types.len()];
    let mut pending = Vec::new();
    for (parent, ty) in types.iter().enumerate() {
        let mut children = direct_children(&ty.kind);
        match ty.kind {
            TypeKind::Ref { .. } | TypeKind::Slice { .. } => {
                contains[parent] = true;
                pending.push(parent);
            }
            TypeKind::Box { inner } | TypeKind::Vec { inner } => children.push(inner),
            _ => {}
        }
        for child in children {
            parents[child].push(parent);
        }
    }
    while let Some(child) = pending.pop() {
        for parent in &parents[child] {
            if !contains[*parent] {
                contains[*parent] = true;
                pending.push(*parent);
            }
        }
    }
    contains
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
    for ty in types {
        match &ty.kind {
            TypeKind::Ref {
                inner,
                mutable: true,
            } if matches!(types[*inner].kind, TypeKind::Str) => {
                return Err(error(
                    1,
                    "E_UNSUPPORTED",
                    "mutable text views are not implemented",
                ));
            }
            TypeKind::Box { inner } | TypeKind::Vec { inner } | TypeKind::Slice { inner, .. }
                if matches!(types[*inner].kind, TypeKind::Str) =>
            {
                return Err(error(
                    1,
                    "E_TYPE",
                    "unsized text cannot be an owned or collection element",
                ));
            }
            _ => {}
        }
        for child in direct_children(&ty.kind) {
            if matches!(types[child].kind, TypeKind::Str) {
                return Err(error(1, "E_TYPE", "unsized text cannot be stored by value"));
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
    let contains_reference = reference_containment(types);
    for i in 0..types.len() {
        if matches!(
            types[i].kind,
            TypeKind::Box { .. } | TypeKind::Vec { .. } | TypeKind::Array { .. }
        ) && contains_reference[i]
        {
            return Err(error(
                1,
                "E_BORROW",
                "views cannot be stored in Box, Vec, or owned arrays",
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
    let mut function_names = BTreeSet::new();
    for function in &program.functions {
        if !function_names.insert(function.name.as_str()) {
            return Err(error(1, "E_K", "duplicate function name"));
        }
        if function.params > function.locals.len() || function.result >= program.types.len() {
            return Err(error(1, "E_K", "invalid function signature"));
        }
        for local in &function.locals {
            if local.ty >= program.types.len() {
                return Err(error(1, "E_K", "unknown local type"));
            }
            if matches!(
                program.types[local.ty].kind,
                TypeKind::Slice { .. } | TypeKind::Str
            ) {
                return Err(error(
                    1,
                    "E_UNSUPPORTED",
                    "unsized values need a borrowed type",
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
        if matches!(
            program.types[function.result].kind,
            TypeKind::Slice { .. } | TypeKind::Str
        ) {
            return Err(error(
                1,
                "E_UNSUPPORTED",
                "unsized results need a borrowed type",
            ));
        }
        if let Some(source) = &function.return_source {
            let valid = is_view(&program.types, function.result)
                && match source {
                    ReturnSource::Parameter { index, projections } => {
                        parameter_source(&program.types, function, *index, projections).is_some()
                    }
                    ReturnSource::Static => reference_paths(&program.types, function.result)
                        .iter()
                        .all(|(_, mutable)| !mutable),
                };
            if !valid {
                return Err(error(
                    1,
                    "E_BORROW",
                    "invalid returned-view source contract",
                ));
            }
        }
    }
    let mut declared = Vec::new();
    for (ty, called) in &program.destructors {
        if *ty >= program.types.len() || *called >= program.functions.len() || declared.contains(ty)
        {
            return Err(error(
                1,
                "E_K",
                "invalid or duplicate destructor declaration",
            ));
        }
        declared.push(*ty);
        if !matches!(
            program.types[*ty].kind,
            TypeKind::Struct { .. } | TypeKind::Enum { .. }
        ) || is_copy(&program.types, *ty)
        {
            return Err(error(
                1,
                "E_TYPE",
                "custom Drop requires a non-Copy nominal type",
            ));
        }
        let function = &program.functions[*called];
        if function.params != 1
            || function.result != UNIT
            || !matches!(program.types[function.locals[0].ty].kind, TypeKind::Ref { inner, mutable: true } if inner == *ty)
        {
            return Err(error(
                1,
                "E_TYPE",
                "destructor must have signature fn(&mut T) -> ()",
            ));
        }
    }
    for function in &program.functions {
        check_function(&program, function).map_err(|mut problem| {
            problem.file = function.file;
            problem
        })?;
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
            file: 0,
            return_source: None,
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
    fn recursive_containers_preserve_the_reference_storage_rule() {
        for reference in [false, true] {
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

    #[test]
    fn heap_storage_rejects_views() {
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

    #[test]
    fn view_field_keeps_its_owner_borrowed() {
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

    #[test]
    fn by_value_view_can_return_its_stored_reference() {
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

    #[test]
    fn by_value_view_cannot_return_its_owned_cache() {
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

    #[test]
    fn by_value_view_cannot_return_address_of_its_reference_slot() {
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

    #[test]
    fn reborrow_freezes_the_view_that_holds_its_mutable_parent() {
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

    #[test]
    fn exclusive_view_borrow_also_blocks_direct_use_of_reference_fields() {
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

    #[test]
    fn declared_return_source_is_verified_against_the_body() {
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

    #[test]
    fn one_view_cannot_combine_two_source_roots() {
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

    #[test]
    fn custom_drop_forbids_partial_moves() {
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
                src: 0,
                variant: 0,
            },
            Op::If { condition: 1 },
        ];
        if replace_field {
            body.extend([
                Op::Int { dst: 7, value: 42 },
                Op::Assign {
                    place: Place {
                        local: 0,
                        projections: vec![Projection::VariantField(0, 1)],
                    },
                    src: 7,
                },
            ]);
        }
        if let Some(before_branch) = mutation {
            let at = if before_branch { 1 } else { 2 };
            body.splice(
                at..at,
                [
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
                ],
            );
        }
        body.extend([
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
        ]);
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

    #[test]
    fn payload_moves_and_field_writes_preserve_the_enclosing_enum_tag() {
        for replace in [false, true] {
            check(enum_payload_operations(replace, None)).unwrap();
        }
        let mut dropped = enum_payload_operations(false, None);
        let operation = dropped.functions[0]
            .body
            .iter_mut()
            .find(|instruction| matches!(instruction.op, Op::Read { dst: 2, .. }))
            .unwrap();
        operation.op = Op::Drop {
            place: Place {
                local: 0,
                projections: vec![Projection::VariantField(0, 0)],
            },
        };
        dropped.functions[0].body.retain(|instruction| {
            !matches!(&instruction.op, Op::Drop { place } if *place == Place::local(2))
        });
        check(dropped).unwrap();
    }

    #[test]
    fn whole_enum_mutation_invalidates_both_saved_predicates_and_proven_tags() {
        for before_branch in [false, true] {
            let failure = check(enum_payload_operations(false, Some(before_branch)))
                .err()
                .unwrap();
            assert_eq!(failure.code, "E_TYPE");
            assert!(
                failure.message.contains("matching checked tag"),
                "{failure:?}"
            );
        }
    }

    #[test]
    fn tag_proof_cannot_follow_a_reused_index_into_another_reference() {
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
                        src: 2,
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
