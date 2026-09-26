//! Scoped native declaration notation. This is an adapter into the existing authored intent:
//! names are typed locators, structural types are notation, and only explicit creates allocate.

use super::input::{Block, ChangeInput, SyntaxKind};
use super::*;

#[derive(Clone)]
struct Unit {
    syntax: usize,
    kind: String,
    name: String,
    scope: String,
    parent: String,
    label: String,
    clauses: Vec<usize>,
    existing: Option<OwnerKey>,
}

#[derive(Default)]
pub(super) struct Lowered {
    pub private: BTreeSet<String>,
    edits: BTreeMap<String, OwnerKey>,
}

struct Lowering<'a> {
    block: &'a Block,
    units: Vec<Unit>,
    names: BTreeMap<(String, String), String>,
    aliases: BTreeMap<String, (String, usize)>,
    imports: BTreeMap<String, String>,
    imported: BTreeMap<(String, String), String>,
    resolving: BTreeSet<String>,
    records: Vec<CompactRecord>,
    bodies: BTreeMap<String, Block>,
    reserved: &'a mut BTreeSet<String>,
    private: &'a mut BTreeSet<String>,
    next: &'a mut usize,
    edits: &'a mut BTreeMap<String, OwnerKey>,
    existing: &'a BTreeMap<OwnerKey, crate::platform::kernel::OwnerRecord>,
    package: Option<PackageId>,
    remaining_records: usize,
}

pub(super) fn lower(
    input: &mut ChangeInput,
    mut reader: Option<&mut super::canonical::Reader<'_>>,
) -> Result<Lowered, Diagnostic> {
    let mut lowered = Lowered::default();
    let mut next = 0;
    if input.units.len() > 1 {
        let mut blocks = std::mem::take(&mut input.units).into_iter();
        if let Some(mut combined) = blocks.next() {
            for block in blocks {
                let items = block.list(block.root)?;
                if items.first().and_then(|id| block.atom(*id).ok()) != Some("units") {
                    return Err(block.error(
                        block.root,
                        "change_unit_form",
                        "declarations.begin requires (units ...)",
                    ));
                }
                let offset = combined.syntax.len();
                let roots: Vec<_> = items[1..].iter().map(|id| id + offset).collect();
                for mut node in block.syntax {
                    if let SyntaxKind::List(children) = &mut node.kind {
                        for child in children {
                            *child += offset;
                        }
                    }
                    combined.syntax.push(node);
                }
                if let SyntaxKind::List(items) = &mut combined.syntax[combined.root].kind {
                    items.extend(roots);
                }
            }
            input.units.push(combined);
        }
    }
    let mut existing = BTreeMap::new();
    // Exact edit selections are collected independently of displayed names. Reading an identity
    // does not yet authorize mutation; complete contract membership is checked below.
    for block in &input.units {
        for (id, syntax) in block.syntax.iter().enumerate() {
            if let SyntaxKind::List(parts) = &syntax.kind
                && parts.len() >= 3
                && block.atom(parts[0]).ok().and_then(namespace).is_some()
                && block.atom(parts[1]).ok() == Some("edit")
            {
                let owner: OwnerKey =
                    block.atom(parts[2])?.parse().map_err(|mut e: Diagnostic| {
                        e.location = Some(syntax.location.clone());
                        e
                    })?;
                if existing.contains_key(&owner) {
                    return Err(block.error(
                        id,
                        "change_unit_duplicate",
                        format!(
                            "exact owner {owner} is selected by more than one complete edit unit"
                        ),
                    ));
                }
                let reader = reader.as_deref_mut().ok_or_else(|| {
                    block.error(
                        id,
                        "change_unit_context",
                        "edit units require the accepted repository/base context",
                    )
                })?;
                existing.insert(owner, reader.owner(owner)?);
            }
        }
    }
    for owner in existing.keys().copied().collect::<Vec<_>>() {
        if matches!(owner, OwnerKey::Target(_)) {
            let reader = reader
                .as_deref_mut()
                .ok_or_else(|| canonical::error("target edit requires its base"))?;
            for edge in reader.reader.incoming(
                owner,
                crate::platform::kernel::RelationKind::HttpRouteTarget,
                crate::platform::change::MAXIMUM_AUTHORED_CHANGES,
            )? {
                let crate::platform::kernel::RelationEndpoint::Owner(source) = edge.source else {
                    return Err(canonical::error("route ownership has no source"));
                };
                if source.package != reader.view.package() {
                    return Err(canonical::error("route ownership escaped its package"));
                }
                existing.insert(source.owner, reader.owner(source.owner)?);
            }
        }
    }
    let package = reader.as_ref().map(|r| r.view.package());
    // A single unit block supplies one collection/resolution scope. All declarations in that
    // block are collected before any body or type is resolved, including later declarations.
    for block in std::mem::take(&mut input.units) {
        let mut lowering = Lowering {
            block: &block,
            units: Vec::new(),
            names: BTreeMap::new(),
            aliases: BTreeMap::new(),
            imports: BTreeMap::new(),
            imported: BTreeMap::new(),
            resolving: BTreeSet::new(),
            records: Vec::new(),
            bodies: BTreeMap::new(),
            reserved: &mut input.public_labels,
            private: &mut lowered.private,
            next: &mut next,
            edits: &mut lowered.edits,
            existing: &existing,
            package,
            remaining_records: crate::platform::control::compact::MAXIMUM_COMPACT_RECORDS
                .saturating_sub(input.records.len()),
        };
        let items = block.list(block.root)?;
        if items.first().and_then(|id| block.atom(*id).ok()) != Some("units") {
            return Err(block.error(
                block.root,
                "change_unit_form",
                "declarations.begin requires one (units ...) collection",
            ));
        }
        for id in &items[1..] {
            lowering.collect(*id, "", "")?;
        }
        for (name, (_, at)) in &lowering.aliases {
            if ["declaration", "type-parameter"].iter().any(|class| {
                lowering
                    .names
                    .contains_key(&(name.clone(), (*class).to_owned()))
            }) {
                return Err(block.error(
                    *at,
                    "change_unit_duplicate",
                    format!("type alias '{name}' conflicts with a typed name in the same scope"),
                ));
            }
        }
        lowering.check_contracts(&input.records)?;
        for (scope, id) in lowering.aliases.values().cloned().collect::<Vec<_>>() {
            lowering.ty(id, &scope, 1)?;
        }
        // Function signature additions precede their declaration in the established public
        // library fixtures. Keep that authored identity traversal for cross-notation review.
        // The shared lowerer collects all declaration creates before applying child mutations.
        let units = lowering.units.clone();
        let mut emitted = BTreeSet::new();
        for (index, unit) in units.iter().enumerate() {
            if !emitted.insert(index) {
                continue;
            }
            if matches!(unit.kind.as_str(), "function" | "external") {
                let parent = unit
                    .existing
                    .map(|owner| owner.to_string())
                    .unwrap_or_else(|| unit.label.clone());
                for (child_index, child) in units
                    .iter()
                    .enumerate()
                    .filter(|(_, child)| child.parent == parent)
                {
                    if emitted.insert(child_index) {
                        lowering.emit(child)?;
                    }
                }
            }
            lowering.emit(unit)?;
        }
        input.records.extend(lowering.records);
        input.blocks.extend(lowering.bodies);
        if input.records.len() > crate::platform::control::compact::MAXIMUM_COMPACT_RECORDS {
            return Err(block.error(
                block.root,
                "change_unit_capacity",
                "lowered declaration request exceeds the compact-record admission",
            ));
        }
    }
    Ok(lowered)
}

impl Lowering<'_> {
    fn parts(&self, id: usize) -> Result<(&str, &[usize]), Diagnostic> {
        let items = self.block.list(id)?;
        let first = *items
            .first()
            .ok_or_else(|| self.error(id, "empty declaration clause"))?;
        Ok((self.block.atom(first)?, &items[1..]))
    }

    fn error(&self, id: usize, message: impl Into<String>) -> Diagnostic {
        self.block.error(id, "change_unit_form", message)
    }

    fn allocate(&mut self, prefix: char) -> Result<String, Diagnostic> {
        loop {
            let value = format!("{prefix}__unit_{}", self.next);
            *self.next = self
                .next
                .checked_add(1)
                .ok_or_else(|| self.error(self.block.root, "declaration ordinal overflow"))?;
            if self.reserved.insert(value.clone()) {
                if prefix == '$' {
                    self.private.insert(value.clone());
                }
                return Ok(value);
            }
        }
    }

    fn record(
        &mut self,
        at: usize,
        operation: &str,
        fields: Vec<(&str, String)>,
    ) -> Result<(), Diagnostic> {
        self.remaining_records = self.remaining_records.checked_sub(1).ok_or_else(|| {
            self.block.error(
                at,
                "change_unit_capacity",
                "complete lowered request exceeds record admission",
            )
        })?;
        self.records.try_reserve(1).map_err(|_| {
            self.block.error(
                at,
                "change_unit_capacity",
                "declaration record allocation failed",
            )
        })?;
        let location = self.block.syntax[at].location.clone();
        self.records.push(CompactRecord {
            operation: operation.to_owned(),
            location: location.clone(),
            fields: fields
                .into_iter()
                .map(|(name, value)| CompactField {
                    name: name.to_owned(),
                    value,
                    location: location.clone(),
                })
                .collect(),
        });
        Ok(())
    }

    fn one(&self, id: usize) -> Result<usize, Diagnostic> {
        let (_, args) = self.parts(id)?;
        if args.len() != 1 {
            return Err(self.error(id, "clause requires one operand"));
        }
        Ok(args[0])
    }

    fn clause(&self, clauses: &[usize], name: &str) -> Result<Option<usize>, Diagnostic> {
        let mut found = None;
        for id in clauses {
            if self.block.head(*id) == Some(name) && found.replace(*id).is_some() {
                return Err(self.error(*id, format!("duplicate {name} clause")));
            }
        }
        Ok(found)
    }

    fn required_clause(&self, unit: &Unit, name: &str) -> Result<usize, Diagnostic> {
        self.clause(&unit.clauses, name)?
            .ok_or_else(|| self.error(unit.syntax, format!("{} requires ({name} ...)", unit.kind)))
    }

    fn scalar(&self, unit: &Unit, name: &str) -> Result<String, Diagnostic> {
        Ok(self
            .block
            .atom(self.one(self.required_clause(unit, name)?)?)?
            .to_owned())
    }

    fn insert_name(
        &mut self,
        scope: &str,
        class: &str,
        name: &str,
        label: &str,
        at: usize,
    ) -> Result<(), Diagnostic> {
        Name::new(name).map_err(|mut error| {
            error.location = Some(self.block.syntax[at].location.clone());
            error
        })?;
        let path = qualify(scope, name);
        if self
            .names
            .insert((path.clone(), class.to_owned()), label.to_owned())
            .is_some()
        {
            return Err(self.block.error(
                at,
                "change_unit_duplicate",
                format!("duplicate {class} name '{path}'"),
            ));
        }
        Ok(())
    }

    fn collect(&mut self, id: usize, scope: &str, parent: &str) -> Result<(), Diagnostic> {
        let (kind, args) = self.parts(id)?;
        let kind = kind.to_owned();
        let args = args.to_vec();
        if kind == "use" {
            if args.len() != 2 && args.len() != 3 {
                return Err(self.error(
                    id,
                    "use requires alias and builtin, or alias and exact package/revision",
                ));
            }
            let name = self.block.atom(args[0])?.to_owned();
            Name::new(&name)?;
            let label = self.allocate('$')?;
            let mut fields = vec![("as", label.clone())];
            if args.len() == 2 && self.block.atom(args[1])? == "builtin" {
                fields.push(("source", "builtin".into()));
            } else if args.len() == 3 {
                fields.push(("package", self.block.atom(args[1])?.to_owned()));
                fields.push(("package-revision", self.block.atom(args[2])?.to_owned()));
            } else {
                return Err(self.error(id, "use must select an exact supplier"));
            }
            if !scope.is_empty() || self.imports.insert(name, label).is_some() {
                return Err(self.error(
                    id,
                    "supplier aliases must be unique at unit-collection scope",
                ));
            }
            self.record(id, "reference.package", fields)?;
            return Ok(());
        }
        if kind == "type-alias" {
            if args.len() != 2 {
                return Err(self.error(id, "type-alias requires a name and a type"));
            }
            let name = self.block.atom(args[0])?;
            Name::new(name)?;
            if self
                .aliases
                .insert(qualify(scope, name), (scope.to_owned(), args[1]))
                .is_some()
            {
                return Err(self.block.error(
                    id,
                    "change_unit_duplicate",
                    "duplicate structural type alias",
                ));
            }
            return Ok(());
        }
        if kind == "reference" {
            if args.len() != 3 {
                return Err(self.error(
                    id,
                    "reference requires an alias, typed namespace, and exact owner",
                ));
            }
            let alias = self.block.atom(args[0])?.to_owned();
            let class = self.block.atom(args[1])?.to_owned();
            if !COMPACT_NAMESPACE_CLASSES
                .iter()
                .any(|(name, _)| *name == class)
            {
                return Err(self.error(id, "reference has an unknown namespace"));
            }
            let value = self.block.atom(args[2])?.to_owned();
            let exact = value.strip_prefix("parameter:").unwrap_or(&value);
            let owner = exact
                .split_once('/')
                .map(|(package, owner)| {
                    package.parse::<PackageId>()?;
                    owner.parse::<OwnerKey>()
                })
                .unwrap_or_else(|| exact.parse::<OwnerKey>())?;
            let actual = match owner {
                OwnerKey::Declaration(_) => "declaration",
                OwnerKey::Field(_) => "field",
                OwnerKey::Case(_) => "case",
                OwnerKey::Operation(_) => "operation",
                OwnerKey::Requirement(_) => "requirement",
                OwnerKey::Port(_) => "port",
                OwnerKey::TypeParameter(_) => "type-parameter",
                OwnerKey::EffectParameter(_) => "effect-parameter",
                OwnerKey::RequirementParameter(_) => "requirement-parameter",
                OwnerKey::Parameter(_) => "parameter",
                OwnerKey::Module(_) => "module",
                OwnerKey::Target(_) => "target",
                _ => "unsupported",
            };
            if actual != class {
                return Err(self.error(id, "reference exact owner and typed namespace disagree"));
            }
            self.insert_name(scope, &class, &alias, &value, id)?;
            return Ok(());
        }
        let class = namespace(&kind)
            .ok_or_else(|| self.error(id, format!("unknown declaration family '{kind}'")))?;
        let mode = args.first().map(|id| self.block.atom(*id)).transpose()?;
        let (existing, name_index) = match mode {
            Some("create") if args.len() >= 2 => (None, 1),
            Some("edit") if args.len() >= 3 => {
                (Some(self.block.atom(args[1])?.parse::<OwnerKey>()?), 2)
            }
            _ => {
                return Err(self.error(
                    id,
                    "declaration requires explicit create NAME or edit OWNER NAME",
                ));
            }
        };
        let name = self.block.atom(args[name_index])?.to_owned();
        let clauses = args[name_index + 1..].to_vec();
        let label = match self.clause(&clauses, "as")? {
            Some(clause) => {
                let label = self.block.atom(self.one(clause)?)?.to_owned();
                let record = CompactRecord {
                    operation: kind.clone(),
                    fields: vec![],
                    location: self.block.syntax[id].location.clone(),
                };
                validate_local_label(&record, "as", &label, '$')?;
                label
            }
            None => self.allocate('$')?,
        };
        let reference = if let Some(owner) = existing {
            let record = self
                .existing
                .get(&owner)
                .ok_or_else(|| self.error(id, "edit identity is absent"))?;
            let displayed_name = record
                .name()
                .map(ToString::to_string)
                .unwrap_or_else(|| owner.to_string());
            if displayed_name != name || owner_family(record) != kind {
                return Err(self.block.error(id, "change_unit_binding", "displayed kind/name differs from the exact bound owner; use an explicit rename operation"));
            }
            self.edits.insert(label.clone(), owner);
            match class {
                "module" | "parameter" | "type-parameter" | "target" => owner.to_string(),
                _ => format!(
                    "{}/{}",
                    self.package
                        .ok_or_else(|| self.error(id, "missing package binding"))?,
                    owner
                ),
            }
        } else {
            label.clone()
        };
        self.insert_name(scope, class, &name, &reference, id)?;
        let mut unit = Unit {
            syntax: id,
            kind,
            name,
            scope: scope.to_owned(),
            parent: parent.to_owned(),
            label,
            clauses,
            existing,
        };
        if let Some(clause) = self.clause(&unit.clauses, "in")? {
            if !parent.is_empty() {
                return Err(self.error(clause, "nested declaration already has an owning scope"));
            }
            unit.parent = self.block.atom(self.one(clause)?)?.to_owned();
        }
        self.units.push(unit.clone());
        let child_scope = qualify(scope, &unit.name);
        for clause in &unit.clauses {
            let (child_kind, child_args) = self.parts(*clause)?;
            let is_unit = namespace(child_kind).is_some()
                && child_args
                    .first()
                    .and_then(|id| self.block.atom(*id).ok())
                    .is_some_and(|mode| matches!(mode, "create" | "edit"));
            if child_allowed(&unit.kind, child_kind) && !is_unit {
                return Err(self.error(
                    *clause,
                    "contract children require explicit create or edit units",
                ));
            }
            if is_unit || child_kind == "type-alias" {
                if !child_allowed(&unit.kind, child_kind) && child_kind != "type-alias" {
                    return Err(self.error(
                        *clause,
                        format!("{child_kind} is not owned by {}", unit.kind),
                    ));
                }
                self.collect(
                    *clause,
                    &child_scope,
                    &unit
                        .existing
                        .map(|o| o.to_string())
                        .unwrap_or_else(|| unit.label.clone()),
                )?;
            }
        }
        Ok(())
    }

    fn resolve(
        &mut self,
        value: &str,
        scope: &str,
        class: &str,
        at: usize,
    ) -> Result<String, Diagnostic> {
        // Authored lexical names may themselves begin with an owner-ID prefix. Resolve
        // the collected typed scope before considering explicit compact selectors.
        for prefix in scopes(scope) {
            if let Some(label) = self.names.get(&(qualify(prefix, value), class.to_owned())) {
                return Ok(label.clone());
            }
            if class == "requirement"
                && let Some(label) = self
                    .names
                    .get(&(qualify(prefix, value), "requirement-parameter".into()))
            {
                return Ok(if label.starts_with("parameter:") {
                    label.clone()
                } else {
                    format!("parameter:{label}")
                });
            }
        }
        if value.starts_with('$')
            || value.starts_with("pkg_")
            || value.starts_with("decl_")
            || value.starts_with("mod_")
            || value.starts_with("param_")
            || value.starts_with("field_")
            || value.starts_with("case_")
            || value.starts_with("op_")
            || value.starts_with("req_")
            || value.starts_with("port_")
            || value.starts_with("typeparam_")
            || value.starts_with("effectparam_")
            || value.starts_with("reqparam_")
            || value.starts_with("target_")
            || value.starts_with("parameter:")
        {
            return Ok(value.to_owned());
        }
        let segments: Vec<_> = value.split("::").collect();
        let supplier = self.imports.contains_key(segments[0]);
        let (package, names) = if let Some(package) = self.imports.get(segments[0]) {
            (package.clone(), &segments[1..])
        } else {
            ("local".to_owned(), &segments[..])
        };
        let classes = match class {
            "module" => vec!["module"],
            "declaration" if supplier && names.len() == 1 => vec!["declaration"],
            "declaration" => vec!["module", "declaration"],
            "target" => vec!["target"],
            _ if supplier && names.len() == 2 => vec!["declaration", class],
            _ => vec!["module", "declaration", class],
        };
        if names.len() != classes.len() {
            return Err(self.block.error(at, "change_unit_unresolved", format!("unresolved {class} '{value}'; qualify local module::declaration or supplier::declaration and an explicit member; ambiguous exports need an exact typed reference alias")));
        }
        let mut parent: Option<String> = None;
        let mut path = package.clone();
        for (name, class) in names.iter().zip(classes) {
            path = qualify(&path, name);
            let key = (path.clone(), class.to_owned());
            let alias = if let Some(alias) = self.imported.get(&key) {
                alias.clone()
            } else {
                let alias = self.allocate('$')?;
                let mut fields = vec![
                    ("as", alias.clone()),
                    ("package", package.clone()),
                    ("class", class.to_owned()),
                    ("name", (*name).to_owned()),
                ];
                if let Some(parent) = &parent {
                    fields.push(("parent", parent.clone()));
                }
                self.record(at, "reference.owner", fields)?;
                self.imported.insert(key, alias.clone());
                alias
            };
            parent = Some(alias);
        }
        parent.ok_or_else(|| self.error(at, "empty typed reference"))
    }

    fn reference(&mut self, id: usize, scope: &str, class: &str) -> Result<String, Diagnostic> {
        let value = self.block.atom(id)?.to_owned();
        self.resolve(&value, scope, class, id)
    }

    fn ty(&mut self, id: usize, scope: &str, depth: usize) -> Result<String, Diagnostic> {
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(self.error(id, "inline type exceeds the supported type depth"));
        }
        if let Ok(value) = self.block.atom(id) {
            let value = value.to_owned();
            let primitive = match value.as_str() {
                "Unit" => "unit",
                "Bool" => "bool",
                "I64" => "i64",
                "F64" => "f64",
                "Text" => "text",
                "Bytes" => "bytes",
                "StaticText" => "static-text",
                "Secret" => "secret",
                other => other,
            };
            if matches!(
                primitive,
                "unit" | "bool" | "i64" | "f64" | "text" | "bytes" | "static-text" | "secret"
            ) || value.starts_with('@')
            {
                return Ok(primitive.to_owned());
            }
            for prefix in scopes(scope) {
                let path = qualify(prefix, &value);
                if let Some((alias_scope, target)) = self.aliases.get(&path).cloned() {
                    if !self.resolving.insert(path.clone()) {
                        return Err(self.error(id, "recursive structural type alias"));
                    }
                    let result = self.ty(target, &alias_scope, depth + 1);
                    self.resolving.remove(&path);
                    return result;
                }
                if let Some(parameter) = self.names.get(&(path, "type-parameter".into())).cloned() {
                    let label = self.allocate('@')?;
                    self.record(
                        id,
                        "type.parameter",
                        vec![("as", label.clone()), ("parameter", parameter)],
                    )?;
                    return Ok(label);
                }
            }
            let declaration = self.resolve(&value, scope, "declaration", id)?;
            let label = self.allocate('@')?;
            self.record(
                id,
                "type.named",
                vec![("as", label.clone()), ("declaration", declaration)],
            )?;
            return Ok(label);
        }
        let (form, args) = self.parts(id)?;
        let form = form.to_owned();
        let args = args.to_vec();
        let label = self.allocate('@')?;
        let mut fields = vec![("as", label.clone())];
        let operation = match form.as_str() {
            "list" | "option" | "stream" => {
                if args.len() != 1 {
                    return Err(self.error(id, "type constructor requires one type"));
                }
                fields.push(("item", self.ty(args[0], scope, depth + 1)?));
                format!("type.{form}")
            }
            "map" | "result" => {
                if args.len() != 2 {
                    return Err(self.error(id, "type constructor requires two types"));
                }
                let keys = if form == "map" {
                    ["key", "value"]
                } else {
                    ["ok", "error"]
                };
                for (key, arg) in keys.into_iter().zip(args) {
                    fields.push((key, self.ty(arg, scope, depth + 1)?));
                }
                format!("type.{form}")
            }
            "function" | "task-function" => {
                if args.len() != if form == "function" { 2 } else { 3 } {
                    return Err(self.error(id, "function type requires parameter list, result, and an explicit row for task-function"));
                }
                let params = self.block.list(args[0])?.to_vec();
                for (index, param) in params.into_iter().enumerate() {
                    let ty = self.ty(param, scope, depth + 1)?;
                    self.record(
                        param,
                        "type.argument",
                        vec![
                            ("parent", label.clone()),
                            ("index", index.to_string()),
                            ("type", ty),
                        ],
                    )?;
                }
                fields.push(("result", self.ty(args[1], scope, depth + 1)?));
                if form == "task-function" {
                    fields.push(("effect", self.row(args[2], scope)?));
                }
                format!("type.{form}")
            }
            "record" => {
                for (index, field) in args.into_iter().enumerate() {
                    let (name, types) = self.parts(field)?;
                    let name = name.to_owned();
                    let types = types.to_vec();
                    if types.len() != 1 {
                        return Err(self.error(field, "structural field requires name and type"));
                    }
                    let ty = self.ty(types[0], scope, depth + 1)?;
                    self.record(
                        field,
                        "type.field",
                        vec![
                            ("parent", label.clone()),
                            ("index", index.to_string()),
                            ("name", name),
                            ("type", ty),
                        ],
                    )?;
                }
                "type.structural-record".into()
            }
            "resource" | "parameter-type" => {
                if args.len() != 1 {
                    return Err(self.error(id, "type requires one typed owner"));
                }
                let (operation, field, class) = if form == "resource" {
                    ("type.capability-resource", "interface", "declaration")
                } else {
                    ("type.parameter", "parameter", "type-parameter")
                };
                fields.push((field, self.reference(args[0], scope, class)?));
                operation.into()
            }
            _ => {
                let declaration = self.resolve(&form, scope, "declaration", id)?;
                fields.push(("declaration", declaration));
                for (index, arg) in args.into_iter().enumerate() {
                    let ty = self.ty(arg, scope, depth + 1)?;
                    self.record(
                        arg,
                        "type.argument",
                        vec![
                            ("parent", label.clone()),
                            ("index", index.to_string()),
                            ("type", ty),
                        ],
                    )?;
                }
                "type.application".into()
            }
        };
        self.record(id, &operation, fields)?;
        Ok(label)
    }

    fn row(&mut self, id: usize, scope: &str) -> Result<String, Diagnostic> {
        let label = self.allocate('@')?;
        self.record(id, "effect.row", vec![("as", label.clone())])?;
        self.effect_edges(id, scope, &label)?;
        Ok(label)
    }

    fn effect_edges(&mut self, id: usize, scope: &str, parent: &str) -> Result<(), Diagnostic> {
        let (kind, args) = self.parts(id)?;
        if !matches!(kind, "task" | "row") {
            return Err(self.error(id, "effect row requires (task ...) or (row ...)"));
        }
        let args = args.to_vec();
        let mut requirements = 0;
        let mut parameters = 0;
        for arg in args {
            let (kind, values) = self.parts(arg)?;
            let kind = kind.to_owned();
            let values = values.to_vec();
            if values.len() != 1 {
                return Err(self.error(arg, "effect member requires one typed reference"));
            }
            let (index, field, class) = match kind.as_str() {
                "requirement" => {
                    let n = requirements;
                    requirements += 1;
                    (n, "requirement", "requirement")
                }
                "parameter" => {
                    let n = parameters;
                    parameters += 1;
                    (n, "parameter", "effect-parameter")
                }
                _ => return Err(self.error(arg, "effect member must be requirement or parameter")),
            };
            let reference = self.reference(values[0], scope, class)?;
            self.record(
                arg,
                &format!("effect.{field}"),
                vec![
                    ("parent", parent.to_owned()),
                    ("index", index.to_string()),
                    (field, reference),
                ],
            )?;
        }
        Ok(())
    }

    fn body(&mut self, id: usize, scope: &str) -> Result<String, Diagnostic> {
        let label = self.allocate('$')?;
        let mut block = self.block.clone();
        block.root = id;
        for ((path, class), value) in &self.names {
            if class == "parameter"
                && path
                    .rsplit_once("::")
                    .is_some_and(|(parent, _)| parent == scope)
                && let Some((_, name)) = path.rsplit_once("::")
            {
                block.locals.insert(name.to_owned(), value.clone());
            }
        }
        // Transform only the typed operand positions of the structural grammar, never arbitrary
        // atom/text substitution. The existing layout still owns lexical binding and shadowing.
        let mut pending = vec![id];
        while let Some(node) = pending.pop() {
            let (form, args) = self.parts(node)?;
            let form = form.to_owned();
            let args = args.to_vec();
            let mut reference = |index: usize, class: &str| -> Result<(), Diagnostic> {
                if let Some(arg) = args.get(index) {
                    let value = self.reference(*arg, scope, class)?;
                    block.syntax[*arg].kind = SyntaxKind::Atom {
                        value,
                        quoted: false,
                    };
                }
                Ok(())
            };
            match form.as_str() {
                "call" | "function-value" | "constant" => reference(0, "declaration")?,
                "variant" => reference(0, "case")?,
                "capability-call" => {
                    reference(0, "requirement")?;
                    reference(1, "operation")?;
                }
                "transaction" | "transaction-outcome" => reference(0, "requirement")?,
                _ => {}
            }
            match form.as_str() {
                "types" | "type" => {
                    for arg in &args {
                        let value = self.ty(*arg, scope, 1)?;
                        block.syntax[*arg].kind = SyntaxKind::Atom {
                            value,
                            quoted: false,
                        };
                    }
                    continue;
                }
                "effects" => {
                    for arg in &args {
                        let value = self.row(*arg, scope)?;
                        block.syntax[*arg].kind = SyntaxKind::Atom {
                            value,
                            quoted: false,
                        };
                    }
                    continue;
                }
                "requirements" => {
                    for arg in &args {
                        let value = self.reference(*arg, scope, "requirement")?;
                        block.syntax[*arg].kind = SyntaxKind::Atom {
                            value,
                            quoted: false,
                        };
                    }
                    continue;
                }
                "list" | "map" => {
                    for arg in args.iter().take(if form == "list" { 1 } else { 2 }) {
                        let value = self.ty(*arg, scope, 1)?;
                        block.syntax[*arg].kind = SyntaxKind::Atom {
                            value,
                            quoted: false,
                        };
                    }
                }
                "record" => {
                    if let Some(arg) = args.first()
                        && self.block.atom(*arg)? != "structural"
                    {
                        let value = self.reference(*arg, scope, "declaration")?;
                        block.syntax[*arg].kind = SyntaxKind::Atom {
                            value,
                            quoted: false,
                        };
                        for field in &args[1..] {
                            if self.block.head(*field) == Some("field") {
                                let (_, fields) = self.parts(*field)?;
                                if let Some(field) = fields.first().copied() {
                                    let value = self.reference(field, scope, "field")?;
                                    block.syntax[field].kind = SyntaxKind::Atom {
                                        value,
                                        quoted: false,
                                    };
                                }
                            }
                        }
                    }
                }
                "field"
                    if args.len() == 2
                        && matches!(self.block.syntax[args[0]].kind, SyntaxKind::List(_)) =>
                {
                    if self.block.head(args[1]) != Some("name") {
                        let value = self.reference(args[1], scope, "field")?;
                        block.syntax[args[1]].kind = SyntaxKind::Atom {
                            value,
                            quoted: false,
                        };
                    }
                }
                "payload" if args.len() >= 2 => {
                    let value = self.ty(args[1], scope, 1)?;
                    block.syntax[args[1]].kind = SyntaxKind::Atom {
                        value,
                        quoted: false,
                    };
                }
                "outcome" if args.len() == 6 => {
                    for (index, arg) in args.iter().enumerate() {
                        let value = self.reference(
                            *arg,
                            scope,
                            if index < 2 { "declaration" } else { "case" },
                        )?;
                        block.syntax[*arg].kind = SyntaxKind::Atom {
                            value,
                            quoted: false,
                        };
                    }
                }
                "arm" => {
                    if let Some(arg) = args.first() {
                        let value = self.reference(*arg, scope, "case")?;
                        block.syntax[*arg].kind = SyntaxKind::Atom {
                            value,
                            quoted: false,
                        };
                    }
                }
                _ => {}
            }
            for arg in args.into_iter().rev() {
                if matches!(&block.syntax[arg].kind, SyntaxKind::List(_)) {
                    pending.push(arg);
                }
            }
        }
        self.record(id, "expression.block", vec![("as", label.clone())])?;
        self.bodies.insert(label.clone(), block.subtree(id)?);
        Ok(label)
    }

    fn emit(&mut self, unit: &Unit) -> Result<(), Diagnostic> {
        let scope = qualify(&unit.scope, &unit.name);
        let mut fields = vec![("as", unit.label.clone()), ("name", unit.name.clone())];
        let mut allowed = vec!["as", "type-alias"];
        let kind = unit.kind.as_str();
        if !matches!(kind, "module" | "target") {
            allowed.push("in");
        }
        if namespace(kind) == Some("declaration") {
            fields.push((
                "module",
                self.resolve(&unit.parent, &unit.scope, "module", unit.syntax)?,
            ));
            fields.push(("visibility", self.scalar(unit, "visibility")?));
            allowed.push("visibility");
        }
        let operation = match kind {
            "module" | "record" | "variant" | "interface" | "component" => format!("create.{kind}"),
            "function" | "external" => {
                let ty = self.one(self.required_clause(unit, "returns")?)?;
                fields.push(("result", self.ty(ty, &scope, 1)?));
                allowed.push("returns");
                if kind == "external" {
                    fields.push(("implementation", self.scalar(unit, "implementation")?));
                    allowed.push("implementation");
                } else {
                    let effect = self.one(self.required_clause(unit, "effect")?)?;
                    let effect_name = if self.block.atom(effect).ok() == Some("pure") {
                        "pure"
                    } else {
                        self.effect_edges(effect, &scope, &unit.label)?;
                        "task"
                    };
                    fields.push(("effect", effect_name.to_owned()));
                    let body = self.one(self.required_clause(unit, "body")?)?;
                    fields.push(("body", self.body(body, &scope)?));
                    allowed.extend(["effect", "body"]);
                }
                format!("create.{kind}")
            }
            "constant" => {
                let ty = self.one(self.required_clause(unit, "type")?)?;
                fields.push(("type", self.ty(ty, &scope, 1)?));
                let value = self.one(self.required_clause(unit, "value")?)?;
                fields.push(("value", self.body(value, &scope)?));
                allowed.extend(["type", "value"]);
                "create.constant".into()
            }
            "test" => {
                for name in ["actual", "expected"] {
                    let id = self.one(self.required_clause(unit, name)?)?;
                    fields.push((name, self.body(id, &scope)?));
                }
                allowed.extend(["actual", "expected"]);
                "create.test".into()
            }
            "field" | "case" | "parameter" => {
                let (parent_key, class) = match kind {
                    "field" => ("record", "declaration"),
                    "case" => ("variant", "declaration"),
                    _ => {
                        let operation = self.units.iter().any(|other| {
                            (other.label == unit.parent
                                || other
                                    .existing
                                    .is_some_and(|owner| owner.to_string() == unit.parent))
                                && other.kind == "operation"
                        });
                        (
                            if operation { "operation" } else { "function" },
                            if operation {
                                "operation"
                            } else {
                                "declaration"
                            },
                        )
                    }
                };
                fields.push((
                    parent_key,
                    self.resolve(&unit.parent, &unit.scope, class, unit.syntax)?,
                ));
                let type_key = if kind == "case" { "payload" } else { "type" };
                match self.clause(&unit.clauses, type_key)? {
                    Some(clause) => {
                        let ty = self.ty(self.one(clause)?, &unit.scope, 1)?;
                        fields.push((type_key, ty));
                    }
                    None if kind != "case" => {
                        return Err(
                            self.error(unit.syntax, format!("{kind} requires ({type_key} TYPE)"))
                        );
                    }
                    None => {}
                }
                allowed.push(type_key);
                if kind == "parameter" {
                    if self.clause(&unit.clauses, "use")?.is_some() {
                        fields.push(("use", self.scalar(unit, "use")?));
                    }
                    if let Some(clause) = self.clause(&unit.clauses, "requirement")? {
                        fields.push((
                            "requirement",
                            self.reference(self.one(clause)?, &unit.scope, "requirement")?,
                        ));
                    }
                    allowed.extend(["use", "requirement"]);
                }
                format!("add.{kind}")
            }
            "type-parameter" | "effect-parameter" | "requirement-parameter" => {
                fields.push(("declaration", unit.parent.clone()));
                if kind == "type-parameter" {
                    if self.clause(&unit.clauses, "constraint")?.is_some() {
                        fields.push(("constraint", self.scalar(unit, "constraint")?));
                    }
                    allowed.push("constraint");
                }
                if kind == "requirement-parameter" {
                    fields.push((
                        "interface",
                        self.reference(
                            self.one(self.required_clause(unit, "interface")?)?,
                            &unit.scope,
                            "declaration",
                        )?,
                    ));
                    self.operations(unit, "requirement-parameter.operation")?;
                    allowed.extend(["interface", "operations"]);
                }
                format!("add.{kind}")
            }
            "operation" => {
                fields.push(("interface", unit.parent.clone()));
                fields.push((
                    "result",
                    self.ty(self.one(self.required_clause(unit, "returns")?)?, &scope, 1)?,
                ));
                fields.push(("idempotency", self.scalar(unit, "idempotency")?));
                fields.push((
                    "external-visibility",
                    self.scalar(unit, "external-visibility")?,
                ));
                allowed.extend(["returns", "idempotency", "external-visibility"]);
                "add.operation".into()
            }
            "requirement" => {
                fields.push(("component", unit.parent.clone()));
                fields.push((
                    "interface",
                    self.reference(
                        self.one(self.required_clause(unit, "interface")?)?,
                        &unit.scope,
                        "declaration",
                    )?,
                ));
                self.operations(unit, "requirement.operation")?;
                let limits = self.required_clause(unit, "limits")?;
                for (index, limit) in self.parts(limits)?.1.to_vec().into_iter().enumerate() {
                    let parts = self.block.list(limit)?;
                    if parts.len() != 3 {
                        return Err(self.error(limit, "limit requires name maximum unit"));
                    }
                    self.record(
                        limit,
                        "requirement.limit",
                        vec![
                            ("parent", unit.label.clone()),
                            ("index", index.to_string()),
                            ("name", self.block.atom(parts[0])?.to_owned()),
                            ("maximum", self.block.atom(parts[1])?.to_owned()),
                            ("unit", self.block.atom(parts[2])?.to_owned()),
                        ],
                    )?;
                }
                allowed.extend(["interface", "operations", "limits"]);
                "add.requirement".into()
            }
            "port" => {
                fields.push(("component", unit.parent.clone()));
                fields.push((
                    "type",
                    self.ty(
                        self.one(self.required_clause(unit, "type")?)?,
                        &unit.scope,
                        1,
                    )?,
                ));
                match (
                    self.clause(&unit.clauses, "function")?,
                    self.clause(&unit.clauses, "value")?,
                ) {
                    (Some(clause), None) => fields.push((
                        "function",
                        self.reference(self.one(clause)?, &unit.scope, "declaration")?,
                    )),
                    (None, Some(clause)) => {
                        fields.push(("value", self.body(self.one(clause)?, &unit.scope)?))
                    }
                    _ => {
                        return Err(self.error(
                            unit.syntax,
                            "port requires exactly one of (function REF) or (value EXPR)",
                        ));
                    }
                }
                allowed.extend(["type", "function", "value"]);
                "add.port".into()
            }
            "target" => {
                fields.push((
                    "component",
                    self.reference(
                        self.one(self.required_clause(unit, "component")?)?,
                        &unit.scope,
                        "declaration",
                    )?,
                ));
                fields.push(("runner", self.scalar(unit, "runner")?));
                if let Some(clause) = self.clause(&unit.clauses, "port")? {
                    fields.push((
                        "port",
                        self.reference(self.one(clause)?, &unit.scope, "port")?,
                    ));
                }
                allowed.extend(["component", "runner", "port"]);
                "create.target".into()
            }
            "route" => {
                fields.retain(|(key, _)| *key != "name");
                fields.push(("target", unit.parent.clone()));
                fields.push(("method", self.scalar(unit, "method")?));
                match (
                    self.clause(&unit.clauses, "path")?,
                    self.clause(&unit.clauses, "pattern")?,
                ) {
                    (Some(clause), None) => {
                        fields.push(("path", self.block.text(self.one(clause)?)?.to_owned()))
                    }
                    (None, Some(clause)) => {
                        fields.push(("pattern", self.block.text(self.one(clause)?)?.to_owned()))
                    }
                    _ => {
                        return Err(self
                            .error(unit.syntax, "route requires exactly one of path or pattern"));
                    }
                }
                fields.push((
                    "port",
                    self.reference(
                        self.one(self.required_clause(unit, "port")?)?,
                        &unit.scope,
                        "port",
                    )?,
                ));
                allowed.extend(["method", "path", "pattern", "port"]);
                "add.http-route".into()
            }
            _ => return Err(self.error(unit.syntax, "unimplemented declaration family")),
        };
        for clause in &unit.clauses {
            let name = self.parts(*clause)?.0;
            if !allowed.contains(&name) && !child_allowed(kind, name) {
                return Err(self.error(*clause, format!("unknown {kind} clause '{name}'")));
            }
        }
        self.record(unit.syntax, &operation, fields)?;
        Ok(())
    }

    fn operations(&mut self, unit: &Unit, operation: &str) -> Result<(), Diagnostic> {
        let clause = self.required_clause(unit, "operations")?;
        for (index, id) in self.parts(clause)?.1.to_vec().into_iter().enumerate() {
            let reference = self.reference(id, &unit.scope, "operation")?;
            self.record(
                id,
                operation,
                vec![
                    ("parent", unit.label.clone()),
                    ("index", index.to_string()),
                    ("operation", reference),
                ],
            )?;
        }
        Ok(())
    }

    fn check_contracts(&self, precise: &[CompactRecord]) -> Result<(), Diagnostic> {
        use crate::platform::kernel::OwnerRecord;
        for unit in &self.units {
            let Some(owner) = unit.existing else { continue };
            let record = &self.existing[&owner];
            if let Some(namespace) = crate::platform::kernel::owner_namespace(record)
                && let Some(parent) = namespace.parent
                && unit.parent != parent.to_string()
            {
                return Err(self.block.error(
                    unit.syntax,
                    "change_unit_parent",
                    "edit unit changed its exact owning scope; use an explicit move operation",
                ));
            }
            if matches!(record, OwnerRecord::Module(_)) {
                continue;
            }
            if let OwnerRecord::HttpRoute(route) = record
                && unit.parent != route.target.to_string()
            {
                return Err(self.error(unit.syntax, "route edit changed its exact target"));
            }
            let mut expected: BTreeSet<_> = contract_children(record).into_iter().collect();
            if let OwnerKey::Target(target) = owner {
                expected.extend(self.existing.iter().filter_map(|(id, record)| {
                    matches!(record, OwnerRecord::HttpRoute(route) if route.target == target)
                        .then_some(*id)
                }));
            }
            let supplied: BTreeSet<_> = self
                .units
                .iter()
                .filter(|u| u.parent == owner.to_string())
                .filter_map(|u| u.existing)
                .collect();
            let deleted: BTreeSet<_> = precise
                .iter()
                .filter(|r| r.operation == "delete.owner")
                .filter_map(|r| optional(r, "owner").and_then(|s| s.parse::<OwnerKey>().ok()))
                .collect();
            if supplied != expected.difference(&deleted).copied().collect() {
                return Err(self.block.error(unit.syntax, "change_unit_children", "complete edit signature must retain every exact contract child, or account for it with explicit delete.owner"));
            }
            // Existing ordered contracts cannot be silently reordered, nor may an append-only
            // addition be displayed as an insertion. The precise owner operations remain available.
            for kind in [
                "parameter",
                "type-parameter",
                "effect-parameter",
                "requirement-parameter",
            ] {
                let ordered: Vec<_> = contract_children(record)
                    .into_iter()
                    .filter(|o| {
                        !deleted.contains(o)
                            && self
                                .existing
                                .get(o)
                                .is_some_and(|r| owner_family(r) == kind)
                    })
                    .collect();
                let children: Vec<_> = self
                    .units
                    .iter()
                    .filter(|u| u.parent == owner.to_string() && u.kind == kind)
                    .collect();
                let actual: Vec<_> = children.iter().filter_map(|u| u.existing).collect();
                let mut addition_seen = false;
                for child in children {
                    if child.existing.is_none() {
                        addition_seen = true;
                    } else if addition_seen {
                        return Err(self.block.error(
                            child.syntax,
                            "change_unit_order",
                            "new ordered contract children append after retained exact children",
                        ));
                    }
                }
                if actual != ordered {
                    return Err(self.block.error(
                        unit.syntax,
                        "change_unit_order",
                        format!("existing {kind} order differs from its bound signature"),
                    ));
                }
            }
        }
        Ok(())
    }
}

pub(super) fn contract_children(record: &crate::platform::kernel::OwnerRecord) -> Vec<OwnerKey> {
    use crate::platform::kernel::{DeclarationPayload as D, OwnerRecord as O};
    let mut children = Vec::new();
    match record {
        O::Declaration(d) => match &d.payload {
            D::Record {
                type_parameters,
                fields,
            } => {
                children.extend(type_parameters.iter().copied().map(OwnerKey::TypeParameter));
                children.extend(fields.iter().copied().map(OwnerKey::Field));
            }
            D::Variant {
                type_parameters,
                cases,
            } => {
                children.extend(type_parameters.iter().copied().map(OwnerKey::TypeParameter));
                children.extend(cases.iter().copied().map(OwnerKey::Case));
            }
            D::Function(f) => {
                children.extend(
                    f.type_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::TypeParameter),
                );
                children.extend(f.parameters.iter().copied().map(OwnerKey::Parameter));
                children.extend(
                    f.effect_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::EffectParameter),
                );
                children.extend(
                    f.requirement_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::RequirementParameter),
                );
            }
            D::External(f) => {
                children.extend(
                    f.type_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::TypeParameter),
                );
                children.extend(f.parameters.iter().copied().map(OwnerKey::Parameter));
            }
            D::Interface { operations } => {
                children.extend(operations.iter().copied().map(OwnerKey::Operation))
            }
            D::Component {
                requirements,
                ports,
            } => {
                children.extend(requirements.iter().copied().map(OwnerKey::Requirement));
                children.extend(ports.iter().copied().map(OwnerKey::Port));
            }
            _ => {}
        },
        O::Operation(o) => children.extend(o.parameters.iter().copied().map(OwnerKey::Parameter)),
        _ => {}
    }
    children
}

pub(super) fn owner_family(record: &crate::platform::kernel::OwnerRecord) -> &'static str {
    use crate::platform::kernel::{DeclarationPayload as D, OwnerRecord as O};
    match record {
        O::Module(_) => "module",
        O::Declaration(d) => match &d.payload {
            D::Record { .. } => "record",
            D::Variant { .. } => "variant",
            D::Interface { .. } => "interface",
            D::Function(_) => "function",
            D::External(_) => "external",
            D::Constant { .. } => "constant",
            D::Component { .. } => "component",
            D::Test { .. } => "test",
        },
        O::Field(_) => "field",
        O::Case(_) => "case",
        O::Parameter(_) => "parameter",
        O::TypeParameter(_) => "type-parameter",
        O::EffectParameter(_) => "effect-parameter",
        O::RequirementParameter(_) => "requirement-parameter",
        O::Operation(_) => "operation",
        O::Requirement(_) => "requirement",
        O::Port(_) => "port",
        O::Target(_) => "target",
        O::HttpRoute(_) => "route",
        _ => "unsupported",
    }
}

pub(super) fn finish(
    mut request: NormalizedChangeRequest,
    lowered: Lowered,
    reader: Option<&mut canonical::Reader<'_>>,
) -> Result<NormalizedChangeRequest, Diagnostic> {
    if lowered.edits.is_empty() {
        return Ok(request);
    }
    let reader =
        reader.ok_or_else(|| canonical::error("edit normalization requires its exact base"))?;
    let mut changes = Vec::new();
    for change in request.semantic.changes {
        let symbol = match &change {
            AuthoredChange::CreateModule { symbol, .. }
            | AuthoredChange::CreateRecord { symbol, .. }
            | AuthoredChange::CreateVariant { symbol, .. }
            | AuthoredChange::CreateInterface { symbol, .. }
            | AuthoredChange::CreateExternal { symbol, .. }
            | AuthoredChange::CreateFunction { symbol, .. }
            | AuthoredChange::CreateConstant { symbol, .. }
            | AuthoredChange::CreateComponent { symbol, .. }
            | AuthoredChange::CreateTest { symbol, .. }
            | AuthoredChange::CreateTarget { symbol, .. }
            | AuthoredChange::AddHttpRoute { symbol, .. } => Some(symbol),
            AuthoredChange::AddField { field, .. } => Some(&field.symbol),
            AuthoredChange::AddCase { case, .. } => Some(&case.symbol),
            AuthoredChange::AddOperation { operation, .. } => Some(&operation.symbol),
            AuthoredChange::AddParameter { parameter, .. } => Some(&parameter.symbol),
            AuthoredChange::AddTypeParameter { parameter, .. } => Some(&parameter.symbol),
            AuthoredChange::AddEffectParameter { parameter, .. } => Some(&parameter.symbol),
            AuthoredChange::AddRequirementParameter { parameter, .. } => Some(&parameter.symbol),
            AuthoredChange::AddRequirement { requirement, .. } => Some(&requirement.symbol),
            AuthoredChange::AddPort { port, .. } => Some(&port.symbol),
            _ => None,
        };
        let Some(owner) = symbol.and_then(|s| lowered.edits.get(s)).copied() else {
            changes.push(change);
            continue;
        };
        let exact = OwnerSelector::Exact { owner };
        let declaration = match owner {
            OwnerKey::Declaration(declaration) => Some(DeclarationSelector::Id { declaration }),
            _ => None,
        };
        match change {
            AuthoredChange::CreateModule { name, .. }
            | AuthoredChange::AddEffectParameter {
                parameter: AuthoredEffectParameter { name, .. },
                ..
            } => changes.push(AuthoredChange::RenameOwner { owner: exact, name }),
            AuthoredChange::CreateRecord { visibility, .. }
            | AuthoredChange::CreateVariant { visibility, .. }
            | AuthoredChange::CreateInterface { visibility, .. }
            | AuthoredChange::CreateComponent { visibility, .. } => {
                changes.push(AuthoredChange::SetDeclarationVisibility {
                    declaration: declaration
                        .ok_or_else(|| canonical::error("expected declaration identity"))?,
                    visibility,
                })
            }
            AuthoredChange::CreateFunction {
                visibility,
                result,
                effect,
                body,
                ..
            } => {
                let function =
                    declaration.ok_or_else(|| canonical::error("expected function identity"))?;
                changes.push(AuthoredChange::SetDeclarationVisibility {
                    declaration: function.clone(),
                    visibility,
                });
                changes.push(AuthoredChange::SetFunctionContract {
                    function: function.clone(),
                    result,
                    effect,
                });
                let crate::platform::kernel::OwnerRecord::Declaration(old) = reader.owner(owner)?
                else {
                    return Err(canonical::error("expected function owner"));
                };
                let crate::platform::kernel::DeclarationPayload::Function(old) = old.payload else {
                    return Err(canonical::error("expected function payload"));
                };
                let previous = reader.expression(old.body)?;
                if let Some(change) = super::literal_edit::function_body(
                    reader,
                    function,
                    body,
                    previous,
                    request.semantic.base,
                    request.semantic.budget,
                )? {
                    changes.push(change);
                }
            }
            AuthoredChange::CreateExternal {
                visibility,
                result,
                implementation,
                ..
            } => {
                let external =
                    declaration.ok_or_else(|| canonical::error("expected external identity"))?;
                changes.push(AuthoredChange::SetDeclarationVisibility {
                    declaration: external.clone(),
                    visibility,
                });
                changes.push(AuthoredChange::SetExternalContract {
                    external,
                    result,
                    implementation,
                });
            }
            AuthoredChange::CreateConstant {
                visibility,
                ty,
                value,
                ..
            } => {
                let constant =
                    declaration.ok_or_else(|| canonical::error("expected constant identity"))?;
                changes.push(AuthoredChange::SetDeclarationVisibility {
                    declaration: constant.clone(),
                    visibility,
                });
                let crate::platform::kernel::OwnerRecord::Declaration(old) = reader.owner(owner)?
                else {
                    return Err(canonical::error("expected constant owner"));
                };
                let crate::platform::kernel::DeclarationPayload::Constant {
                    ty: old_type,
                    value: old_value,
                } = old.payload
                else {
                    return Err(canonical::error("expected constant payload"));
                };
                let previous = AuthoredChange::SetConstant {
                    constant: constant.clone(),
                    ty: reader.ty(old_type)?,
                    value: reader.expression(old_value)?,
                };
                let proposed = AuthoredChange::SetConstant {
                    constant,
                    ty,
                    value,
                };
                if !equivalent(
                    &proposed,
                    &previous,
                    request.semantic.base,
                    request.semantic.budget,
                )? {
                    changes.push(proposed);
                }
            }
            AuthoredChange::CreateTest {
                visibility,
                actual,
                expected,
                ..
            } => {
                let test = declaration.ok_or_else(|| canonical::error("expected test identity"))?;
                changes.push(AuthoredChange::SetDeclarationVisibility {
                    declaration: test.clone(),
                    visibility,
                });
                let crate::platform::kernel::OwnerRecord::Declaration(old) = reader.owner(owner)?
                else {
                    return Err(canonical::error("expected test owner"));
                };
                let crate::platform::kernel::DeclarationPayload::Test {
                    actual: old_actual,
                    expected: old_expected,
                    ..
                } = old.payload
                else {
                    return Err(canonical::error("expected test payload"));
                };
                let previous = AuthoredChange::SetTest {
                    test: test.clone(),
                    actual: reader.expression(old_actual)?,
                    expected: reader.expression(old_expected)?,
                };
                let proposed = AuthoredChange::SetTest {
                    test,
                    actual,
                    expected,
                };
                if !equivalent(
                    &proposed,
                    &previous,
                    request.semantic.base,
                    request.semantic.budget,
                )? {
                    changes.push(proposed);
                }
            }
            AuthoredChange::AddField { field, .. } => changes.push(AuthoredChange::SetFieldType {
                field: exact,
                ty: field.ty,
            }),
            AuthoredChange::AddCase { case, .. } => changes.push(AuthoredChange::SetCasePayload {
                case: exact,
                payload: case.payload,
            }),
            AuthoredChange::AddParameter { parameter, .. } => {
                let crate::platform::kernel::OwnerRecord::Parameter(old) = reader.owner(owner)?
                else {
                    return Err(canonical::error("expected parameter owner"));
                };
                let requirement =
                    old.resource_requirement
                        .map(|r| AuthoredRequirementReference::Exact {
                            package: r.package,
                            requirement: r.requirement,
                        });
                if parameter.use_mode != old.use_mode
                    || parameter.resource_requirement != requirement
                {
                    return Err(canonical::error(
                        "changing an existing parameter use mode/resource binding requires a supported explicit contract operation",
                    ));
                }
                changes.push(AuthoredChange::SetParameterType {
                    parameter: exact,
                    ty: parameter.ty,
                });
            }
            AuthoredChange::AddTypeParameter { parameter, .. } => {
                changes.push(AuthoredChange::SetTypeParameterConstraint {
                    parameter: exact,
                    constraints: parameter.constraints,
                })
            }
            AuthoredChange::AddRequirementParameter { parameter, .. } => {
                changes.push(AuthoredChange::SetRequirementParameter {
                    parameter: exact,
                    interface: parameter.interface,
                    operations: parameter.operations,
                })
            }
            AuthoredChange::AddOperation { operation, .. } => {
                changes.push(AuthoredChange::SetOperationContract {
                    operation: exact,
                    result: operation.result,
                    idempotency: operation.idempotency,
                    external_visibility: operation.external_visibility,
                })
            }
            AuthoredChange::AddRequirement { requirement, .. } => {
                changes.push(AuthoredChange::SetRequirementContract {
                    requirement: exact,
                    interface: requirement.interface,
                    operations: requirement.operations,
                    limits: requirement.limits,
                })
            }
            AuthoredChange::AddPort { port, .. } => {
                let crate::platform::kernel::OwnerRecord::Port(old) = reader.owner(owner)? else {
                    return Err(canonical::error("expected port owner"));
                };
                let implementation = match old.implementation {
                    crate::platform::kernel::PortImplementation::Function(old) => {
                        AuthoredPortImplementation::Function {
                            function: canonical::declaration(old),
                        }
                    }
                    crate::platform::kernel::PortImplementation::Expression(old) => {
                        AuthoredPortImplementation::Expression {
                            expression: reader.expression(old)?,
                        }
                    }
                };
                let previous = AuthoredChange::SetPort {
                    port: exact.clone(),
                    function_type: reader.ty(old.function_type)?,
                    implementation,
                };
                let proposed = AuthoredChange::SetPort {
                    port: exact,
                    function_type: port.function_type,
                    implementation: port.implementation,
                };
                if !equivalent(
                    &proposed,
                    &previous,
                    request.semantic.base,
                    request.semantic.budget,
                )? {
                    changes.push(proposed);
                }
            }
            AuthoredChange::CreateTarget {
                component,
                port,
                runner,
                ..
            } => changes.push(AuthoredChange::SetTarget {
                target: exact,
                component,
                port,
                runner,
            }),
            AuthoredChange::AddHttpRoute {
                method,
                selector,
                port,
                ..
            } => changes.push(AuthoredChange::SetHttpRoute {
                route: exact,
                method,
                selector,
                port,
            }),
            _ => {
                return Err(canonical::error(
                    "this native edit family is not implemented yet",
                ));
            }
        }
    }
    request.semantic.changes = changes;
    normalize_change_request(request.semantic, request.options)
}

fn equivalent(
    proposed: &AuthoredChange,
    previous: &AuthoredChange,
    base: RevisionId,
    budget: crate::platform::change::ChangeBudget,
) -> Result<bool, Diagnostic> {
    let encode = |change: &AuthoredChange| {
        crate::platform::change::canonical_authored_intent_bytes(&AuthoredChangeSet {
            base,
            preconditions: Vec::new(),
            changes: vec![change.clone()],
            budget,
        })
    };
    Ok(encode(proposed).ok() == Some(encode(previous)?))
}

fn qualify(scope: &str, name: &str) -> String {
    if scope.is_empty() {
        name.to_owned()
    } else {
        format!("{scope}::{name}")
    }
}

fn scopes(mut scope: &str) -> Vec<&str> {
    let mut result = vec![scope];
    while let Some((parent, _)) = scope.rsplit_once("::") {
        result.push(parent);
        scope = parent;
    }
    if !scope.is_empty() {
        result.push("");
    }
    result
}

fn namespace(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "module" => "module",
        "record" | "variant" | "interface" | "external" | "function" | "constant" | "component"
        | "test" => "declaration",
        "field" => "field",
        "case" => "case",
        "parameter" => "parameter",
        "operation" => "operation",
        "type-parameter" => "type-parameter",
        "effect-parameter" => "effect-parameter",
        "requirement-parameter" => "requirement-parameter",
        "requirement" => "requirement",
        "port" => "port",
        "target" => "target",
        "route" => "route",
        _ => return None,
    })
}

fn child_allowed(parent: &str, child: &str) -> bool {
    match parent {
        "module" => namespace(child) == Some("declaration"),
        "record" => matches!(child, "field" | "type-parameter"),
        "variant" => matches!(child, "case" | "type-parameter"),
        "interface" => child == "operation",
        "function" => matches!(
            child,
            "parameter" | "type-parameter" | "effect-parameter" | "requirement-parameter"
        ),
        "external" => matches!(child, "parameter" | "type-parameter"),
        "operation" => child == "parameter",
        "component" => matches!(child, "port" | "requirement"),
        "target" => child == "route",
        _ => false,
    }
}
