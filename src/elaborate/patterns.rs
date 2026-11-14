//! Typed pattern plans share their source subtrees. Testing precedes binding:
//! no failed test or failed guard moves a payload from the scrutinee.
use super::pattern_choices::PatternChoices;
use super::{Builder, Result, a, err, integers, ir};
use std::collections::{BTreeMap, BTreeSet};

type PatternConstructor = (Option<usize>, Vec<(ir::Projection, usize)>);
type PatternMatrix = (Vec<Vec<Option<usize>>>, Vec<usize>);
type CoverageMemo = BTreeMap<PatternMatrix, bool>;
type BindingSignature = BTreeMap<String, (usize, bool, Option<bool>)>;

#[derive(Clone, Copy)]
pub(super) struct IntegerRange {
    start: Option<i64>,
    end: Option<i64>,
    inclusive: bool,
}
#[derive(Clone)]
pub(super) enum PatternKind {
    Any,
    Bind {
        name: String,
        mutable: bool,
        borrow: Option<bool>,
    },
    Literal(usize),
    Range(IntegerRange),
    Product(Vec<(ir::Projection, usize)>),
    Variant {
        tag: usize,
        fields: Vec<(ir::Projection, usize)>,
    },
    Deref(usize),
    Or(Vec<usize>),
}
#[derive(Clone)]
pub(super) struct PatternNode {
    pub(super) ty: usize,
    pub(super) span: a::Span,
    pub(super) kind: PatternKind,
}
#[derive(Default)]
pub(super) struct PatternPlan {
    pub(super) nodes: Vec<PatternNode>,
}
#[derive(Clone)]
pub(super) struct PatternBinding {
    pub(super) name: String,
    pub(super) mutable: bool,
    pub(super) borrow: Option<bool>,
    pub(super) ty: usize,
    pub(super) place: ir::Place,
}
pub(super) struct PatternGuardInput<'a> {
    pub(super) expression: usize,
    pub(super) condition: usize,
    pub(super) tested: &'a [ir::Place],
    pub(super) choices: Option<&'a PatternChoices>,
}
struct BindingTargets<'a> {
    destinations: &'a BTreeMap<String, usize>,
    guard: bool,
    choices: Option<&'a PatternChoices>,
}
impl PatternPlan {
    fn add(&mut self, ty: usize, span: a::Span, kind: PatternKind) -> usize {
        let id = self.nodes.len();
        self.nodes.push(PatternNode { ty, span, kind });
        id
    }
    pub(super) fn has_or(&self, root: usize) -> bool {
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            match &self.nodes[id].kind {
                PatternKind::Or(_) => return true,
                PatternKind::Product(fields) | PatternKind::Variant { fields, .. } => {
                    for (_, id) in fields {
                        pending.push(*id);
                    }
                }
                PatternKind::Deref(inner) => pending.push(*inner),
                _ => {}
            }
        }
        false
    }
    pub(super) fn tested_places(&self, root: usize, place: ir::Place) -> Vec<ir::Place> {
        let mut tested = Vec::new();
        let mut pending = vec![(root, place)];
        while let Some((id, place)) = pending.pop() {
            match &self.nodes[id].kind {
                PatternKind::Literal(_) | PatternKind::Range(_) => tested.push(place),
                PatternKind::Product(fields) | PatternKind::Variant { fields, .. } => {
                    if matches!(self.nodes[id].kind, PatternKind::Variant { .. }) {
                        tested.push(place.clone());
                    }
                    for (projection, child) in fields {
                        let mut field = place.clone();
                        field.projections.push(projection.clone());
                        pending.push((*child, field));
                    }
                }
                PatternKind::Deref(inner) => {
                    let mut field = place;
                    field.projections.push(ir::Projection::Deref);
                    pending.push((*inner, field));
                }
                PatternKind::Or(alternatives) => {
                    for id in alternatives {
                        pending.push((*id, place.clone()));
                    }
                }
                _ => {}
            }
        }
        tested
    }
    /// A shared continuation can bind once when every alternative names the
    /// same original places. Walking the graph never expands products of ors.
    pub(super) fn uniform_bindings(&self, root: usize, place: ir::Place) -> bool {
        let mut bindings: BTreeMap<String, PatternBinding> = BTreeMap::new();
        for binding in self.bindings(root, place.clone()) {
            bindings.insert(binding.name.clone(), binding);
        }
        let mut pending = vec![(root, place)];
        while let Some((id, place)) = pending.pop() {
            let node = &self.nodes[id];
            match &node.kind {
                PatternKind::Bind {
                    name,
                    mutable,
                    borrow,
                } => {
                    let Some(binding) = bindings.get(name) else {
                        return false;
                    };
                    if binding.place != place
                        || binding.ty != node.ty
                        || binding.mutable != *mutable
                        || binding.borrow != *borrow
                    {
                        return false;
                    }
                }
                PatternKind::Product(fields) | PatternKind::Variant { fields, .. } => {
                    for (projection, child) in fields {
                        let mut child_place = place.clone();
                        child_place.projections.push(projection.clone());
                        pending.push((*child, child_place));
                    }
                }
                PatternKind::Deref(inner) => {
                    let mut child_place = place;
                    child_place.projections.push(ir::Projection::Deref);
                    pending.push((*inner, child_place));
                }
                PatternKind::Or(alternatives) => {
                    for id in alternatives {
                        pending.push((*id, place.clone()));
                    }
                }
                _ => {}
            }
        }
        true
    }
    pub(super) fn bindings(&self, root: usize, place: ir::Place) -> Vec<PatternBinding> {
        let mut result = Vec::new();
        let mut pending = vec![(root, place)];
        while let Some((id, place)) = pending.pop() {
            let node = &self.nodes[id];
            match &node.kind {
                PatternKind::Bind {
                    name,
                    mutable,
                    borrow,
                } => result.push(PatternBinding {
                    name: name.clone(),
                    mutable: *mutable,
                    borrow: *borrow,
                    ty: node.ty,
                    place,
                }),
                PatternKind::Product(fields) | PatternKind::Variant { fields, .. } => {
                    let mut index = fields.len();
                    while index != 0 {
                        index -= 1;
                        let mut child = place.clone();
                        child.projections.push(fields[index].0.clone());
                        pending.push((fields[index].1, child));
                    }
                }
                PatternKind::Deref(inner) => {
                    let mut child = place;
                    child.projections.push(ir::Projection::Deref);
                    pending.push((*inner, child));
                }
                PatternKind::Or(alternatives) => {
                    if let Some(first) = alternatives.first() {
                        pending.push((*first, place));
                    }
                }
                _ => {}
            }
        }
        result
    }
}
#[derive(Clone, Copy)]
struct Domain {
    maximum: u64,
    sign: u64,
    character: bool,
}
impl Domain {
    fn new(kind: &ir::TypeKind) -> Option<Self> {
        if matches!(kind, ir::TypeKind::Char) {
            return Some(Self {
                maximum: 0x10ffff_u64 - 0x800_u64,
                sign: 0,
                character: true,
            });
        }
        if let ir::TypeKind::Int { bits, signed } = kind {
            Some(Self {
                maximum: if *bits == 64 {
                    !0_u64
                } else {
                    (1u64 << bits) - 1
                },
                sign: if *signed { 1u64 << (bits - 1) } else { 0 },
                character: false,
            })
        } else {
            None
        }
    }
    fn rank(self, value: i64) -> u64 {
        if self.character {
            let value = value as u64;
            if value >= 0xe000 {
                value - 0x800
            } else {
                value
            }
        } else {
            (value as u64 & self.maximum) ^ self.sign
        }
    }
}
pub(super) fn positional_fields(
    source: &a::Program,
    fields: &[usize],
    count: usize,
    span: a::Span,
) -> Result<Vec<(usize, usize)>> {
    let mut rest = None;
    for (index, pattern) in fields.iter().enumerate() {
        if matches!(source.patterns[*pattern].kind, a::Pattern::Rest) {
            if rest.replace(index).is_some() {
                return Err(err(span, "E_TYPE", "tuple pattern has more than one rest"));
            }
        }
    }
    let explicit = fields.len() - rest.is_some() as usize;
    if explicit > count || (rest.is_none() && explicit != count) {
        return Err(err(
            span,
            "E_TYPE",
            "constructor pattern field count differs",
        ));
    }
    let mut positions = Vec::new();
    for (index, pattern) in fields.iter().enumerate() {
        if Some(index) == rest {
            continue;
        }
        let field = if rest.is_some_and(|rest| index > rest) {
            count - (fields.len() - index)
        } else {
            index
        };
        positions.push((field, *pattern));
    }
    Ok(positions)
}
impl Builder<'_, '_> {
    pub(super) fn pattern_hint(&self, arms: &[a::Arm]) -> Option<usize> {
        let mut pending: Vec<usize> = Vec::new();
        for arm in arms {
            pending.push(arm.pattern);
        }
        while let Some(pattern) = pending.pop() {
            let ids = match &self.compiler.source.patterns[pattern].kind {
                a::Pattern::Literal(id) => vec![*id],
                a::Pattern::Range { start, end, .. } => {
                    let mut ids = Vec::new();
                    if let Some(id) = start {
                        ids.push(*id);
                    }
                    if let Some(id) = end {
                        ids.push(*id);
                    }
                    ids
                }
                a::Pattern::Or(ids) => {
                    pending.extend_from_slice(ids);
                    Vec::new()
                }
                _ => Vec::new(),
            };
            for mut id in ids {
                if let a::Expr::Unary { op, value } = &self.compiler.source.exprs[id].kind {
                    if op == "-" {
                        id = *value;
                    }
                }
                if matches!(self.compiler.source.exprs[id].kind, a::Expr::Char(_)) {
                    return self.compiler.type_names.get(&"char".to_string()).copied();
                }
                if let a::Expr::Int(text) = &self.compiler.source.exprs[id].kind {
                    if let Some(ty) = integers::suffix_type(&self.compiler.type_names, text) {
                        return Some(ty);
                    }
                }
            }
        }
        None
    }
    fn integer_pattern(&self, expression: usize, ty: usize) -> Result<i64> {
        let node = &self.compiler.source.exprs[expression];
        if matches!(self.compiler.types[ty].kind, ir::TypeKind::Char) {
            return match node.kind {
                a::Expr::Char(value) => Ok(value as i64),
                _ => Err(err(
                    node.span,
                    "E_TYPE",
                    "character pattern requires a character literal",
                )),
            };
        }
        let (text, negative) = match &node.kind {
            a::Expr::Int(text) => (text, false),
            a::Expr::Unary { op, value } if op == "-" => {
                match &self.compiler.source.exprs[*value].kind {
                    a::Expr::Int(text) => (text, true),
                    _ => {
                        return Err(err(
                            node.span,
                            "E_TYPE",
                            "pattern endpoint requires an integer literal",
                        ));
                    }
                }
            }
            _ => {
                return Err(err(
                    node.span,
                    "E_UNSUPPORTED",
                    "pattern endpoints require integer or byte literals",
                ));
            }
        };
        integers::literal(
            &self.compiler.types,
            &self.compiler.type_names,
            text,
            Some(ty),
            negative,
            node.span,
        )
        .map(|(value, _)| value)
    }
    pub(super) fn prepare_pattern(
        &mut self,
        plan: &mut PatternPlan,
        pattern: usize,
        ty: usize,
        borrow: Option<bool>,
    ) -> Result<usize> {
        let root = self.prepare_pattern_inner(plan, pattern, ty, borrow)?;
        let mut names = BTreeSet::new();
        for binding in plan.bindings(root, ir::Place::local(0)) {
            if !names.insert(binding.name) {
                return Err(err(
                    self.compiler.source.patterns[pattern].span,
                    "E_TYPE",
                    "pattern binds a name more than once",
                ));
            }
        }
        Ok(root)
    }
    fn prepare_pattern_inner(
        &mut self,
        plan: &mut PatternPlan,
        pattern: usize,
        ty: usize,
        borrow: Option<bool>,
    ) -> Result<usize> {
        let node = self.compiler.source.patterns[pattern].clone();
        if let a::Pattern::Path(path)
        | a::Pattern::Variant { path, .. }
        | a::Pattern::Struct { path, .. } = &node.kind
        {
            self.explicit_type_arguments(&path.args, node.span)?;
        }
        // A string pattern already has type &str; another implicit dereference
        // would wrongly accept a Rust-invalid &&str scrutinee.
        let string_literal = matches!(node.kind, a::Pattern::Literal(value) if matches!(self.compiler.source.exprs[value].kind, a::Expr::String(_)));
        // Match ergonomics follow references only for structural/literal forms.
        if !string_literal
            && !matches!(
                node.kind,
                a::Pattern::Bind { .. }
                    | a::Pattern::Wild
                    | a::Pattern::Ref { .. }
                    | a::Pattern::Or(_)
            )
        {
            if let ir::TypeKind::Ref { inner, mutable } = self.compiler.types[ty].kind {
                let mode = Some(borrow.unwrap_or(true) && mutable);
                let child = self.prepare_pattern_inner(plan, pattern, inner, mode)?;
                return Ok(plan.add(ty, node.span, PatternKind::Deref(child)));
            }
        }
        let kind = match node.kind {
            a::Pattern::Wild => PatternKind::Any,
            a::Pattern::Rest => {
                return Err(err(
                    node.span,
                    "E_TYPE",
                    "rest pattern requires tuple fields",
                ));
            }
            a::Pattern::Bind {
                name,
                mutable,
                by_ref,
                ref_mut,
            } => PatternKind::Bind {
                name,
                mutable: mutable && !by_ref,
                borrow: if by_ref { Some(ref_mut) } else { borrow },
            },
            a::Pattern::Literal(value) => {
                if Domain::new(&self.compiler.types[ty].kind).is_some() {
                    self.integer_pattern(value, ty)?;
                } else if !matches!(
                    (
                        &self.compiler.source.exprs[value].kind,
                        &self.compiler.types[ty].kind
                    ),
                    (a::Expr::Bool(_), ir::TypeKind::Bool) | (a::Expr::Float(_), ir::TypeKind::F64)
                ) && !(string_literal
                    && matches!(self.compiler.types[ty].kind, ir::TypeKind::Ref { inner, mutable: false } if matches!(self.compiler.types[inner].kind, ir::TypeKind::Str)))
                {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "literal pattern differs from scrutinee type",
                    ));
                }
                PatternKind::Literal(value)
            }
            a::Pattern::Range {
                start,
                end,
                inclusive,
            } => {
                let domain: Domain = match Domain::new(&self.compiler.types[ty].kind) {
                    Some(domain) => domain,
                    None => {
                        return Err(err(
                            node.span,
                            "E_TYPE",
                            "range pattern requires an integer scrutinee",
                        ));
                    }
                };
                let start = match start {
                    Some(id) => Some(self.integer_pattern(id, ty)?),
                    None => None,
                };
                let end = match end {
                    Some(id) => Some(self.integer_pattern(id, ty)?),
                    None => None,
                };
                let first = start.map(|v| domain.rank(v)).unwrap_or(0);
                let last = match end {
                    Some(v) if !inclusive => {
                        let rank = domain.rank(v);
                        if rank == 0 {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "range pattern is empty or reversed",
                            ));
                        }
                        rank - 1
                    }
                    Some(v) => domain.rank(v),
                    None => domain.maximum,
                };
                if first > last {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "range pattern is empty or reversed",
                    ));
                }
                PatternKind::Range(IntegerRange {
                    start,
                    end,
                    inclusive,
                })
            }
            a::Pattern::Ref { mutable, inner } => {
                let ir::TypeKind::Ref {
                    inner: referent,
                    mutable: actual,
                } = self.compiler.types[ty].kind
                else {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "reference pattern requires a reference",
                    ));
                };
                if mutable != actual {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "reference pattern mutability differs",
                    ));
                }
                let child = self.prepare_pattern_inner(plan, inner, referent, None)?;
                PatternKind::Deref(child)
            }
            a::Pattern::Or(alternatives) => {
                let mut roots = Vec::new();
                let mut signature: Option<BindingSignature> = None;
                for alternative in alternatives {
                    let root = self.prepare_pattern_inner(plan, alternative, ty, borrow)?;
                    let mut names = BTreeMap::new();
                    for binding in plan.bindings(root, ir::Place::local(0)) {
                        if names
                            .insert(binding.name, (binding.ty, binding.mutable, binding.borrow))
                            .is_some()
                        {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "pattern binds a name more than once",
                            ));
                        }
                    }
                    if signature.is_some() {
                        let expected = signature.as_ref().unwrap();
                        if expected != &names {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "or-pattern alternatives must bind the same names with the same types and modes",
                            ));
                        }
                    } else {
                        signature = Some(names);
                    }
                    roots.push(root);
                }
                PatternKind::Or(roots)
            }
            a::Pattern::Tuple(fields) => {
                let declared = match &self.compiler.types[ty].kind {
                    ir::TypeKind::Struct { fields, .. }
                        if fields
                            .iter()
                            .enumerate()
                            .all(|(i, f)| f.name == format!("{i}")) =>
                    {
                        fields.clone()
                    }
                    ir::TypeKind::Unit => Vec::new(),
                    _ => return Err(err(node.span, "E_TYPE", "tuple pattern requires a tuple")),
                };
                let mut children = Vec::new();
                for (index, pattern) in
                    positional_fields(self.compiler.source, &fields, declared.len(), node.span)?
                {
                    children.push((
                        ir::Projection::Field(index),
                        self.prepare_pattern_inner(plan, pattern, declared[index].ty, borrow)?,
                    ));
                }
                PatternKind::Product(children)
            }
            a::Pattern::Path(path) => {
                if matches!(self.compiler.types[ty].kind, ir::TypeKind::Enum { .. }) {
                    let (_, tag) = self.variant(&path, Some(ty), node.span)?;
                    self.variant_style(ty, tag, a::VariantStyle::Unit, node.span)?;
                    let ir::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind else {
                        unreachable!()
                    };
                    if !variants[tag].fields.is_empty() {
                        return Err(err(
                            node.span,
                            "E_TYPE",
                            "variant pattern requires its fields",
                        ));
                    }
                    PatternKind::Variant {
                        tag,
                        fields: Vec::new(),
                    }
                } else {
                    self.check_struct_pattern(&path, ty, node.span)?;
                    let ir::TypeKind::Struct { fields, .. } = &self.compiler.types[ty].kind else {
                        return Err(err(
                            node.span,
                            "E_TYPE",
                            "unit pattern requires a unit constructor",
                        ));
                    };
                    if !fields.is_empty() {
                        return Err(err(
                            node.span,
                            "E_TYPE",
                            "constructor pattern requires its fields",
                        ));
                    }
                    PatternKind::Product(Vec::new())
                }
            }
            a::Pattern::Variant { path, fields } => {
                let (tag, declared) =
                    if matches!(self.compiler.types[ty].kind, ir::TypeKind::Enum { .. }) {
                        let (_, tag) = self.variant(&path, Some(ty), node.span)?;
                        self.variant_style(ty, tag, a::VariantStyle::Tuple, node.span)?;
                        let ir::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind
                        else {
                            unreachable!()
                        };
                        (Some(tag), variants[tag].fields.clone())
                    } else {
                        self.check_struct_pattern(&path, ty, node.span)?;
                        let ir::TypeKind::Struct { fields, .. } = &self.compiler.types[ty].kind
                        else {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "tuple constructor pattern requires a struct or enum",
                            ));
                        };
                        if !fields
                            .iter()
                            .enumerate()
                            .all(|(index, field)| field.name == format!("{index}"))
                        {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "tuple pattern requires tuple fields",
                            ));
                        }
                        (None, fields.clone())
                    };
                let mut children = Vec::new();
                for (index, pattern) in
                    positional_fields(self.compiler.source, &fields, declared.len(), node.span)?
                {
                    let projection = match tag {
                        Some(tag) => ir::Projection::VariantField(tag, index),
                        None => ir::Projection::Field(index),
                    };
                    children.push((
                        projection,
                        self.prepare_pattern_inner(plan, pattern, declared[index].ty, borrow)?,
                    ));
                }
                match tag {
                    Some(tag) => PatternKind::Variant {
                        tag,
                        fields: children,
                    },
                    None => PatternKind::Product(children),
                }
            }
            a::Pattern::Struct { path, fields, rest } => {
                let (tag, declared) =
                    if matches!(self.compiler.types[ty].kind, ir::TypeKind::Enum { .. }) {
                        let (_, tag) = self.variant(&path, Some(ty), node.span)?;
                        self.variant_style(ty, tag, a::VariantStyle::Named, node.span)?;
                        let ir::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind
                        else {
                            unreachable!()
                        };
                        (Some(tag), variants[tag].fields.clone())
                    } else {
                        self.check_struct_pattern(&path, ty, node.span)?;
                        let ir::TypeKind::Struct { fields, .. } = &self.compiler.types[ty].kind
                        else {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "named pattern requires a struct or enum",
                            ));
                        };
                        (None, fields.clone())
                    };
                let mut indices: BTreeMap<String, usize> = BTreeMap::new();
                for (index, field) in declared.iter().enumerate() {
                    indices.insert(field.name.clone(), index);
                }
                let mut seen = BTreeSet::new();
                let mut children = Vec::new();
                for (name, pattern) in fields {
                    let selected = indices.get(&name).copied();
                    let index = match selected {
                        Some(index) => index,
                        None => {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                &format!("unknown pattern field {name}"),
                            ));
                        }
                    };
                    if !seen.insert(index) {
                        return Err(err(
                            node.span,
                            "E_TYPE",
                            &format!("duplicate pattern field {name}"),
                        ));
                    }
                    if tag.is_none() {
                        self.compiler
                            .field_access(ty, &name, self.scope, node.span)?;
                    }
                    let projection = match tag {
                        Some(tag) => ir::Projection::VariantField(tag, index),
                        None => ir::Projection::Field(index),
                    };
                    children.push((
                        projection,
                        self.prepare_pattern_inner(plan, pattern, declared[index].ty, borrow)?,
                    ));
                }
                if !rest && seen.len() != declared.len() {
                    return Err(err(node.span, "E_TYPE", "pattern is missing fields"));
                }
                match tag {
                    Some(tag) => PatternKind::Variant {
                        tag,
                        fields: children,
                    },
                    None => PatternKind::Product(children),
                }
            }
        };
        Ok(plan.add(ty, node.span, kind))
    }
    fn check_struct_pattern(&mut self, path: &a::Path, ty: usize, span: a::Span) -> Result<()> {
        let target = self
            .constructor_target(path, Some(ty), span)?
            .ok_or_else(|| {
                err(
                    span,
                    "E_TYPE",
                    "pattern requires the scrutinee's struct declaration",
                )
            })?;
        if self.nominal_declaration(ty, span)? != target.declaration {
            return Err(err(
                span,
                "E_TYPE",
                "pattern constructor differs from scrutinee",
            ));
        }
        Ok(())
    }
    fn pattern_boolean(&mut self, value: bool, line: usize) -> usize {
        let dst = self.temp(ir::BOOL);
        self.emit(
            ir::Op::Int {
                dst,
                value: value as i64,
            },
            line,
        );
        dst
    }
    pub(super) fn pattern_condition(
        &mut self,
        plan: &PatternPlan,
        id: usize,
        place: ir::Place,
    ) -> Result<usize> {
        let node = &plan.nodes[id];
        let line = node.span.line;
        match &node.kind {
            PatternKind::Any | PatternKind::Bind { .. } => Ok(self.pattern_boolean(true, line)),
            PatternKind::Deref(inner) => {
                let mut place = place;
                place.projections.push(ir::Projection::Deref);
                self.pattern_condition(plan, *inner, place)
            }
            PatternKind::Literal(expression) => {
                if matches!(
                    self.compiler.source.exprs[*expression].kind,
                    a::Expr::String(_)
                ) {
                    let left = self.read(place, line)?.0;
                    let ty = self.ty(left);
                    let right = self.expr(*expression, Some(ty))?.0;
                    return Ok(self.text_equality_values("==", left, right, node.span)?.0);
                }
                let left = self.read(place, line)?.0;
                let right = self.expr(*expression, Some(node.ty))?.0;
                let dst = self.temp(ir::BOOL);
                self.emit(
                    ir::Op::Binary {
                        dst,
                        op: "==".to_string(),
                        left,
                        right,
                    },
                    line,
                );
                Ok(dst)
            }
            PatternKind::Range(range) => self.range_condition(place, node.ty, *range, line),
            PatternKind::Product(fields) => self.product_condition(plan, fields, place, line),
            PatternKind::Variant { tag, fields } => {
                let result = self.local(ir::BOOL, "$pattern", true);
                let no = self.pattern_boolean(false, line);
                self.emit(
                    ir::Op::Assign {
                        place: ir::Place::local(result),
                        src: no,
                    },
                    line,
                );
                let condition = self.temp(ir::BOOL);
                self.emit(
                    ir::Op::TagTest {
                        dst: condition,
                        place: place.clone(),
                        variant: *tag,
                    },
                    line,
                );
                self.emit(ir::Op::If { condition }, line);
                let before = self.snapshot();
                let start = self.live.len();
                let yes = self.product_condition(plan, fields, place, line)?;
                self.emit(
                    ir::Op::Assign {
                        place: ir::Place::local(result),
                        src: yes,
                    },
                    line,
                );
                self.cleanup(start, None, line);
                self.emit(ir::Op::End, line);
                self.restore(&before);
                Ok(result)
            }
            PatternKind::Or(alternatives) => {
                let mut result = self.pattern_boolean(false, line);
                for alternative in alternatives {
                    let right = self.pattern_condition(plan, *alternative, place.clone())?;
                    let dst = self.temp(ir::BOOL);
                    self.emit(
                        ir::Op::Binary {
                            dst,
                            op: "|".to_string(),
                            left: result,
                            right,
                        },
                        line,
                    );
                    result = dst;
                }
                Ok(result)
            }
        }
    }
    fn product_condition(
        &mut self,
        plan: &PatternPlan,
        fields: &[(ir::Projection, usize)],
        place: ir::Place,
        line: usize,
    ) -> Result<usize> {
        let mut result = self.pattern_boolean(true, line);
        for (projection, pattern) in fields {
            let mut field = place.clone();
            field.projections.push(projection.clone());
            let right = self.pattern_condition(plan, *pattern, field)?;
            let dst = self.temp(ir::BOOL);
            self.emit(
                ir::Op::Binary {
                    dst,
                    op: "&".to_string(),
                    left: result,
                    right,
                },
                line,
            );
            result = dst;
        }
        Ok(result)
    }
    fn establish_pattern_tag(&mut self, place: ir::Place, tag: usize, line: usize) {
        let condition = self.temp(ir::BOOL);
        self.emit(
            ir::Op::TagTest {
                dst: condition,
                place,
                variant: tag,
            },
            line,
        );
        self.emit(ir::Op::If { condition }, line);
        let selected = self.snapshot();
        self.emit(ir::Op::Else, line);
        self.emit(
            ir::Op::Trap {
                message: "unreachable selected pattern".to_string(),
            },
            line,
        );
        self.emit(ir::Op::End, line);
        self.restore(&selected);
    }

    /// Select each shared or subtree independently, then reach one arm body.
    /// IR checks each destination's reference alternatives at every join.
    fn bind_pattern_tree(
        &mut self,
        plan: &PatternPlan,
        id: usize,
        place: ir::Place,
        targets: &BindingTargets<'_>,
        line: usize,
    ) -> Result<()> {
        match &plan.nodes[id].kind {
            PatternKind::Bind { name, borrow, .. } => {
                let binding_source = place.clone();
                let value = match borrow {
                    Some(mutable) => self.borrow(place, *mutable, line)?,
                    None if targets.guard => self.borrow(place, false, line)?,
                    None => self.read(place, line)?,
                };
                self.annotation_binding(
                    &binding_source,
                    *targets.destinations.get(name).unwrap(),
                    borrow.is_some(),
                );
                self.emit(
                    ir::Op::Assign {
                        place: ir::Place::local(*targets.destinations.get(name).unwrap()),
                        src: value.0,
                    },
                    line,
                );
            }
            PatternKind::Deref(inner) => {
                let mut place = place;
                place.projections.push(ir::Projection::Deref);
                self.bind_pattern_tree(plan, *inner, place, targets, line)?;
            }
            PatternKind::Product(fields) | PatternKind::Variant { fields, .. } => {
                if let PatternKind::Variant { tag, .. } = &plan.nodes[id].kind {
                    self.establish_pattern_tag(place.clone(), *tag, line);
                }
                for (projection, child) in fields {
                    let mut field = place.clone();
                    field.projections.push(projection.clone());
                    self.bind_pattern_tree(plan, *child, field, targets, line)?;
                }
            }
            PatternKind::Or(alternatives) => {
                let start = self.live.len();
                let mut exit = None;
                for (index, alternative) in alternatives.iter().enumerate() {
                    let condition = if let Some(choice) =
                        targets.choices.and_then(|choices| choices.slots.get(&id))
                    {
                        self.pattern_choice_equals(choice.position, index, line)?
                    } else {
                        self.pattern_condition(plan, *alternative, place.clone())?
                    };
                    self.emit(ir::Op::If { condition }, line);
                    let failure = self.snapshot();
                    self.bind_pattern_tree(plan, *alternative, place.clone(), targets, line)?;
                    self.cleanup(start, None, line);
                    self.written.truncate(start);
                    self.written.resize(self.function.locals.len(), false);
                    if let Some(previous) = &exit {
                        self.merge_carriers(previous);
                    }
                    exit = Some(self.snapshot());
                    self.emit(ir::Op::Else, line);
                    self.restore(&failure);
                }
                self.emit(
                    ir::Op::Trap {
                        message: "unreachable selected or-pattern".to_string(),
                    },
                    line,
                );
                for _ in alternatives {
                    self.emit(ir::Op::End, line);
                }
                if let Some(exit) = exit {
                    self.restore(&exit);
                }
            }
            _ => {}
        }
        Ok(())
    }
    // A selected constructor proves its tag even when it binds no fields.
    // Preserve that evidence on unit-variant exits so a join cannot revive a
    // reference template that is absent on every feasible branch.
    fn establish_pattern_shells(
        &mut self,
        plan: &PatternPlan,
        root: usize,
        place: ir::Place,
        line: usize,
    ) -> BTreeSet<(ir::Place, usize)> {
        let mut established = BTreeSet::new();
        let mut pending = vec![(root, place)];
        while let Some((id, place)) = pending.pop() {
            match &plan.nodes[id].kind {
                PatternKind::Deref(inner) => {
                    let mut nested = place;
                    nested.projections.push(ir::Projection::Deref);
                    pending.push((*inner, nested));
                }
                PatternKind::Product(fields) | PatternKind::Variant { fields, .. } => {
                    if let PatternKind::Variant { tag, .. } = &plan.nodes[id].kind {
                        if established.insert((place.clone(), *tag)) {
                            self.establish_pattern_tag(place.clone(), *tag, line);
                        }
                    }
                    let mut index = fields.len();
                    while index != 0 {
                        index -= 1;
                        let mut nested = place.clone();
                        nested.projections.push(fields[index].0.clone());
                        pending.push((fields[index].1, nested));
                    }
                }
                // Alternative-specific tags are established along the selected
                // binding path; an unselected alternative grants no evidence.
                PatternKind::Or(_) => {}
                _ => {}
            }
        }
        established
    }

    /// Binding projections additionally establish tags inside uniform or-patterns.
    /// Alternatives may differ in tests while sharing exactly these places.
    fn establish_bindings(&mut self, bindings: &[PatternBinding], line: usize) {
        self.establish_binding_tags(bindings, &mut BTreeSet::new(), line);
    }
    fn establish_binding_tags(
        &mut self,
        bindings: &[PatternBinding],
        established: &mut BTreeSet<(ir::Place, usize)>,
        line: usize,
    ) {
        for binding in bindings {
            let mut place = ir::Place::local(binding.place.local);
            for projection in &binding.place.projections {
                if let ir::Projection::VariantField(tag, _) = projection {
                    if established.insert((place.clone(), *tag)) {
                        self.establish_pattern_tag(place.clone(), *tag, line);
                    }
                }
                place.projections.push(projection.clone());
            }
        }
    }
    pub(super) fn bind_pattern_plan(
        &mut self,
        plan: &PatternPlan,
        id: usize,
        place: ir::Place,
        line: usize,
    ) -> Result<()> {
        self.bind_pattern_choice(plan, id, place, None, true, line)
    }
    pub(super) fn bind_pattern_choice(
        &mut self,
        plan: &PatternPlan,
        id: usize,
        place: ir::Place,
        choices: Option<&PatternChoices>,
        read_shells: bool,
        line: usize,
    ) -> Result<()> {
        if plan.has_or(id) && !plan.uniform_bindings(id, place.clone()) {
            let mut destinations = BTreeMap::new();
            for binding in plan.bindings(id, place.clone()) {
                let ty = match binding.borrow {
                    Some(mutable) => self.compiler.reference(binding.ty, mutable),
                    None => binding.ty,
                };
                let destination = self.local(ty, &binding.name, binding.mutable);
                destinations.insert(binding.name.clone(), destination);
                self.bindings.push((binding.name, destination));
            }
            let targets = BindingTargets {
                destinations: &destinations,
                guard: false,
                choices,
            };
            return self.bind_pattern_tree(plan, id, place, &targets, line);
        }
        let bindings = plan.bindings(id, place.clone());
        // Guarded matches use tests cached before any guard. A guard may move
        // an otherwise unbound scrutinee, so supplementary tag reads are valid
        // only on the ordinary path. Binding projections still need witnesses.
        let mut established = if read_shells {
            self.establish_pattern_shells(plan, id, place, line)
        } else {
            BTreeSet::new()
        };
        self.establish_binding_tags(&bindings, &mut established, line);
        for binding in bindings {
            let source = binding.place.clone();
            let value = match binding.borrow {
                Some(mutable) => self.borrow(binding.place, mutable, line)?,
                None => self.read(binding.place, line)?,
            };
            let destination = self.bind_name(binding.name, binding.mutable, value, line);
            self.annotation_binding(&source, destination, binding.borrow.is_some());
        }
        Ok(())
    }
    pub(super) fn pattern_guard(
        &mut self,
        plan: &PatternPlan,
        id: usize,
        place: ir::Place,
        guard: usize,
        condition: usize,
        tested: &[ir::Place],
    ) -> Result<usize> {
        self.pattern_guard_with_choices(
            plan,
            id,
            place,
            PatternGuardInput {
                expression: guard,
                condition,
                tested,
                choices: None,
            },
        )
    }
    pub(super) fn pattern_guard_with_choices(
        &mut self,
        plan: &PatternPlan,
        id: usize,
        place: ir::Place,
        input: PatternGuardInput<'_>,
    ) -> Result<usize> {
        let line = plan.nodes[id].span.line;
        let PatternGuardInput {
            expression: guard,
            condition,
            tested,
            choices,
        } = input;
        let result = self.local(ir::BOOL, "$guard", true);
        let no = self.pattern_boolean(false, line);
        self.emit(
            ir::Op::Assign {
                place: ir::Place::local(result),
                src: no,
            },
            line,
        );
        self.emit(ir::Op::If { condition }, line);
        let before = self.snapshot();
        let start = self.live.len();
        let names = self.bindings.len();
        let aliases = self.guard_aliases.clone();
        let shells = self.guard_shells.len();
        let bindings = plan.bindings(id, place.clone());
        let mut destinations = BTreeMap::new();
        for binding in &bindings {
            let ty = self
                .compiler
                .reference(binding.ty, binding.borrow.unwrap_or(false));
            let destination = self.local(ty, "$guard_binding", false);
            destinations.insert(binding.name.clone(), destination);
        }
        if !bindings.is_empty() {
            if plan.uniform_bindings(id, place.clone()) {
                self.establish_bindings(&bindings, line);
                for binding in &bindings {
                    let value =
                        self.borrow(binding.place.clone(), binding.borrow.unwrap_or(false), line)?;
                    self.emit(
                        ir::Op::Assign {
                            place: ir::Place::local(*destinations.get(&binding.name).unwrap()),
                            src: value.0,
                        },
                        line,
                    );
                }
            } else {
                let targets = BindingTargets {
                    destinations: &destinations,
                    guard: true,
                    choices,
                };
                self.bind_pattern_tree(plan, id, place, &targets, line)?;
            }
        }
        for binding in bindings {
            let local = *destinations.get(&binding.name).unwrap();
            if binding.borrow.is_none() {
                self.guard_aliases.insert(
                    binding.name,
                    ir::Place {
                        local,
                        projections: vec![ir::Projection::Deref],
                    },
                );
            } else {
                self.bindings.push((binding.name.clone(), local));
                if binding.borrow == Some(true) {
                    let frozen = self.borrow(ir::Place::local(local), false, line)?.0;
                    self.guard_aliases.insert(
                        binding.name,
                        ir::Place {
                            local: frozen,
                            projections: vec![ir::Projection::Deref],
                        },
                    );
                }
            }
        }
        self.guard_shells.extend_from_slice(tested);
        let value = self.expr(guard, Some(ir::BOOL))?;
        self.guard_shells.truncate(shells);
        let reached = self.reachable;
        if reached {
            self.emit(
                ir::Op::Assign {
                    place: ir::Place::local(result),
                    src: value.0,
                },
                line,
            );
            self.cleanup(start, None, line);
        }
        let after = self.snapshot();
        self.bindings.truncate(names);
        self.guard_aliases = aliases;
        self.emit(ir::Op::End, line);
        self.restore(&before);
        if reached {
            self.merge_carriers(&after);
        }
        Ok(result)
    }
    fn range_condition(
        &mut self,
        place: ir::Place,
        ty: usize,
        range: IntegerRange,
        line: usize,
    ) -> Result<usize> {
        let mut conditions = Vec::new();
        for index in 0..2 {
            let (bound, op) = if index == 0 {
                (range.start, ">=")
            } else {
                (range.end, if range.inclusive { "<=" } else { "<" })
            };
            if let Some(value) = bound {
                let left = self.read(place.clone(), line)?.0;
                let right = self.temp(ty);
                self.emit(ir::Op::Int { dst: right, value }, line);
                let dst = self.temp(ir::BOOL);
                self.emit(
                    ir::Op::Binary {
                        dst,
                        op: op.to_string(),
                        left,
                        right,
                    },
                    line,
                );
                conditions.push(dst);
            }
        }
        if conditions.len() == 2 {
            let dst = self.temp(ir::BOOL);
            self.emit(
                ir::Op::Binary {
                    dst,
                    op: "&".to_string(),
                    left: conditions[0],
                    right: conditions[1],
                },
                line,
            );
            Ok(dst)
        } else {
            Ok(conditions[0])
        }
    }
    pub(super) fn pattern_irrefutable(&self, plan: &PatternPlan, root: usize) -> bool {
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            match &plan.nodes[id].kind {
                PatternKind::Any | PatternKind::Bind { .. } => {}
                PatternKind::Product(fields) => {
                    for (_, id) in fields {
                        pending.push(*id);
                    }
                }
                PatternKind::Deref(inner) => pending.push(*inner),
                PatternKind::Variant { fields, .. } => {
                    if !matches!(&self.compiler.types[plan.nodes[id].ty].kind, ir::TypeKind::Enum { variants, .. } if variants.len() == 1)
                    {
                        return false;
                    }
                    for (_, id) in fields {
                        pending.push(*id);
                    }
                }
                _ => return false,
            }
        }
        true
    }
    pub(super) fn patterns_exhaustive(
        &self,
        plan: &PatternPlan,
        roots: &[usize],
        ty: usize,
    ) -> Result<bool> {
        let mut rows = Vec::new();
        for id in roots {
            rows.push(vec![Some(*id)]);
        }
        self.pattern_matrix(plan, rows, vec![ty], &mut CoverageMemo::new())
    }
    fn pattern_matrix(
        &self,
        plan: &PatternPlan,
        mut rows: Vec<Vec<Option<usize>>>,
        types: Vec<usize>,
        memo: &mut CoverageMemo,
    ) -> Result<bool> {
        // Constructor alternatives often reach the very same shared suffix.
        // Normalize bindings to wildcards so coverage checks that suffix once.
        let mut row_index = 0;
        while row_index < rows.len() {
            let mut column = 0;
            while column < rows[row_index].len() {
                if let Some(id) = rows[row_index][column] {
                    if matches!(
                        plan.nodes[id].kind,
                        PatternKind::Any | PatternKind::Bind { .. }
                    ) {
                        rows[row_index][column] = None;
                    }
                }
                column += 1;
            }
            row_index += 1;
        }
        rows.sort();
        let mut unique: Vec<Vec<Option<usize>>> = Vec::new();
        for row in rows {
            if unique.is_empty() || unique[unique.len() - 1] != row {
                unique.push(row);
            }
        }
        let rows = unique;
        let key = (rows.clone(), types.clone());
        if let Some(result) = memo.get(&key) {
            return Ok(*result);
        }
        let result = self.pattern_matrix_inner(plan, rows, types, memo)?;
        memo.insert(key, result);
        Ok(result)
    }
    fn pattern_matrix_inner(
        &self,
        plan: &PatternPlan,
        mut rows: Vec<Vec<Option<usize>>>,
        types: Vec<usize>,
        memo: &mut CoverageMemo,
    ) -> Result<bool> {
        if types.is_empty() {
            return Ok(!rows.is_empty());
        }
        if rows.is_empty() {
            return Ok(false);
        }
        // Expand one column only; shared suffixes stay shared in the plan arena.
        let mut pending = std::mem::replace(&mut rows, Vec::new());
        while let Some(row) = pending.pop() {
            if let Some(id) = row[0] {
                if let PatternKind::Or(alternatives) = &plan.nodes[id].kind {
                    for alternative in alternatives {
                        let mut next = row.clone();
                        next[0] = Some(*alternative);
                        pending.push(next);
                    }
                    continue;
                }
            }
            rows.push(row);
        }
        let wild = |id: Option<usize>| match id {
            Some(id) => matches!(
                plan.nodes[id].kind,
                PatternKind::Any | PatternKind::Bind { .. }
            ),
            None => true,
        };
        for row in &rows {
            let mut all_wild = true;
            for id in row {
                if !wild(*id) {
                    all_wild = false;
                    break;
                }
            }
            if all_wild {
                return Ok(true);
            }
        }
        let ty = types[0];
        let constructors: Vec<PatternConstructor> = match &self.compiler.types[ty].kind {
            ir::TypeKind::Enum { variants, .. } => {
                let mut constructors = Vec::new();
                for (tag, variant) in variants.iter().enumerate() {
                    let mut fields = Vec::new();
                    for (index, field) in variant.fields.iter().enumerate() {
                        fields.push((ir::Projection::VariantField(tag, index), field.ty));
                    }
                    constructors.push((Some(tag), fields));
                }
                constructors
            }
            ir::TypeKind::Struct { fields, .. } => {
                let mut constructor = Vec::new();
                for (index, field) in fields.iter().enumerate() {
                    constructor.push((ir::Projection::Field(index), field.ty));
                }
                vec![(None, constructor)]
            }
            ir::TypeKind::Ref { inner, .. } => {
                vec![(None, vec![(ir::Projection::Deref, *inner)])]
            }
            ir::TypeKind::Unit => vec![(None, Vec::new())],
            _ => Vec::new(),
        };
        if !constructors.is_empty() {
            let mut common: Vec<usize> = Vec::new();
            let mut tagged: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            for row_index in 0..rows.len() {
                if let Some(id) = rows[row_index][0] {
                    if let PatternKind::Variant { tag, .. } = plan.nodes[id].kind {
                        if tagged.contains_key(&tag) {
                            tagged.get_mut(&tag).unwrap().push(row_index);
                        } else {
                            tagged.insert(tag, vec![row_index]);
                        }
                        continue;
                    }
                }
                common.push(row_index);
            }
            for (tag, fields) in constructors {
                let mut specialized = Vec::new();
                let mut selected_rows = common.clone();
                if let Some(tag) = tag {
                    if let Some(selected) = tagged.get(&tag) {
                        selected_rows.extend_from_slice(selected);
                    }
                }
                for row_index in selected_rows {
                    let row = &rows[row_index];
                    let selected = match row[0] {
                        None => None,
                        Some(id) => match &plan.nodes[id].kind {
                            PatternKind::Any | PatternKind::Bind { .. } => None,
                            PatternKind::Variant {
                                tag: actual,
                                fields,
                            } if Some(*actual) == tag => Some(fields.clone()),
                            PatternKind::Product(fields) if tag.is_none() => Some(fields.clone()),
                            PatternKind::Deref(inner) => {
                                Some(vec![(ir::Projection::Deref, *inner)])
                            }
                            _ => continue,
                        },
                    };
                    let mut next = Vec::new();
                    for (projection, _) in &fields {
                        let mut found = None;
                        if let Some(selected) = &selected {
                            for (candidate, id) in selected {
                                if candidate == projection {
                                    found = Some(*id);
                                    break;
                                }
                            }
                        }
                        next.push(found);
                    }
                    next.extend_from_slice(&row[1..]);
                    specialized.push(next);
                }
                let mut next_types = Vec::new();
                for (_, ty) in &fields {
                    next_types.push(*ty);
                }
                next_types.extend_from_slice(&types[1..]);
                if !self.pattern_matrix(plan, specialized, next_types, memo)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        let mut cuts: BTreeSet<u64> = BTreeSet::new();
        cuts.insert(0u64);
        let domain = Domain::new(&self.compiler.types[ty].kind);
        let maximum = if ty == ir::BOOL {
            1
        } else if let Some(domain) = domain {
            domain.maximum
        } else {
            let mut selected = Vec::new();
            for row in rows {
                if wild(row[0]) {
                    selected.push(row[1..].to_vec());
                }
            }
            return self.pattern_matrix(plan, selected, types[1..].to_vec(), memo);
        };
        let mut intervals = Vec::new();
        for row in &rows {
            let interval = if wild(row[0]) {
                Some((0, maximum))
            } else {
                let node = &plan.nodes[row[0].unwrap()];
                match &node.kind {
                    PatternKind::Range(range) => {
                        let domain = domain.unwrap();
                        Some((
                            range.start.map(|v| domain.rank(v)).unwrap_or(0),
                            range
                                .end
                                .map(|v| domain.rank(v) - (!range.inclusive) as u64)
                                .unwrap_or(maximum),
                        ))
                    }
                    PatternKind::Literal(expression) => {
                        let value = if ty == ir::BOOL {
                            if let a::Expr::Bool(v) = self.compiler.source.exprs[*expression].kind {
                                v as u64
                            } else {
                                return Ok(false);
                            }
                        } else {
                            domain.unwrap().rank(self.integer_pattern(*expression, ty)?)
                        };
                        Some((value, value))
                    }
                    _ => None,
                }
            };
            if let Some((first, last)) = interval {
                cuts.insert(first);
                if last < maximum {
                    cuts.insert(last + 1);
                }
            }
            intervals.push(interval);
        }
        for first in cuts {
            let mut selected = Vec::new();
            for (row, interval) in rows.iter().zip(&intervals) {
                if let Some((low, high)) = interval {
                    if *low <= first && first <= *high {
                        selected.push(row[1..].to_vec());
                    }
                }
            }
            if !self.pattern_matrix(plan, selected, types[1..].to_vec(), memo)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
