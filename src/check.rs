//! The single gate for explicit K. Each structured edge has an ownership state.
use crate::ast::{Diagnostic, Span};
use crate::facts::Facts;
use crate::k::*;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub(crate) mod scoped;

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
            TypeKind::Unit
            | TypeKind::Bool
            | TypeKind::Char
            | TypeKind::Int { .. }
            | TypeKind::F64 => {}
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
            TypeKind::World | TypeKind::Stdout => return false,
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Root {
    /// Immutable storage that lives for the entire module instance.
    Static,
    Local(usize),
    Parameter(usize),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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

/// An exact referent path, or a bound that includes any descendant selected by
/// a checked call. Projecting through a bound must not narrow its footprint.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourceBound {
    origin: Origin,
    exact: bool,
}
#[derive(Clone)]
struct Loan {
    // Diagnostic metadata is deliberately excluded from semantic equality.
    created: Vec<(usize, usize)>,
    holder: Place,
    sources: Vec<SourceBound>,
    mutable: bool,
    parents: Vec<Place>,
    effects: BTreeSet<scoped::EffectId>,
}
impl PartialEq for Loan {
    fn eq(&self, other: &Self) -> bool {
        self.holder == other.holder
            && same_set(&self.sources, &other.sources)
            && self.mutable == other.mutable
            && same_set(&self.parents, &other.parents)
            && self.effects == other.effects
    }
}
impl Eq for Loan {}

/// Each reference leaf has one holder identity. Root and parent indexes keep a
/// shared read independent of unrelated loans and of other shared references.
#[derive(Clone, Default)]
struct Loans {
    values: BTreeMap<Place, Loan>,
    holders: BTreeMap<usize, BTreeSet<Place>>,
    targets: BTreeMap<Root, BTreeSet<Place>>,
    mutable_targets: BTreeMap<Root, BTreeSet<Place>>,
    parents: BTreeMap<usize, BTreeSet<Place>>,
    effects: BTreeMap<scoped::EffectId, BTreeSet<Place>>,
}
fn remove_index<K: Ord>(map: &mut BTreeMap<K, BTreeSet<Place>>, key: &K, place: &Place) {
    if let Some(entries) = map.get_mut(key) {
        entries.remove(place);
        if entries.is_empty() {
            map.remove(key);
        }
    }
}
impl Loans {
    fn push(&mut self, loan: Loan) {
        let holder = loan.holder.clone();
        self.remove(&holder);
        self.holders
            .entry(holder.local)
            .or_default()
            .insert(holder.clone());
        for source in &loan.sources {
            self.targets
                .entry(source.origin.root.clone())
                .or_default()
                .insert(holder.clone());
            if loan.mutable {
                self.mutable_targets
                    .entry(source.origin.root.clone())
                    .or_default()
                    .insert(holder.clone());
            }
        }
        for parent in &loan.parents {
            self.parents
                .entry(parent.local)
                .or_default()
                .insert(holder.clone());
        }
        for effect in &loan.effects {
            self.effects
                .entry(*effect)
                .or_default()
                .insert(holder.clone());
        }
        self.values.insert(holder, loan);
    }
    fn remove(&mut self, holder: &Place) -> Option<Loan> {
        let loan = self.values.remove(holder)?;
        remove_index(&mut self.holders, &holder.local, holder);
        for source in &loan.sources {
            remove_index(&mut self.targets, &source.origin.root, holder);
            if loan.mutable {
                remove_index(&mut self.mutable_targets, &source.origin.root, holder);
            }
        }
        for parent in &loan.parents {
            remove_index(&mut self.parents, &parent.local, holder);
        }
        for effect in &loan.effects {
            remove_index(&mut self.effects, effect, holder);
        }
        Some(loan)
    }
    fn remove_place(&mut self, place: &Place) {
        let holders: Vec<_> = self
            .for_holder(place.local)
            .filter(|loan| within(&loan.holder, place))
            .map(|loan| loan.holder.clone())
            .collect();
        for holder in holders {
            self.remove(&holder);
        }
    }
    fn extend(&mut self, loans: Vec<Loan>) {
        for loan in loans {
            self.push(loan);
        }
    }
    fn get(&self, holder: &Place) -> Option<&Loan> {
        self.values.get(holder)
    }
    fn for_holder(&self, local: usize) -> impl Iterator<Item = &Loan> {
        self.holders
            .get(&local)
            .into_iter()
            .flatten()
            .map(|holder| &self.values[holder])
    }
    fn for_target(&self, root: &Root, mutable_only: bool) -> impl Iterator<Item = &Loan> {
        let index = if mutable_only {
            &self.mutable_targets
        } else {
            &self.targets
        };
        index
            .get(root)
            .into_iter()
            .flatten()
            .map(|holder| &self.values[holder])
    }
    fn remap_parents(&mut self, from: &Place, to: &Place) {
        let holders: Vec<_> = self
            .parents
            .get(&from.local)
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        for holder in holders {
            let mut loan = self.remove(&holder).unwrap();
            for parent in &mut loan.parents {
                *parent = rebase(parent, from, to);
            }
            self.push(loan);
        }
    }
}
impl PartialEq for Loans {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}
impl Eq for Loans {}
struct ResolvedOrigin {
    sources: Vec<SourceBound>,
    parents: Vec<Place>,
    effects: BTreeSet<scoped::EffectId>,
    absent: bool,
}
impl ResolvedOrigin {
    fn exact(&self) -> bool {
        self.sources.len() == 1 && self.sources[0].exact
    }
    fn storage(&self, fallback: &Place) -> Place {
        if self.exact() {
            storage_place(&self.sources[0].origin, fallback)
        } else {
            fallback.clone()
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
struct Tag {
    target: Vec<Origin>,
    witness: Place,
    variant: usize,
}
#[derive(Clone, PartialEq)]
struct State {
    // Definite and possible presence of each owner shell. A shell can remain
    // present while some fields are absent; those facts are tracked separately.
    live: Facts,
    maybe_live: Facts,
    moved: Vec<Place>,
    definitely_moved: Vec<Place>,
    views: Rc<Vec<bool>>,
    site: Option<(usize, usize)>,
    loans: Loans,
    effects: scoped::Effects,
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
    start: usize,
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
fn normalize_sources(sources: &mut Vec<SourceBound>) {
    sources.sort();
    // A bound subsumes the exact source with the same prefix.
    sources.dedup_by(|a, b| a.origin == b.origin);
}
fn source_origins(sources: &[SourceBound]) -> Vec<Origin> {
    let mut origins: Vec<_> = sources.iter().map(|source| source.origin.clone()).collect();
    origins.sort();
    origins.dedup();
    origins
}
fn overlaps_sources(a: &[SourceBound], b: &[SourceBound]) -> bool {
    a.iter()
        .any(|a| b.iter().any(|b| overlaps(&a.origin, &b.origin)))
}
fn source_overlaps(sources: &[SourceBound], target: &Origin) -> bool {
    sources
        .iter()
        .any(|source| overlaps(&source.origin, target))
}
fn resolve_origin(state: &State, place: &Place) -> ResolvedOrigin {
    let initial = SourceBound {
        origin: holder_origin(place),
        exact: true,
    };
    let mut pending = vec![(initial, Vec::<Place>::new(), BTreeSet::new())];
    let mut permissions = Vec::new();
    let mut sources = Vec::new();
    let mut parents = Vec::new();
    let mut absent = false;
    while let Some((source, visited, mut effects)) = pending.pop() {
        let mut found = None;
        if source.exact {
            if let Root::Local(local) = source.origin.root {
                for (index, projection) in source.origin.projections.iter().enumerate() {
                    if !matches!(projection, Projection::Deref) {
                        continue;
                    }
                    let holder = Place {
                        local,
                        projections: source.origin.projections[..index].to_vec(),
                    };
                    if visited.contains(&holder) {
                        continue;
                    }
                    if let Some(loan) = state.loans.get(&holder) {
                        found = Some((index, Some(loan)));
                        break;
                    }
                    if state.absent_references.contains(&holder) {
                        found = Some((index, None));
                        break;
                    }
                }
            }
        }
        if let Some((index, Some(loan))) = found {
            let mut visited = visited;
            visited.push(loan.holder.clone());
            for parent in &loan.parents {
                if !visited.contains(parent) {
                    visited.push(parent.clone());
                }
            }
            for parent in &visited {
                if !parents.contains(parent) {
                    parents.push(parent.clone());
                }
            }
            if state.absent_references.contains(&loan.holder) {
                absent = true;
            }
            effects.extend(&loan.effects);
            for bound in &loan.sources {
                let mut next = bound.clone();
                if next.exact {
                    next.origin
                        .projections
                        .extend_from_slice(&source.origin.projections[index + 1..]);
                }
                pending.push((next, visited.clone(), effects.clone()));
            }
        } else if found.is_some() {
            absent = true;
        } else {
            sources.push(source);
            permissions.push(effects);
        }
    }
    normalize_sources(&mut sources);
    ResolvedOrigin {
        effects: scoped::common_permissions(permissions.iter()),
        sources,
        parents,
        absent,
    }
}
fn origin(state: &State, place: &Place) -> (Vec<Origin>, Vec<Place>) {
    let resolved = resolve_origin(state, place);
    (source_origins(&resolved.sources), resolved.parents)
}
fn affects_tag(target: &Origin, tag: &Tag) -> bool {
    // Index operands in the witness are read again at each projection. A
    // changed operand retargets that witness even though its owner is intact.
    // Indices in resolved origins may instead describe an address captured by
    // an earlier borrow; they do not retarget a fixed pointer witness.
    let retargeted = matches!(&target.root, Root::Local(local)
        if tag.witness.projections.iter().any(|projection|
            matches!(projection, Projection::Index(index) if index == local)));
    retargeted
        || tag.target.iter().any(|tested| {
            target.projections.len() <= tested.projections.len() && overlaps(target, tested)
        })
        || (target.projections.len() <= tag.witness.projections.len()
            && overlaps(target, &holder_origin(&tag.witness)))
}
fn invalidate(state: &mut State, targets: &[Origin]) {
    state
        .tags
        .retain(|tag| !targets.iter().any(|target| affects_tag(target, tag)));
    state
        .predicates
        .retain(|_, tag| !targets.iter().any(|target| affects_tag(target, tag)));
}
fn tag_at(state: &State, targets: &[Origin], witness: &Place) -> Option<usize> {
    state
        .tags
        .iter()
        .find(|tag| same_set(&tag.target, targets) && tag.witness == *witness)
        .map(|tag| tag.variant)
}
fn set_tag(state: &mut State, tag: Tag) {
    state.tags.retain(|old| old.witness != tag.witness);
    state.tags.push(tag);
}

/// Reference leaves of a view. Owned heap containers cannot contain views.
pub(crate) fn reference_paths(types: &[Type], ty: usize) -> Vec<(Vec<Projection>, bool)> {
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
/// A signature selector names one stored input reference leaf. Moving exclusive
/// leaves out through a borrowed container needs a mutation contract; selecting
/// shared leaves does not alter the container's inventory.
fn parameter_leaf_source(
    types: &[Type],
    function: &Function,
    index: usize,
    projections: &[Projection],
) -> Option<Place> {
    if index >= function.params {
        return None;
    }
    let mut ty = function.locals[index].ty;
    let mut borrowed = false;
    for projection in projections {
        ty = match (projection, &types[ty].kind) {
            (Projection::Deref, TypeKind::Ref { inner, .. }) => {
                borrowed = true;
                *inner
            }
            (Projection::Field(field), TypeKind::Struct { fields, .. }) => fields.get(*field)?.ty,
            (Projection::VariantField(tag, field), TypeKind::Enum { variants, .. }) => {
                variants.get(*tag)?.fields.get(*field)?.ty
            }
            _ => return None,
        };
    }
    let TypeKind::Ref { mutable, .. } = types[ty].kind else {
        return None;
    };
    if borrowed && mutable {
        return None;
    }
    Some(Place {
        local: index,
        projections: projections.to_vec(),
    })
}
fn projected_type(types: &[Type], mut ty: usize, projections: &[Projection]) -> Option<usize> {
    for projection in projections {
        ty = match (projection, &types[ty].kind) {
            (Projection::Field(field), TypeKind::Struct { fields, .. }) => fields.get(*field)?.ty,
            (Projection::VariantField(tag, field), TypeKind::Enum { variants, .. }) => {
                variants.get(*tag)?.fields.get(*field)?.ty
            }
            _ => return None,
        };
    }
    Some(ty)
}
fn valid_leaf_contract(types: &[Type], function: &Function, leaves: &[ReturnLeaf]) -> bool {
    let paths = reference_paths(types, function.result);
    if leaves.len() != paths.len() {
        return false;
    }
    let mut seen = BTreeSet::new();
    for leaf in leaves {
        if !seen.insert(leaf.projections.clone()) {
            return false;
        }
        let Some(output) = projected_type(types, function.result, &leaf.projections) else {
            return false;
        };
        let TypeKind::Ref {
            inner: output_inner,
            mutable: output_mutable,
        } = types[output].kind
        else {
            return false;
        };
        match &leaf.origin {
            LeafOrigin::Static => {
                if output_mutable {
                    return false;
                }
            }
            LeafOrigin::Inputs { inputs, exact } => {
                if inputs.is_empty() {
                    return false;
                }
                let mut selectors = BTreeSet::new();
                for input in inputs {
                    let Some(input) =
                        parameter_leaf_source(types, function, input.index, &input.projections)
                    else {
                        return false;
                    };
                    if !selectors.insert(input.clone()) {
                        return false;
                    }
                    let ty = place_type(types, function, &input).unwrap();
                    let TypeKind::Ref { inner, mutable } = types[ty].kind else {
                        return false;
                    };
                    if (output_mutable && !mutable) || (*exact && inner != output_inner) {
                        return false;
                    }
                }
            }
        }
    }
    paths.iter().all(|(path, _)| seen.contains(path))
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
fn common_origin(loans: &[Loan], line: usize) -> Result<Option<ResolvedOrigin>, Diagnostic> {
    if loans.is_empty() {
        return Ok(None);
    }
    let mut sources = Vec::new();
    let mut parents = Vec::new();
    for loan in loans {
        if loan.mutable
            && loan
                .sources
                .iter()
                .any(|source| source.origin.root == Root::Static)
        {
            return Err(error(line, "E_BORROW", "static storage is immutable"));
        }
        sources.extend_from_slice(&loan.sources);
        for parent in &loan.parents {
            if !parents.contains(parent) {
                parents.push(parent.clone());
            }
        }
    }
    normalize_sources(&mut sources);
    Ok(Some(ResolvedOrigin {
        effects: scoped::common_permissions(loans.iter().map(|loan| &loan.effects)),
        sources,
        parents,
        absent: false,
    }))
}
fn validate_view(loans: &[Loan], line: usize) -> Result<(), Diagnostic> {
    let mut indexed: BTreeMap<Root, Vec<&Loan>> = BTreeMap::new();
    for loan in loans {
        let roots: BTreeSet<_> = loan
            .sources
            .iter()
            .map(|source| source.origin.root.clone())
            .collect();
        if loan.mutable && roots.contains(&Root::Static) {
            return Err(error(line, "E_BORROW", "static storage is immutable"));
        }
        for root in roots {
            indexed.entry(root).or_default().push(loan);
        }
    }
    for a in loans.iter().filter(|loan| loan.mutable) {
        for source in &a.sources {
            for b in &indexed[&source.origin.root] {
                if a.holder != b.holder
                    && !exclusive_variants(&a.holder, &b.holder)
                    && overlaps_sources(&a.sources, &b.sources)
                {
                    return Err(error(
                        line,
                        "E_BORROW",
                        "view contains aliasing mutable references",
                    ));
                }
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
    let mut result = Provenance {
        loans: Vec::new(),
        absent: Vec::new(),
    };
    if !state.views[value_ty] {
        return Ok(result);
    }
    let resolved = resolve_origin(state, from);
    for (path, mutable) in reference_paths(types, value_ty) {
        let mut holder = to.clone();
        holder.projections.extend_from_slice(&path);
        let local_storage = if resolved.exact() {
            match resolved.sources[0].origin.root {
                Root::Local(local) => Some(local),
                _ => None,
            }
        } else {
            None
        };
        if let Some(local) = local_storage {
            let mut source = Place {
                local,
                projections: resolved.sources[0].origin.projections.clone(),
            };
            source.projections.extend_from_slice(&path);
            let absent = source
                .projections
                .iter()
                .any(|p| matches!(p, Projection::VariantField(_, _)))
                && state.absent_references.contains(&source);
            if let Some(loan) = state.loans.get(&source) {
                let mut next = loan.clone();
                next.holder = holder.clone();
                result.loans.push(next);
            } else if !absent {
                return Err(error(
                    line,
                    "E_BORROW",
                    "reference-bearing value has incomplete source provenance",
                ));
            }
            if absent {
                result.absent.push(holder);
            }
        } else {
            let mut source = from.clone();
            source.projections.extend_from_slice(&path);
            source.projections.push(Projection::Deref);
            let resolved = resolve_origin(state, &source);
            if resolved.sources.is_empty() && !resolved.absent {
                return Err(error(
                    line,
                    "E_BORROW",
                    "reference-bearing value has incomplete source provenance",
                ));
            }
            if resolved.absent {
                result.absent.push(holder.clone());
            }
            if !resolved.sources.is_empty() {
                result.loans.push(Loan {
                    created: state.site.into_iter().collect(),
                    holder,
                    sources: resolved.sources,
                    mutable,
                    parents: resolved.parents,
                    effects: resolved.effects,
                });
            }
        }
    }
    Ok(result)
}
fn mapped_call_provenance(
    state: &State,
    types: &[Type],
    function: &Function,
    callee: &Function,
    args: &[usize],
    dst: usize,
    line: usize,
) -> Result<Provenance, Diagnostic> {
    let Some(ReturnSource::Leaves(leaves)) = &callee.return_source else {
        unreachable!()
    };
    let mut result = Provenance {
        loans: Vec::new(),
        absent: Vec::new(),
    };
    for leaf in leaves {
        let ty = projected_type(types, callee.result, &leaf.projections).unwrap();
        let TypeKind::Ref { mutable, .. } = types[ty].kind else {
            unreachable!()
        };
        let holder = Place {
            local: dst,
            projections: leaf.projections.clone(),
        };
        let optional = leaf
            .projections
            .iter()
            .any(|p| matches!(p, Projection::VariantField(_, _)));
        let mut static_source = matches!(leaf.origin, LeafOrigin::Static);
        if let LeafOrigin::Inputs { inputs, exact } = &leaf.origin {
            let mut sources = Vec::new();
            let mut parents = Vec::new();
            let mut created = Vec::new();
            let mut permissions = Vec::new();
            let mut absent = false;
            for input in inputs {
                let input = Place {
                    local: args[input.index],
                    projections: input.projections.clone(),
                };
                let value = carried_provenance(state, types, function, &input, &holder, line)?;
                absent |= !value.absent.is_empty();
                for loan in value.loans {
                    sources.extend(loan.sources);
                    parents.extend(loan.parents);
                    created.extend(loan.created);
                    permissions.push(loan.effects);
                }
            }
            if !sources.is_empty() {
                if !exact {
                    for source in &mut sources {
                        source.exact = false;
                    }
                }
                normalize_sources(&mut sources);
                parents.sort();
                parents.dedup();
                if !exact {
                    created = state.site.into_iter().collect();
                }
                created.sort();
                created.dedup();
                result.loans.push(Loan {
                    created,
                    holder: holder.clone(),
                    sources,
                    mutable,
                    parents,
                    effects: scoped::common_permissions(permissions.iter()),
                });
                if absent && optional {
                    result.absent.push(holder);
                }
                continue;
            }
            if !*exact && !mutable {
                // A returning shared leaf with no live selected input can only
                // come from immutable static storage, as checked in the body.
                static_source = true;
            } else if optional {
                result.absent.push(holder);
                continue;
            } else {
                return Err(error(
                    line,
                    "E_BORROW",
                    "call cannot materialize a reference from absent input leaves",
                ));
            }
        }
        if static_source {
            result.loans.push(Loan {
                created: state.site.into_iter().collect(),
                holder,
                sources: vec![SourceBound {
                    origin: Origin {
                        root: Root::Static,
                        projections: Vec::new(),
                    },
                    exact: false,
                }],
                mutable: false,
                parents: Vec::new(),
                effects: BTreeSet::new(),
            });
        }
    }
    validate_view(&result.loans, line)?;
    Ok(result)
}
fn remove_provenance(state: &mut State, place: &Place) {
    state.loans.remove_place(place);
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
        if loan.sources.len() != 1 || !loan.sources[0].exact || loan.sources[0].origin != expected {
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
        let loans: Vec<Loan> = state.loans.for_holder(local).cloned().collect();
        validate_view(&loans, line)?;
    }
    Ok(())
}
fn remap_parents(state: &mut State, from: &Place, to: &Place) {
    state.loans.remap_parents(from, to);
}
fn remap_carried(loans: &mut [Loan], from: &Place, to: &Place) {
    for loan in loans {
        for parent in &mut loan.parents {
            *parent = rebase(parent, from, to);
        }
    }
}
pub(crate) fn place_writable(types: &[Type], function: &Function, place: &Place) -> bool {
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
pub(crate) fn borrowed_projection(types: &[Type], function: &Function, place: &Place) -> bool {
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
fn loan_description(types: &[Type], function: &Function, loan: &Loan) -> String {
    let location = if loan.created.is_empty() {
        "supplied by a function parameter".to_string()
    } else {
        let places: Vec<_> = loan
            .created
            .iter()
            .map(|(line, column)| format!("{}:{}", line, column))
            .collect();
        format!("created at {}", places.join(" or "))
    };
    format!(
        "{} borrow held by {} ({})",
        if loan.mutable { "exclusive" } else { "shared" },
        place_name(types, function, &loan.holder),
        location
    )
}
/// An enum parameter initially has provenance for every alternative. A checked
/// tag proves only the alternatives below that exact witness inactive; it does
/// not prove that every possible referent of a borrowed witness has that tag.
/// Keep the inventory so invalidation and joins restore conservative activity.
fn loan_active(state: &State, loan: &Loan) -> bool {
    if !loan
        .holder
        .projections
        .iter()
        .any(|projection| matches!(projection, Projection::VariantField(_, _)))
    {
        return true;
    }
    !state.tags.iter().any(|tag| {
        within(&loan.holder, &tag.witness)
            && matches!(
                loan.holder.projections.get(tag.witness.projections.len()),
                Some(Projection::VariantField(variant, _)) if *variant != tag.variant
            )
    })
}
fn loan_use(
    state: &State,
    types: &[Type],
    function: &Function,
    loan: &Loan,
    mutable: bool,
    line: usize,
) -> Result<(), Diagnostic> {
    if !loan_active(state, loan) {
        return Ok(());
    }
    state
        .effects
        .check(&loan.sources, &loan.effects, mutable, line)
        .map_err(|mut problem| {
            problem.message = format!(
                "{}: {}",
                place_name(types, function, &loan.holder),
                problem.message
            );
            problem
        })?;
    let mut candidates = BTreeSet::new();
    for source in &loan.sources {
        for other in state.loans.for_target(&source.origin.root, !mutable) {
            candidates.insert(other.holder.clone());
        }
    }
    for holder in candidates {
        let other = state.loans.get(&holder).unwrap();
        if !loan_active(state, other)
            || other.holder == loan.holder
            || loan.parents.contains(&other.holder)
            || exclusive_variants(&loan.holder, &other.holder)
        {
            continue;
        }
        if (mutable || other.mutable) && overlaps_sources(&loan.sources, &other.sources) {
            return Err(error(
                line,
                "E_BORROW",
                &format!(
                    "{} conflicts with {}",
                    place_name(types, function, &loan.holder),
                    loan_description(types, function, other)
                ),
            ));
        }
    }
    Ok(())
}

// Mode 0 reads a whole value; mode 1 replaces/drops remaining fields.
// Mode 2 additionally allows an absent root for conditional cleanup or an
// overwrite. It never permits projecting through a possibly absent shell.
// Mode 3 releases a proven destructor-free tree of scalar/reference descriptors;
// source overlap still forbids dropping storage that a live reference addresses.
fn access(
    state: &State,
    types: &[Type],
    function: &Function,
    place: &Place,
    write: bool,
    mode: u8,
    line: usize,
) -> Result<(), Diagnostic> {
    let optional_root = (mode == 2 || mode == 3) && place.projections.is_empty();
    if !state.live[place.local] && !optional_root {
        return Err(error(
            line,
            "E_MOVE",
            &format!(
                "{} is uninitialized or moved on a reachable path",
                place_name(types, function, place)
            ),
        ));
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
    let resolved = resolve_origin(state, place);
    state
        .effects
        .check(&resolved.sources, &resolved.effects, write, line)
        .map_err(|mut problem| {
            problem.message = format!(
                "{}: {}",
                place_name(types, function, place),
                problem.message
            );
            problem
        })?;
    let targets = source_origins(&resolved.sources);
    let parents = resolved.parents;
    if targets.is_empty() {
        return Err(error(line, "E_BORROW", "place has no reachable referent"));
    }
    for target in &targets {
        if write && target.root == Root::Static {
            return Err(error(line, "E_BORROW", "static storage is immutable"));
        }
        if let Root::Local(local) = target.root {
            if !state.live[local] && !(optional_root && local == place.local) {
                return Err(error(
                    line,
                    "E_MOVE",
                    &format!(
                        "{} points to an owner that may be moved",
                        place_name(types, function, place)
                    ),
                ));
            }
            for moved in &state.moved {
                if moved.local == local && path_overlaps(&moved.projections, &target.projections) {
                    let ancestor = is_prefix(&moved.projections, &target.projections)
                        && moved.projections.len() < target.projections.len();
                    if mode == 0 || ancestor {
                        return Err(error(
                            line,
                            "E_MOVE",
                            &format!(
                                "{} contains {} which may have been moved",
                                place_name(types, function, place),
                                place_name(types, function, moved)
                            ),
                        ));
                    }
                }
            }
        }
    }
    let value_ty = place_type(types, function, place).unwrap();
    let view_owner =
        mode != 3 && state.views[value_ty] && !matches!(types[value_ty].kind, TypeKind::Ref { .. });
    let mut candidates = BTreeSet::new();
    for target in &targets {
        for loan in state.loans.for_target(&target.root, !write) {
            candidates.insert(loan.holder.clone());
        }
        if write && view_owner {
            if let Root::Local(local) = target.root {
                if let Some(holders) = state.loans.parents.get(&local) {
                    candidates.extend(holders.iter().cloned());
                }
            }
        }
    }
    for parent in &parents {
        for loan in state.loans.for_target(&Root::Local(parent.local), !write) {
            candidates.insert(loan.holder.clone());
        }
    }
    for holder in candidates {
        let loan = state.loans.get(&holder).unwrap();
        if !loan_active(state, loan) || loan.holder == *place || parents.contains(&loan.holder) {
            continue;
        }
        if (write || loan.mutable)
            && targets
                .iter()
                .any(|target| source_overlaps(&loan.sources, target))
        {
            return Err(error(
                line,
                "E_BORROW",
                &format!(
                    "{} conflicts with {}",
                    place_name(types, function, place),
                    loan_description(types, function, loan)
                ),
            ));
        }
        for parent in &parents {
            if (write || loan.mutable) && source_overlaps(&loan.sources, &holder_origin(parent)) {
                return Err(error(
                    line,
                    "E_BORROW",
                    &format!(
                        "{} uses reference storage frozen by {}",
                        place_name(types, function, place),
                        loan_description(types, function, loan)
                    ),
                ));
            }
        }
        if write && view_owner && !within(&loan.holder, place) {
            for parent in &loan.parents {
                if targets
                    .iter()
                    .any(|target| overlaps(&holder_origin(parent), target))
                {
                    return Err(error(
                        line,
                        "E_BORROW",
                        &format!(
                            "{} is still borrowed through {}",
                            place_name(types, function, place),
                            loan_description(types, function, loan)
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}
fn place_name(types: &[Type], function: &Function, place: &Place) -> String {
    let local = &function.locals[place.local];
    let mut name = if local.name.is_empty() {
        format!("local {}", place.local)
    } else {
        local.name.clone()
    };
    let mut ty = local.ty;
    for projection in &place.projections {
        match (projection, &types[ty].kind) {
            (Projection::Deref, TypeKind::Ref { inner, .. } | TypeKind::Box { inner }) => {
                name.push_str(".*");
                ty = *inner;
            }
            (Projection::Field(index), TypeKind::Struct { fields, .. }) => {
                name.push('.');
                name.push_str(&fields[*index].name);
                ty = fields[*index].ty;
            }
            (Projection::VariantField(variant, index), TypeKind::Enum { variants, .. }) => {
                name.push_str("::");
                name.push_str(&variants[*variant].name);
                name.push('.');
                name.push_str(&variants[*variant].fields[*index].name);
                ty = variants[*variant].fields[*index].ty;
            }
            (
                Projection::Index(index),
                TypeKind::Vec { inner }
                | TypeKind::Array { inner, .. }
                | TypeKind::Slice { inner, .. },
            ) => {
                name.push('[');
                name.push_str(&function.locals[*index].name);
                name.push(']');
                ty = *inner;
            }
            _ => {
                name.push_str(".<invalid>");
                break;
            }
        }
    }
    name
}
fn consume(
    state: &mut State,
    types: &[Type],
    function: &Function,
    local: usize,
    line: usize,
) -> Result<(), Diagnostic> {
    access(state, types, function, &Place::local(local), true, 0, line)?;
    forget(state, local);
    Ok(())
}
fn forget(state: &mut State, local: usize) {
    state.live.set(local, false);
    state.maybe_live.set(local, false);
    remove_provenance(state, &Place::local(local));
    state.moved.retain(|place| place.local != local);
    state.definitely_moved.retain(|place| place.local != local);
    invalidate(state, &[Origin::local(local)]);
    state.predicates.remove(&local);
}
fn put(state: &mut State, local: usize, line: usize) -> Result<(), Diagnostic> {
    if state.maybe_live[local] {
        return Err(error(line, "E_K", "destination may still own a value"));
    }
    state.live.set(local, true);
    state.maybe_live.set(local, true);
    state.predicates.remove(&local);
    invalidate(state, &[Origin::local(local)]);
    Ok(())
}
fn same_set<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len() && a.iter().all(|item| b.contains(item))
}
fn validate_merge(
    a: &State,
    b: &State,
    types: &[Type],
    function: &Function,
    line: usize,
) -> Result<(), Diagnostic> {
    if !a.reachable || !b.reachable {
        return Ok(());
    }
    let changed = a
        .live
        .symmetric_difference(&b.live)
        .union(&a.maybe_live.symmetric_difference(&b.maybe_live));
    if let Some(local) = changed.indices_from(0).next() {
        let place = place_name(types, function, &Place::local(local));
        let message = if a.live[local] && !b.live[local] {
            format!(
                "loop backedge leaves {} uninitialized or moved before the next iteration",
                place
            )
        } else {
            format!(
                "loop backedge changes initialization of {}; the loop entry state must be restored",
                place
            )
        };
        return Err(error(line, "E_MOVE", &message));
    }
    if let Some(place) = a
        .moved
        .iter()
        .chain(&b.moved)
        .find(|place| a.moved.contains(place) != b.moved.contains(place))
        .or_else(|| {
            a.definitely_moved
                .iter()
                .chain(&b.definitely_moved)
                .find(|place| {
                    a.definitely_moved.contains(place) != b.definitely_moved.contains(place)
                })
        })
    {
        return Err(error(
            line,
            "E_MOVE",
            &format!(
                "loop backedge changes initialization of {}; the loop entry state must be restored",
                place_name(types, function, place)
            ),
        ));
    }
    if a.loans != b.loans || !same_set(&a.absent_references, &b.absent_references) {
        return Err(error(
            line,
            "E_BORROW",
            "loop backedge changes the reference holders or origins required at loop entry",
        ));
    }
    Ok(())
}
fn merge(mut a: State, b: State) -> State {
    if !a.reachable {
        return b;
    }
    if !b.reachable {
        return a;
    }
    // A parameter carries templates for every enum alternative. When one
    // branch moved a holder and the other proves it absent, or both prove it
    // inactive, their join has no live reference at that leaf. Keep explicit
    // absence after the branch tags disappear. A genuinely active holder on
    // either incoming path retains the ordinary conservative union.
    let absent: Vec<_> = a
        .loans
        .values
        .values()
        .filter(|loan| {
            !loan_active(&a, loan)
                && b.loans
                    .get(&loan.holder)
                    .is_none_or(|other| !loan_active(&b, other))
        })
        .map(|loan| loan.holder.clone())
        .collect();
    for holder in absent {
        a.loans.remove(&holder);
        a.absent_references.push(holder);
    }
    for loan in b.loans.values.values() {
        if a.loans.get(&loan.holder).is_none() && !loan_active(&b, loan) {
            a.absent_references.push(loan.holder.clone());
            continue;
        }
        let mut joined = if let Some(previous) = a.loans.get(&loan.holder) {
            previous.clone()
        } else {
            loan.clone()
        };
        joined.effects =
            scoped::join_permissions(&a.effects, &b.effects, a.loans.get(&loan.holder), loan);
        joined.sources.extend_from_slice(&loan.sources);
        normalize_sources(&mut joined.sources);
        joined.parents.extend_from_slice(&loan.parents);
        joined.parents.sort();
        joined.parents.dedup();
        joined.created.extend_from_slice(&loan.created);
        joined.created.sort();
        joined.created.dedup();
        a.loans.push(joined);
    }
    a.absent_references.extend_from_slice(&b.absent_references);
    a.absent_references.sort();
    a.absent_references.dedup();
    a.live = a.live.intersection(&b.live);
    a.maybe_live = a.maybe_live.union(&b.maybe_live);
    for place in &b.moved {
        if !a.moved.contains(place) {
            a.moved.push(place.clone());
        }
    }
    a.definitely_moved
        .retain(|place| b.definitely_moved.contains(place));
    a.effects.join(&b.effects);
    a.tags.retain(|tag| b.tags.contains(tag));
    a.predicates
        .retain(|local, tag| b.predicates.get(local) == Some(tag));
    a
}
fn local_type(function: &Function, local: usize, line: usize) -> Result<usize, Diagnostic> {
    function
        .locals
        .get(local)
        .map(|l| l.ty)
        .ok_or_else(|| error(line, "E_K", "local index is not declared"))
}
fn enum_place(types: &[Type], function: &Function, mut place: Place) -> Place {
    if matches!(
        types[place_type(types, function, &place).unwrap()].kind,
        TypeKind::Ref { .. }
    ) {
        place.projections.push(Projection::Deref);
    }
    place
}
fn crosses_destructor(
    program: &Program,
    facts: &TypeFacts,
    function: &Function,
    place: &Place,
) -> bool {
    let mut prefix = Place::local(place.local);
    for projection in &place.projections {
        if facts.destructors[place_type(&program.types, function, &prefix).unwrap()] {
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
        "world_stdout" => {
            ts.len() == 1
                && matches!(types[dt].kind, TypeKind::Stdout)
                && matches!(types[ts[0]].kind, TypeKind::Ref { inner, mutable: true }
                if matches!(types[inner].kind, TypeKind::World))
        }
        "stdout_write" => {
            ts.len() == 2
                && dt == UNIT
                && bytes_ref(ts[1])
                && matches!(types[ts[0]].kind, TypeKind::Ref { inner, mutable: true }
                if matches!(types[inner].kind, TypeKind::Stdout))
        }
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
                    (TypeKind::Bool | TypeKind::Char, TypeKind::Int { .. }) => true,
                    (
                        TypeKind::Int {
                            bits: 8,
                            signed: false,
                        },
                        TypeKind::Char,
                    )
                    | (TypeKind::Char, TypeKind::Char) => true,
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
    facts: &TypeFacts,
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
            TypeKind::Char
                if u32::try_from(*value)
                    .ok()
                    .and_then(char::from_u32)
                    .is_some() => {}
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
            if *moving && crosses_destructor(program, facts, function, place) {
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
        Op::ViewRange {
            dst,
            place,
            start,
            end,
            mutable,
            ..
        } => {
            if ty(*start)? != U64 || ty(*end)? != U64 {
                return Err(error(line, "E_TYPE", "range bounds must be usize"));
            }
            let source = place_ty(place)?;
            let referent = match types[ty(*dst)?].kind {
                TypeKind::Ref {
                    inner,
                    mutable: expected,
                } if expected == *mutable => inner,
                _ => {
                    return Err(error(
                        line,
                        "E_TYPE",
                        "range result needs a matching reference type",
                    ));
                }
            };
            match types[source].kind {
                TypeKind::Vec { inner }
                | TypeKind::Array { inner, .. }
                | TypeKind::Slice { inner, .. } => {
                    if !matches!(types[referent].kind, TypeKind::Slice { inner: element, .. } if inner == element)
                    {
                        return Err(error(line, "E_TYPE", "range result element type differs"));
                    }
                }
                TypeKind::Str if !*mutable && matches!(types[referent].kind, TypeKind::Str) => {}
                TypeKind::Str => {
                    return Err(error(
                        line,
                        "E_TYPE",
                        "text ranges require a shared str result",
                    ));
                }
                _ => {
                    return Err(error(
                        line,
                        "E_TYPE",
                        "range source needs Vec, array, slice or str",
                    ));
                }
            }
            if *mutable && !place_writable(types, function, place) {
                return Err(error(line, "E_BORROW", "mutable range of immutable place"));
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
        Op::Drop { place } | Op::DropIf { place } => {
            let _ = place_ty(place)?;
            if borrowed_projection(types, function, place) {
                return Err(error(
                    line,
                    "E_BORROW",
                    &format!(
                        "cannot drop {} through a borrow or indexed collection",
                        place_name(types, function, place)
                    ),
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
                TypeKind::Char => comparison,
                TypeKind::Int { .. } => [
                    "+",
                    "-",
                    "*",
                    "/",
                    "%",
                    "&",
                    "|",
                    "^",
                    "<<",
                    ">>",
                    "==",
                    "!=",
                    "<",
                    "<=",
                    ">",
                    ">=",
                    "wrapping_add",
                    "wrapping_sub",
                    "wrapping_mul",
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
        Op::Discriminant { dst, .. } | Op::TagTest { dst, .. } => {
            let st = match &instruction.op {
                Op::Discriminant { src, .. } => ty(*src)?,
                Op::TagTest { place, .. } => place_ty(place)?,
                _ => unreachable!(),
            };
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
        Op::Else
        | Op::End
        | Op::Loop
        | Op::Break
        | Op::Continue
        | Op::BreakTo { .. }
        | Op::ContinueTo { .. }
        | Op::Trap { .. } => {}
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
    let mut used: BTreeMap<Place, (Loan, bool)> = BTreeMap::new();
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
            let targets = origin(state, &place).0;
            if mutable {
                changed.extend_from_slice(&targets);
            }
            for target in &targets {
                if let Root::Local(local) = target.root {
                    let base = Place {
                        local,
                        projections: target.projections.clone(),
                    };
                    for loan in state.loans.for_holder(local) {
                        if within(&loan.holder, &base) {
                            let entry = used
                                .entry(loan.holder.clone())
                                .or_insert((loan.clone(), false));
                            entry.1 |= mutable && loan.mutable;
                        }
                    }
                }
            }
        } else {
            for loan in state.loans.for_holder(*arg) {
                let entry = used
                    .entry(loan.holder.clone())
                    .or_insert((loan.clone(), false));
                entry.1 |= loan.mutable;
            }
        }
    }
    for (loan, mutable) in used.values() {
        if !loan_active(state, loan) {
            continue;
        }
        loan_use(state, types, function, loan, *mutable, line)?;
        if *mutable {
            changed.extend(source_origins(&loan.sources));
        }
    }
    invalidate(state, &changed);
    Ok(())
}

#[derive(Default)]
struct ReturnObservation {
    inputs: BTreeMap<InputLeaf, Vec<SourceBound>>,
    outputs: BTreeMap<Vec<Projection>, Vec<SourceBound>>,
}

fn check_function(
    program: &Program,
    function: &Function,
    facts: &TypeFacts,
) -> Result<(), Diagnostic> {
    check_function_scoped(program, function, facts, None)
}
fn check_function_scoped(
    program: &Program,
    function: &Function,
    facts: &TypeFacts,
    scope: Option<&scoped::Scope>,
) -> Result<(), Diagnostic> {
    let mut site = None;
    check_function_body(program, function, facts, &mut site, None, scope).map_err(|mut problem| {
        if let Some((line, column)) = site {
            if problem.line == line {
                problem.column = column;
            }
        }
        problem
    })
}
fn check_function_body(
    program: &Program,
    function: &Function,
    facts: &TypeFacts,
    site: &mut Option<(usize, usize)>,
    mut observation: Option<&mut ReturnObservation>,
    scope: Option<&scoped::Scope>,
) -> Result<(), Diagnostic> {
    let types = &program.types;
    let mut state = State {
        live: Facts::new(function.locals.len()),
        maybe_live: Facts::new(function.locals.len()),
        moved: Vec::new(),
        definitely_moved: Vec::new(),
        views: facts.views.clone(),
        site: None,
        loans: Loans::default(),
        effects: scoped::Effects::default(),
        absent_references: Vec::new(),
        tags: Vec::new(),
        predicates: BTreeMap::new(),
        reachable: true,
    };
    for i in 0..function.params {
        state.live.set(i, true);
        state.maybe_live.set(i, true);
        for (path, mutable) in reference_paths(types, function.locals[i].ty) {
            let mut projections = path.clone();
            if !path.is_empty() {
                projections.push(Projection::Deref);
            }
            let target = Origin {
                root: Root::Parameter(i),
                projections,
            };
            state.loans.push(Loan {
                created: state.site.into_iter().collect(),
                holder: Place {
                    local: i,
                    projections: path,
                },
                sources: vec![SourceBound {
                    origin: target.clone(),
                    exact: true,
                }],
                mutable,
                parents: Vec::new(),
                effects: BTreeSet::new(),
            });
        }
    }
    let input_regions = scoped::InputRegions::new(&state, types, function, scope)?;
    // Keep the exact permitted source leaves. A common root alone would allow
    // a borrow of an unrelated scalar field to impersonate a stored reference.
    let return_origins = match &function.return_source {
        Some(ReturnSource::Parameter { index, projections }) => {
            let place = parameter_source(types, function, *index, projections).unwrap();
            carried_provenance(&state, types, function, &place, &place, 1)?
                .loans
                .into_iter()
                .flat_map(|loan| loan.sources.into_iter().map(|source| source.origin))
                .collect::<Vec<_>>()
        }
        _ => Vec::new(),
    };
    let mut leaf_origins = BTreeMap::new();
    if let Some(ReturnSource::Leaves(leaves)) = &function.return_source {
        for leaf in leaves {
            if let LeafOrigin::Inputs { inputs, .. } = &leaf.origin {
                let mut sources = Vec::new();
                for selector in inputs {
                    let input = parameter_leaf_source(
                        types,
                        function,
                        selector.index,
                        &selector.projections,
                    )
                    .unwrap();
                    let value = carried_provenance(&state, types, function, &input, &input, 1)?;
                    for loan in value.loans {
                        if let Some(observation) = observation.as_deref_mut() {
                            observation
                                .inputs
                                .entry(selector.clone())
                                .or_default()
                                .extend_from_slice(&loan.sources);
                        }
                        sources.extend(loan.sources);
                    }
                }
                normalize_sources(&mut sources);
                leaf_origins.insert(leaf.projections.clone(), sources);
            }
        }
    }
    let mut controls: Vec<Control> = Vec::new();
    let mut pc = 0;
    while pc < function.body.len() {
        let instruction_index = pc;
        let instruction = &function.body[pc];
        pc += 1;
        let line = instruction.line;
        *site = Some((line, instruction.column));
        state.site = *site;
        check_shape(program, facts, function, instruction)?;
        match &instruction.op {
            Op::Else => {
                let control = controls
                    .last_mut()
                    .ok_or_else(|| error(line, "E_K", "else outside if"))?;
                if control.is_loop || control.then_state.is_some() {
                    return Err(error(line, "E_K", "invalid else"));
                }
                control.then_state = Some(std::mem::replace(&mut state, control.entry.clone()));
                continue;
            }
            Op::End => {
                let mut control = controls
                    .pop()
                    .ok_or_else(|| error(line, "E_K", "end outside control"))?;
                if control.is_loop {
                    if state.reachable {
                        validate_merge(&control.entry, &state, types, function, line)?;
                        if control.entry.effects.join(&state.effects) {
                            // There are finitely many call-site/leaf effects and
                            // external sources; a loop only adds their unions.
                            // Recheck earlier accesses with every backedge effect.
                            state = control.entry.clone();
                            control.breaks.clear();
                            pc = control.start;
                            controls.push(control);
                            continue;
                        }
                    }
                    state.reachable = false;
                    for exit in control.breaks {
                        state = merge(state, exit);
                    }
                } else {
                    state = merge(state, control.then_state.unwrap_or(control.entry));
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
                    start: pc,
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
                    start: pc,
                    breaks: Vec::new(),
                });
                continue;
            }
            Op::Break | Op::Continue | Op::BreakTo { .. } | Op::ContinueTo { .. } => {
                let depth = match instruction.op {
                    Op::BreakTo { depth } | Op::ContinueTo { depth } => depth,
                    _ => 0,
                };
                if controls
                    .iter()
                    .rev()
                    .filter(|control| control.is_loop)
                    .nth(depth)
                    .is_none()
                {
                    return Err(error(
                        line,
                        "E_K",
                        "loop transfer target is not an enclosing loop",
                    ));
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
                    created: state.site.into_iter().collect(),
                    holder: Place::local(*dst),
                    sources: vec![SourceBound {
                        origin: Origin {
                            root: Root::Static,
                            projections: Vec::new(),
                        },
                        exact: true,
                    }],
                    mutable: false,
                    parents: Vec::new(),
                    effects: BTreeSet::new(),
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
                        state.definitely_moved.push(place.clone());
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
                            target: vec![Origin::local(*dst)],
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
                let resolved = resolve_origin(&state, place);
                put(&mut state, *dst, line)?;
                state.loans.push(Loan {
                    created: state.site.into_iter().collect(),
                    holder: Place::local(*dst),
                    sources: resolved.sources,
                    mutable: *mutable,
                    parents: resolved.parents,
                    effects: resolved.effects,
                });
            }
            Op::ViewRange {
                dst,
                place,
                start,
                end,
                mutable,
                ..
            } => {
                access(&state, types, function, place, *mutable, 0, line)?;
                let mut resolved = resolve_origin(&state, place);
                consume(&mut state, types, function, *start, line)?;
                consume(&mut state, types, function, *end, line)?;
                put(&mut state, *dst, line)?;
                for source in &mut resolved.sources {
                    source.exact = false;
                }
                state.loans.push(Loan {
                    created: state.site.into_iter().collect(),
                    holder: Place::local(*dst),
                    sources: resolved.sources,
                    mutable: *mutable,
                    parents: resolved.parents,
                    effects: resolved.effects,
                });
            }
            Op::Assign { place, src } => {
                if !place.projections.is_empty() || state.maybe_live[place.local] {
                    access(
                        &state,
                        types,
                        function,
                        place,
                        true,
                        if place.projections.is_empty() { 2 } else { 1 },
                        line,
                    )?;
                }
                if state.maybe_live[place.local]
                    && place.projections.is_empty()
                    && !function.locals[place.local].mutable
                {
                    return Err(error(line, "E_TYPE", "assignment to an immutable local"));
                }
                let resolved = resolve_origin(&state, place);
                let targets = source_origins(&resolved.sources);
                let absent = if place.projections.is_empty() {
                    !state.maybe_live[place.local]
                } else {
                    state.definitely_moved.iter().any(|moved| moved == place)
                };
                let pt = place_type(types, function, place).unwrap();
                if !absent && heap_type(&types[pt].kind) {
                    return Err(error(
                        line,
                        "E_K",
                        "replacement of an owned value needs an explicit drop",
                    ));
                }
                if !resolved.exact() && is_view(types, pt) {
                    return Err(error(
                        line,
                        "E_BORROW",
                        "reference-bearing write through an opaque call needs an exact destination contract",
                    ));
                }
                let source = Place::local(*src);
                let destination = resolved.storage(place);
                let mut carried =
                    carried_provenance(&state, types, function, &source, &destination, line)?;
                for target in &targets {
                    check_external_write(target, &destination, &carried, line)?;
                }
                let known_tag = tag_at(&state, &[Origin::local(*src)], &Place::local(*src));
                let predicate = state.predicates.get(src).cloned();
                remap_parents(&mut state, &source, &destination);
                remap_carried(&mut carried.loans, &source, &destination);
                consume(&mut state, types, function, *src, line)?;
                invalidate(&mut state, &targets);
                state.live.set(place.local, true);
                state.maybe_live.set(place.local, true);
                state.definitely_moved.retain(|moved| {
                    !(moved.local == place.local
                        && is_prefix(&place.projections, &moved.projections))
                });
                state.moved.retain(|moved| {
                    !(moved.local == place.local
                        && is_prefix(&place.projections, &moved.projections))
                });
                if resolved.exact() && matches!(targets[0].root, Root::Local(_)) {
                    remove_provenance(&mut state, &destination);
                    install_provenance(&mut state, carried);
                    validate_stored_view(&state, &targets[0], line)?;
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
                            target: targets.clone(),
                            witness: place.clone(),
                            variant,
                        },
                    );
                }
            }
            Op::Drop { place } | Op::DropIf { place } => {
                let conditional = matches!(instruction.op, Op::DropIf { .. });
                if !state.live[place.local]
                    && !(conditional
                        && place.projections.is_empty()
                        && state.maybe_live[place.local])
                {
                    if conditional && place.projections.is_empty() {
                        continue;
                    }
                    return Err(error(
                        line,
                        "E_MOVE",
                        &format!(
                            "cannot drop {}: its owner is uninitialized or moved",
                            place_name(types, function, place)
                        ),
                    ));
                }
                if let Some(moved) = state.moved.iter().find(|moved| within(place, moved)) {
                    if !conditional || moved != place {
                        return Err(error(
                            line,
                            "E_MOVE",
                            &format!(
                                "cannot drop {}: {} was moved or already dropped",
                                place_name(types, function, place),
                                place_name(types, function, moved)
                            ),
                        ));
                    }
                }
                // Projection still requires every ancestor. Conditional cleanup
                // may skip only this exact absent place, never invent its owner.
                access(
                    &state,
                    types,
                    function,
                    place,
                    true,
                    if state.views[place_type(types, function, place).unwrap()]
                        && facts.release_shells[place_type(types, function, place).unwrap()]
                    {
                        3
                    } else if conditional {
                        2
                    } else {
                        1
                    },
                    line,
                )?;
                if conditional && state.definitely_moved.iter().any(|moved| moved == place) {
                    continue;
                }
                let pt = place_type(types, function, place).unwrap();
                if facts.destructors[pt] {
                    if state
                        .moved
                        .iter()
                        .any(|moved| moved != place && within(moved, place))
                    {
                        return Err(error(
                            line,
                            "E_MOVE",
                            "custom drop requires every field to be initialized",
                        ));
                    }
                    for loan in state.loans.for_holder(place.local) {
                        if within(&loan.holder, place) {
                            loan_use(&state, types, function, loan, loan.mutable, line)?;
                        }
                    }
                }
                remove_provenance(&mut state, place);
                if place.projections.is_empty() {
                    forget(&mut state, place.local);
                } else {
                    let target = origin(&state, place).0;
                    invalidate(&mut state, &target);
                    state.moved.retain(|moved| {
                        !(moved.local == place.local
                            && is_prefix(&place.projections, &moved.projections))
                    });
                    state.moved.push(place.clone());
                    state.definitely_moved.retain(|moved| !within(moved, place));
                    state.definitely_moved.push(place.clone());
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
                let call = scoped::CallContext {
                    state: &state,
                    function,
                    types,
                    instruction: instruction_index,
                    args,
                };
                let retained = input_regions.before_call(scope, &call)?;
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
                    Some(ReturnSource::Static) => Some(ResolvedOrigin {
                        absent: false,
                        sources: vec![SourceBound {
                            origin: Origin {
                                root: Root::Static,
                                projections: Vec::new(),
                            },
                            exact: false,
                        }],
                        parents: Vec::new(),
                        effects: BTreeSet::new(),
                    }),
                    None | Some(ReturnSource::Leaves(_)) => None,
                };
                if let Some(ReturnSource::Leaves(_)) = &callee.return_source {
                    returned =
                        mapped_call_provenance(&state, types, function, callee, args, *dst, line)?;
                } else {
                    for (projections, mutable) in reference_paths(types, callee.result) {
                        let holder = Place {
                            local: *dst,
                            projections,
                        };
                        if let Some(source) = &source {
                            if mutable
                                && source
                                    .sources
                                    .iter()
                                    .any(|bound| bound.origin.root == Root::Static)
                            {
                                return Err(error(
                                    line,
                                    "E_BORROW",
                                    "static storage cannot supply an exclusive result",
                                ));
                            }
                            // A call may select any descendant of its declared
                            // input sources. No positional result-leaf mapping is
                            // established by this contract.
                            let mut bounds = Vec::new();
                            for source in &source.sources {
                                let bound = SourceBound {
                                    origin: source.origin.clone(),
                                    exact: false,
                                };
                                if !bounds.contains(&bound) {
                                    bounds.push(bound);
                                }
                            }
                            returned.loans.push(Loan {
                                created: state.site.into_iter().collect(),
                                holder,
                                sources: bounds,
                                mutable,
                                parents: source.parents.clone(),
                                effects: source.effects.clone(),
                            });
                        } else if !mutable && callee.return_source.is_some() {
                            // With no live input reference leaves, an initialized
                            // shared result can only come from immutable static
                            // storage. The callee's return check proves this case.
                            let target = Origin {
                                root: Root::Static,
                                projections: Vec::new(),
                            };
                            returned.loans.push(Loan {
                                created: state.site.into_iter().collect(),
                                holder,
                                sources: vec![SourceBound {
                                    origin: target.clone(),
                                    exact: false,
                                }],
                                mutable: false,
                                parents: Vec::new(),
                                effects: BTreeSet::new(),
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
                }
                retained.permit_results(&call, callee, &mut returned)?;
                call_borrows(&mut state, types, function, args, line)?;
                for arg in args {
                    consume(&mut state, types, function, *arg, line)?;
                }
                put(&mut state, *dst, line)?;
                retained.install(&mut state);
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
                            target: vec![Origin::local(*dst)],
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
            Op::Discriminant { dst, .. } | Op::TagTest { dst, .. } => {
                let source = match &instruction.op {
                    Op::Discriminant { src, .. } => Place::local(*src),
                    Op::TagTest { place, .. } => place.clone(),
                    _ => unreachable!(),
                };
                let place = enum_place(types, function, source);
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
                    let resolved = resolve_origin(&state, &place);
                    if name == "replace"
                        && !resolved.exact()
                        && is_view(types, function.locals[*dst].ty)
                    {
                        return Err(error(
                            line,
                            "E_BORROW",
                            "reference-bearing write through an opaque call needs an exact destination contract",
                        ));
                    }
                    let targets = source_origins(&resolved.sources);
                    let stored = resolved.storage(&place);
                    let destination = Place::local(*dst);
                    let mut returned =
                        carried_provenance(&state, types, function, &place, &destination, line)?;
                    let old_tag = tag_at(&state, &targets, &stored);
                    let incoming = if name == "replace" {
                        let source = Place::local(args[1]);
                        let value =
                            carried_provenance(&state, types, function, &source, &stored, line)?;
                        for target in &targets {
                            check_external_write(target, &stored, &value, line)?;
                        }
                        let tag = tag_at(&state, &[Origin::local(args[1])], &source);
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
                        if resolved.exact() && matches!(targets[0].root, Root::Local(_)) {
                            remove_provenance(&mut state, &stored);
                            install_provenance(&mut state, value);
                            validate_stored_view(&state, &targets[0], line)?;
                        }
                        if let Some(variant) = tag {
                            set_tag(
                                &mut state,
                                Tag {
                                    target: targets.clone(),
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
                                target: vec![Origin::local(*dst)],
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
                    state
                        .effects
                        .check(&loan.sources, &loan.effects, loan.mutable, line)?;
                    let mapped = match &function.return_source {
                        Some(ReturnSource::Leaves(leaves)) => leaves
                            .iter()
                            .find(|leaf| leaf.projections == loan.holder.projections),
                        _ => None,
                    };
                    let allowed = if let Some(leaf) = mapped {
                        match &leaf.origin {
                            LeafOrigin::Static => {
                                !loan.mutable
                                    && loan
                                        .sources
                                        .iter()
                                        .all(|source| source.origin.root == Root::Static)
                            }
                            LeafOrigin::Inputs { exact, .. } => {
                                let expected = &leaf_origins[&leaf.projections];
                                if *exact {
                                    !loan.sources.is_empty()
                                        && loan
                                            .sources
                                            .iter()
                                            .all(|source| expected.contains(source))
                                } else {
                                    !loan.sources.is_empty()
                                        && loan.sources.iter().all(|bound| {
                                            (bound.origin.root == Root::Static && !loan.mutable)
                                                || expected.iter().any(|source| {
                                                    source.origin.root == bound.origin.root
                                                        && is_prefix(
                                                            &source.origin.projections,
                                                            &bound.origin.projections,
                                                        )
                                                })
                                        })
                                }
                            }
                        }
                    } else {
                        !loan.sources.is_empty()
                            && loan.sources.iter().all(|bound| {
                                match (&function.return_source, &bound.origin.root) {
                                    (
                                        Some(ReturnSource::Parameter { index, .. }),
                                        Root::Parameter(actual),
                                    ) => {
                                        index == actual
                                            && return_origins.iter().any(|source| {
                                                source.root == bound.origin.root
                                                    && is_prefix(
                                                        &source.projections,
                                                        &bound.origin.projections,
                                                    )
                                            })
                                    }
                                    (
                                        Some(ReturnSource::Parameter { .. } | ReturnSource::Static),
                                        Root::Static,
                                    ) => !loan.mutable,
                                    _ => false,
                                }
                            })
                    };
                    if !allowed {
                        return Err(error(
                            line,
                            "E_BORROW",
                            "returned view differs from its declared source",
                        ));
                    }
                    if let Some(observation) = observation.as_deref_mut() {
                        observation
                            .outputs
                            .entry(loan.holder.projections.clone())
                            .or_default()
                            .extend_from_slice(&loan.sources);
                    }
                }
                consume(&mut state, types, function, *src, line)?;
                if state.maybe_live.any() {
                    return Err(error(
                        line,
                        "E_K",
                        "return leaves a value without an explicit drop",
                    ));
                }
                state.reachable = false;
            }
            Op::Break | Op::Continue | Op::BreakTo { .. } | Op::ContinueTo { .. } => {
                let depth = match instruction.op {
                    Op::BreakTo { depth } | Op::ContinueTo { depth } => depth,
                    _ => 0,
                };
                let index = controls
                    .iter()
                    .enumerate()
                    .rev()
                    .filter(|(_, control)| control.is_loop)
                    .nth(depth)
                    .unwrap()
                    .0;
                let control = &mut controls[index];
                if matches!(instruction.op, Op::Continue | Op::ContinueTo { .. }) {
                    validate_merge(&control.entry, &state, types, function, line)?;
                    if control.entry.effects.join(&state.effects) {
                        state = control.entry.clone();
                        control.breaks.clear();
                        pc = control.start;
                        controls.truncate(index + 1);
                        continue;
                    }
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

// Shared graph facts are computed once per checker invocation. Ref is a
// descriptor boundary: its referent is not part of the owned drop tree.
struct TypeFacts {
    views: Rc<Vec<bool>>,
    destructors: Vec<bool>,
    release_shells: Vec<bool>,
}
impl TypeFacts {
    fn new(program: &Program) -> Self {
        let mut destructors = vec![false; program.types.len()];
        for (ty, _) in &program.destructors {
            destructors[*ty] = true;
        }
        let mut release_shells = vec![false; program.types.len()];
        let mut parents = vec![Vec::new(); program.types.len()];
        let mut pending = Vec::new();
        for (index, ty) in program.types.iter().enumerate() {
            release_shells[index] = !destructors[index]
                && matches!(
                    ty.kind,
                    TypeKind::Unit
                        | TypeKind::Bool
                        | TypeKind::Char
                        | TypeKind::Int { .. }
                        | TypeKind::F64
                        | TypeKind::Ref { .. }
                        | TypeKind::Struct { .. }
                        | TypeKind::Enum { .. }
                );
            if !release_shells[index] {
                pending.push(index);
            }
            if matches!(ty.kind, TypeKind::Struct { .. } | TypeKind::Enum { .. }) {
                for child in direct_children(&ty.kind) {
                    parents[child].push(index);
                }
            }
        }
        while let Some(child) = pending.pop() {
            for parent in &parents[child] {
                if release_shells[*parent] {
                    release_shells[*parent] = false;
                    pending.push(*parent);
                }
            }
        }
        Self {
            views: Rc::new(reference_containment(&program.types)),
            destructors,
            release_shells,
        }
    }
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

fn check_structure(program: &Program) -> Result<(), Diagnostic> {
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
                    ReturnSource::Leaves(leaves) => {
                        valid_leaf_contract(&program.types, function, leaves)
                    }
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
    Ok(())
}

/// Observe a fully validated function's return provenance to refine an
/// already valid explicit leaf contract. This is inference data, never checked
/// program authority. Emission still requires check/check_all on the final K.
fn observed_return_contract(
    program: &Program,
    index: usize,
    facts: &TypeFacts,
) -> Result<Option<ReturnSource>, Diagnostic> {
    let function = &program.functions[index];
    let Some(ReturnSource::Leaves(original)) = &function.return_source else {
        return Ok(function.return_source.clone());
    };
    let mut site = None;
    let mut observation = ReturnObservation::default();
    check_function_body(
        program,
        function,
        facts,
        &mut site,
        Some(&mut observation),
        None,
    )
    .map_err(|mut problem| {
        problem.file = function.file;
        if let Some((line, column)) = site {
            if problem.line == line {
                problem.column = column;
            }
        }
        problem
    })?;
    let mut leaves = original.clone();
    for leaf in &mut leaves {
        let LeafOrigin::Inputs { inputs, .. } = &leaf.origin else {
            continue;
        };
        let Some(actual) = observation.outputs.get(&leaf.projections) else {
            continue;
        };
        if actual
            .iter()
            .all(|source| source.origin.root == Root::Static)
        {
            leaf.origin = LeafOrigin::Static;
            continue;
        }
        let output = projected_type(&program.types, function.result, &leaf.projections).unwrap();
        let TypeKind::Ref {
            inner: output_inner,
            ..
        } = program.types[output].kind
        else {
            unreachable!()
        };
        let mut selected = BTreeSet::new();
        let mut exact = true;
        for source in actual {
            if source.origin.root == Root::Static {
                exact = false;
                continue;
            }
            let mut best = None;
            for input in inputs {
                if let Some(bounds) = observation.inputs.get(input) {
                    for bound in bounds {
                        if bound.origin.root == source.origin.root
                            && is_prefix(&bound.origin.projections, &source.origin.projections)
                            && best.as_ref().is_none_or(
                                |(_, prior): &(&InputLeaf, &SourceBound)| {
                                    bound.origin.projections.len() > prior.origin.projections.len()
                                },
                            )
                        {
                            best = Some((input, bound));
                        }
                    }
                }
            }
            let Some((input, bound)) = best else {
                return Err(error(
                    1,
                    "E_BORROW",
                    "return observation is outside its validated input contract",
                ));
            };
            let input_ty = place_type(
                &program.types,
                function,
                &Place {
                    local: input.index,
                    projections: input.projections.clone(),
                },
            )
            .unwrap();
            let TypeKind::Ref {
                inner: input_inner, ..
            } = program.types[input_ty].kind
            else {
                unreachable!()
            };
            exact &= source == bound && output_inner == input_inner;
            selected.insert(input.clone());
        }
        if !selected.is_empty() {
            leaf.origin = LeafOrigin::Inputs {
                inputs: selected.into_iter().collect(),
                exact,
            };
        }
    }
    Ok(Some(ReturnSource::Leaves(leaves)))
}

/// Complete inference for generated callable bodies after their shared graph
/// exists. Each body is fully validated before observation; this function never
/// creates CheckedProgram. Shared graph validation and reference containment are
/// computed once, and the ordinary final gate checks every refined contract.
pub(crate) fn refine_return_contracts(
    program: &mut Program,
    functions: &BTreeSet<usize>,
) -> Vec<Diagnostic> {
    let selected: BTreeSet<_> = functions
        .iter()
        .copied()
        .filter(|index| program.functions[*index].return_source.is_some())
        .collect();
    if selected.is_empty() {
        return Vec::new();
    }
    if let Err(problem) = check_structure(program) {
        return vec![problem];
    }
    let facts = TypeFacts::new(program);
    // A body may rely on a callee's validated mapping. Complete acyclic
    // dependencies first; recursive groups keep their conservative contracts.
    // The final independent gate still validates every function in the graph.
    let mut callers = BTreeMap::<usize, Vec<usize>>::new();
    let mut dependencies = BTreeMap::<usize, usize>::new();
    for index in &selected {
        let called: BTreeSet<_> = program.functions[*index]
            .body
            .iter()
            .filter_map(|instruction| match instruction.op {
                Op::Call { function, .. } if selected.contains(&function) => Some(function),
                _ => None,
            })
            .collect();
        dependencies.insert(*index, called.len());
        for callee in called {
            callers.entry(callee).or_default().push(*index);
        }
    }
    let mut ready: BTreeSet<_> = dependencies
        .iter()
        .filter_map(|(index, count)| (*count == 0).then_some(*index))
        .collect();
    let mut problems = Vec::new();
    while let Some(index) = ready.pop_first() {
        match observed_return_contract(program, index, &facts) {
            Ok(source) => program.functions[index].return_source = source,
            Err(problem) => problems.push(problem),
        }
        for caller in callers.get(&index).into_iter().flatten() {
            let remaining = dependencies.get_mut(caller).unwrap();
            *remaining -= 1;
            if *remaining == 0 {
                ready.insert(*caller);
            }
        }
    }
    problems
}

/// The shared type/signature graph must be valid before independent body checks.
/// An error list never contains CheckedProgram authority and cannot reach emit.
pub fn check_all(program: Program) -> Result<CheckedProgram, Vec<Diagnostic>> {
    check_structure(&program).map_err(|problem| vec![problem])?;
    let mut problems = Vec::new();
    let facts = TypeFacts::new(&program);
    for function in &program.functions {
        if let Err(mut problem) = check_function(&program, function, &facts) {
            problem.file = function.file;
            problems.push(problem);
        }
    }
    if problems.is_empty() {
        Ok(CheckedProgram { program })
    } else {
        Err(problems)
    }
}

pub fn check(program: Program) -> Result<CheckedProgram, Diagnostic> {
    check_all(program).map_err(|problems| {
        problems
            .into_iter()
            .next()
            .expect("failed K check has a diagnostic")
    })
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
        Instruction {
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
    fn broad_return_contract_cannot_hide_a_second_parameter_root() {
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
                place: Place::local(0),
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
                "{} at {}:{}: {}",
                failure.code,
                failure.line,
                failure.column,
                failure.message
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
