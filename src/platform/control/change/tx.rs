//! Bounded symbolic transactions lower into the ordinary typed change engine.

use super::input::{Block, ChangeInput, Syntax, SyntaxKind};
use super::*;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::NamespaceClass;
use crate::platform::publication::RepositoryView;
use crate::platform::witness::NamespaceKey;

pub(crate) const MAXIMUM_TRANSACTION_RESULT_BYTES: usize = 65_536;

pub(crate) fn decode(
    path: &str,
    bytes: &[u8],
    view: &RepositoryView,
    control: &ExecutionControl,
) -> Result<NormalizedChangeRequest, Vec<Diagnostic>> {
    decode_inner(path, bytes, view, control).map_err(|e| vec![e])
}

fn decode_inner(
    path: &str,
    bytes: &[u8],
    view: &RepositoryView,
    control: &ExecutionControl,
) -> Result<NormalizedChangeRequest, Diagnostic> {
    let mut block = input::parse_expression(path, bytes)?;
    let root = block.root;
    let parts = block.list(root)?.to_vec();
    if block.head(root) != Some("tx") {
        return Err(block.error(
            root,
            "transaction_form",
            "expected (tx (base @head) UNITS...)",
        ));
    }
    let mut reader = canonical::Reader::for_intent(view, control.clone());
    let uses_standard = block.syntax.iter().any(|node| {
        matches!(&node.kind,
        SyntaxKind::Atom { value, quoted: false } if value.starts_with("std::"))
    });
    let mut declares_standard_module = false;
    let mut units = Vec::new();
    let mut base = None;
    let mut policies = Vec::new();
    for id in &parts[1..] {
        if block.head(*id) == Some("base") {
            let fields = block.list(*id)?;
            if fields.len() != 2 || base.is_some() {
                return Err(block.error(
                    *id,
                    "transaction_base",
                    "transaction requires exactly one base selector",
                ));
            }
            let value = block.atom(fields[1])?;
            let revision = if value == "@head" {
                view.revision()
            } else {
                value.parse()?
            };
            if revision != view.revision() {
                return Err(block.error(
                    *id,
                    "transaction_stale",
                    "transaction base differs from current HEAD",
                ));
            }
            base = Some(revision);
        } else if block.head(*id) == Some("policy") {
            let fields = block.list(*id)?;
            if fields.len() != 3 {
                return Err(block.error(*id, "transaction_policy", "policy requires one dimension and unbounded or an exact u64 allowance"));
            }
            let location = block.syntax[*id].location.clone();
            policies.push(CompactRecord { operation: "change.policy".into(), fields: vec![
                CompactField { name: "name".into(), value: block.atom(fields[1])?.to_owned(), location: location.clone() },
                CompactField { name: "limit".into(), value: block.atom(fields[2])?.to_owned(), location: location.clone() },
            ], location });
        } else {
            units.push(*id);
        }
    }
    let base = base.ok_or_else(|| {
        block.error(
            root,
            "transaction_base",
            "transaction requires one base selector",
        )
    })?;
    if units.is_empty() {
        return Err(block.error(
            root,
            "transaction_empty",
            "transaction requires at least one declaration unit",
        ));
    }
    // The native lowerer owns scope, contracts, ordered children and every body/type rule.
    // This pass supplies only create/edit identity selections from one immutable base.
    let mut pending = units
        .iter()
        .rev()
        .map(|id| (*id, None, String::new()))
        .collect::<Vec<_>>();
    while let Some((id, parent, scope)) = pending.pop() {
        reader.check()?;
        let fields = block.list(id)?.to_vec();
        let head = block
            .head(id)
            .ok_or_else(|| {
                block.error(
                    id,
                    "transaction_unit",
                    "transaction unit requires a declaration family",
                )
            })?
            .to_owned();
        if matches!(head.as_str(), "use" | "type-alias" | "reference") {
            continue;
        }
        let kind = if head == "param" {
            "parameter"
        } else {
            head.strip_prefix("upsert-").ok_or_else(|| {
                block.error(
                    id,
                    "transaction_unit",
                    "transaction units require upsert-KIND or param",
                )
            })?
        };
        let class = declarations::namespace(kind)
            .and_then(|name| NamespaceClass::parse(&name.replace('-', "_")).ok())
            .ok_or_else(|| block.error(id, "transaction_unit", "unsupported upsert namespace"))?;
        let (name, explicit, consumed) = selected_name(&block, &fields[1..], kind, &mut reader)?;
        let name = match name.rsplit_once("::") {
            Some((prefix, leaf)) if prefix == scope => leaf.to_owned(),
            Some(_) => {
                return Err(block.error(
                    id,
                    "transaction_scope",
                    "qualified upsert name differs from its enclosing scope",
                ));
            }
            None => name,
        };
        let name = Name::new(&name)?;
        declares_standard_module |= scope.is_empty() && kind == "module" && name.as_str() == "std";
        let existing = if parent.is_some() || scope.is_empty() {
            reader.namespace(&NamespaceKey {
                parent,
                class,
                name: name.clone(),
            })?
        } else {
            None
        };
        if let Some(selected) = explicit
            && Some(selected) != existing
        {
            return Err(block.error(
                id,
                "transaction_selector_scope",
                "explicit selector does not belong to this upsert namespace",
            ));
        }
        if let Some(owner) = existing
            && declarations::owner_family(&reader.owner(owner)?) != kind
        {
            return Err(block.error(
                id,
                "transaction_selector_kind",
                "existing name belongs to a different declaration kind",
            ));
        }
        let mut clauses = fields[1 + consumed..].to_vec();
        if head == "param" {
            if clauses.len() != 1 {
                return Err(block.error(id, "transaction_parameter", "param requires NAME TYPE"));
            }
            let type_head = atom(&mut block, id, "type")?;
            let ty = node(
                &mut block,
                id,
                SyntaxKind::List(vec![type_head, clauses[0]]),
            )?;
            clauses = vec![ty];
        }
        if class == NamespaceClass::Declaration
            && !clauses
                .iter()
                .any(|id| block.head(*id) == Some("visibility"))
        {
            let visibility = match existing.map(|owner| reader.owner(owner)).transpose()? {
                Some(crate::platform::kernel::OwnerRecord::Declaration(declaration)) => {
                    match declaration.visibility {
                        crate::platform::kernel::DeclarationVisibility::Public => "public",
                        crate::platform::kernel::DeclarationVisibility::Private => "private",
                        crate::platform::kernel::DeclarationVisibility::Package => "package",
                    }
                }
                _ => "private",
            };
            let head = atom(&mut block, id, "visibility")?;
            let value = atom(&mut block, id, visibility)?;
            clauses.push(node(&mut block, id, SyntaxKind::List(vec![head, value]))?);
        }
        let path = if scope.is_empty() {
            name.to_string()
        } else {
            format!("{scope}::{name}")
        };
        for child in clauses.iter().rev() {
            if block
                .head(*child)
                .is_some_and(|h| h.starts_with("upsert-") || h == "param")
            {
                pending.push((*child, existing, path.clone()));
            }
        }
        let mut replacement = vec![
            atom(&mut block, id, kind)?,
            atom(
                &mut block,
                id,
                if existing.is_some() { "edit" } else { "create" },
            )?,
        ];
        if let Some(owner) = existing {
            replacement.push(atom(&mut block, id, &owner.to_string())?);
        }
        replacement.push(atom(&mut block, id, name.as_str())?);
        replacement.extend(clauses);
        block.syntax[id].kind = SyntaxKind::List(replacement);
    }
    // Explicit selectors in expressions are exact typed references. Quoted text is untouched.
    for id in 0..block.syntax.len() {
        let SyntaxKind::List(parts) = &block.syntax[id].kind else {
            continue;
        };
        let parts = parts.clone();
        let mut replacement = Vec::with_capacity(parts.len());
        let mut cursor = 0;
        while cursor < parts.len() {
            let selector = block.atom(parts[cursor]).ok();
            if matches!(selector, Some("@module" | "@fn")) {
                let kind = if selector == Some("@module") {
                    "module"
                } else {
                    "function"
                };
                let (_, owner, consumed) =
                    selected_name(&block, &parts[cursor..], kind, &mut reader)?;
                let owner = owner.ok_or_else(|| {
                    block.error(
                        id,
                        "transaction_selector",
                        "explicit selector omitted its owner",
                    )
                })?;
                let value = match owner {
                    OwnerKey::Module(_) => owner.to_string(),
                    _ => format!("{}/{}", view.package(), owner),
                };
                replacement.push(atom(&mut block, id, &value)?);
                cursor += consumed;
            } else {
                replacement.push(parts[cursor]);
                cursor += 1;
            }
        }
        block.syntax[id].kind = SyntaxKind::List(replacement);
    }
    let has_standard_alias = units.iter().any(|id| {
        block.head(*id) == Some("use")
            && block
                .list(*id)
                .ok()
                .and_then(|p| p.get(1))
                .and_then(|id| block.atom(*id).ok())
                == Some("std")
    });
    if uses_standard
        && !declares_standard_module
        && !has_standard_alias
        && reader
            .namespace(&NamespaceKey {
                parent: None,
                class: NamespaceClass::Module,
                name: Name::new("std")?,
            })?
            .is_none()
    {
        let use_head = atom(&mut block, root, "use")?;
        let alias = atom(&mut block, root, "std")?;
        let source = atom(&mut block, root, "builtin")?;
        units.insert(
            0,
            node(
                &mut block,
                root,
                SyntaxKind::List(vec![use_head, alias, source]),
            )?,
        );
    }
    let units_head = atom(&mut block, root, "units")?;
    units.insert(0, units_head);
    block.syntax[root].kind = SyntaxKind::List(units);
    let location = block.syntax[root].location.clone();
    let mut public_labels = BTreeSet::new();
    for syntax in &block.syntax {
        if let SyntaxKind::Atom {
            value,
            quoted: false,
        } = &syntax.kind
        {
            input::reserve_labels(value, &mut public_labels);
        }
    }
    let mut records = vec![CompactRecord {
            operation: "request".into(),
            fields: vec![CompactField {
                name: "base".into(),
                value: base.to_string(),
                location: location.clone(),
            }],
            location,
        }];
    records.extend(policies);
    let parsed = ChangeInput {
        records,
        blocks: BTreeMap::new(),
        units: vec![block],
        public_labels,
    };
    decode_parsed_change(parsed, Some(&mut reader)).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .unwrap_or_else(|| canonical::error("transaction normalization failed"))
    })
}

fn selected_name(
    block: &Block,
    fields: &[usize],
    kind: &str,
    reader: &mut canonical::Reader<'_>,
) -> Result<(String, Option<OwnerKey>, usize), Diagnostic> {
    let first = fields
        .first()
        .copied()
        .ok_or_else(|| block.error(block.root, "transaction_name", "upsert requires a name"))?;
    let selector = block.atom(first)?;
    if !selector.starts_with('@') {
        return Ok((selector.to_owned(), None, 1));
    }
    let expected = match selector {
        "@module" => "module",
        "@fn" => "function",
        _ => "",
    };
    if kind != expected || expected.is_empty() {
        return Err(block.error(
            first,
            "transaction_selector_kind",
            "selector kind differs from its position",
        ));
    }
    let argument = fields.get(1).copied().ok_or_else(|| {
        block.error(
            first,
            "transaction_selector",
            "selector requires a quoted path",
        )
    })?;
    let args = block.list(argument)?;
    let [value] = args else {
        return Err(block.error(
            argument,
            "transaction_selector",
            "selector requires one path",
        ));
    };
    let SyntaxKind::Atom {
        value,
        quoted: true,
    } = &block.syntax[*value].kind
    else {
        return Err(block.error(
            argument,
            "transaction_selector",
            "selector path requires double quotes",
        ));
    };
    let names = value.split("::").collect::<Vec<_>>();
    let classes = if kind == "module" {
        &[NamespaceClass::Module][..]
    } else {
        &[NamespaceClass::Module, NamespaceClass::Declaration][..]
    };
    if names.len() != classes.len() {
        return Err(block.error(
            argument,
            "transaction_selector",
            "selector path has the wrong number of scopes",
        ));
    }
    let mut owner = None;
    for (name, class) in names.iter().zip(classes) {
        owner = reader.namespace(&NamespaceKey {
            parent: owner,
            class: *class,
            name: Name::new(*name)?,
        })?;
        if owner.is_none() {
            return Err(block.error(
                argument,
                "transaction_selector_missing",
                "explicit selector is absent at the bound HEAD",
            ));
        }
    }
    if let Some(owner) = owner
        && declarations::owner_family(&reader.owner(owner)?) != kind
    {
        return Err(block.error(
            argument,
            "transaction_selector_kind",
            "explicit selector names a different declaration kind",
        ));
    }
    Ok((value.clone(), owner, 2))
}

fn atom(block: &mut Block, at: usize, value: &str) -> Result<usize, Diagnostic> {
    node(
        block,
        at,
        SyntaxKind::Atom {
            value: value.to_owned(),
            quoted: false,
        },
    )
}

fn node(block: &mut Block, at: usize, kind: SyntaxKind) -> Result<usize, Diagnostic> {

    block.syntax.try_reserve(1).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "transaction_syntax_capacity",
            "transaction syntax allocation failed",
        )
    })?;
    let id = block.syntax.len();
    block.syntax.push(Syntax {
        kind,
        location: block.syntax[at].location.clone(),
    });
    Ok(id)
}
