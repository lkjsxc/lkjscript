//! Structural notation lowers directly into authored intent. Arena indices, lexical references
//! and public selector tokens remain distinct until all source references have been resolved.

use super::input::{Block, SyntaxKind};
use super::*;

#[derive(Default)]
pub(super) struct PrivateSymbols {
    reserved: BTreeSet<String>,
    next: usize,
    pub locations: BTreeMap<String, SourceLocation>,
    identities: usize,
}

impl PrivateSymbols {
    pub fn new(reserved: BTreeSet<String>, records: &[CompactRecord]) -> Result<Self, Diagnostic> {
        let mut symbols = Self {
            reserved,
            ..Self::default()
        };
        for record in records {
            if !record.operation.starts_with("reference.") {
                for field in &record.fields {
                    if matches!(field.name.as_str(), "as" | "binding")
                        && field.value.starts_with('$')
                    {
                        symbols.admit(&field.location)?;
                    }
                }
            }
        }
        Ok(symbols)
    }

    fn admit(&mut self, location: &SourceLocation) -> Result<(), Diagnostic> {
        let maximum = crate::platform::change::ChangeBudget::default()
            .authored
            .maximum_allocated_identities;
        if self.identities as u64 >= maximum {
            return Err(capacity(
                location,
                "change_budget_allocated_identities",
                format!("complete change input exceeds its {maximum}-identity authored admission"),
            ));
        }
        self.identities += 1;
        Ok(())
    }

    fn allocate(&mut self, location: &SourceLocation) -> Result<String, Diagnostic> {
        self.admit(location)?;
        loop {
            let symbol = format!("$__block_{}", self.next);
            self.next = self.next.checked_add(1).ok_or_else(|| {
                capacity(
                    location,
                    "change_block_capacity",
                    "private symbol ordinal overflowed",
                )
            })?;
            if !self.reserved.contains(&symbol) {
                self.locations.insert(symbol.clone(), location.clone());
                return Ok(symbol);
            }
        }
    }
}

pub(super) struct Body {
    nodes: BTreeMap<usize, Node>,
    pub depth: usize,
    root: usize,
}

struct Node {
    symbol: String,
    record: CompactRecord,
    children: Vec<usize>,
    types: Vec<CompactField>,
    effects: Vec<CompactField>,
    requirements: Vec<CompactField>,
    members: Vec<CompactRecord>,
    // Only lexical resolution constructs this value. User text always uses the public decoder.
    lexical: Option<String>,
}

enum Work {
    Expression { id: usize, depth: usize },
    Enter { name: String, symbol: String },
    Leave(String),
}

pub(super) fn layout(
    block: Block,
    root_symbol: &str,
    symbols: &mut PrivateSymbols,
) -> Result<Body, Diagnostic> {
    let mut nodes = BTreeMap::new();
    let mut work = vec![Work::Expression {
        id: block.root,
        depth: 1,
    }];
    let mut environment: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut maximum_depth = 0;
    while let Some(task) = work.pop() {
        let (id, depth) = match task {
            Work::Enter { name, symbol } => {
                environment.entry(name).or_default().push(symbol);
                continue;
            }
            Work::Leave(name) => {
                if let Some(values) = environment.get_mut(&name) {
                    values.pop();
                }
                continue;
            }
            Work::Expression { id, depth } => (id, depth),
        };
        if depth > crate::platform::kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(capacity(
                &block.syntax[id].location,
                "change_authored_expression_depth",
                "authored expression exceeds the maximum structural depth",
            ));
        }
        maximum_depth = maximum_depth.max(depth);
        let items = block.list(id)?;
        let head = items.first().copied().ok_or_else(|| {
            block.error(
                id,
                "change_block_form",
                "an expression requires a form name",
            )
        })?;
        let form = block.atom(head)?;
        let args = &items[1..];
        let symbol = if id == block.root {
            root_symbol.to_owned()
        } else {
            symbols.allocate(&block.syntax[id].location)?
        };
        let mut node = Node {
            symbol,
            record: CompactRecord {
                operation: format!("expression.{form}"),
                fields: Vec::new(),
                location: block.syntax[id].location.clone(),
            },
            children: Vec::new(),
            types: Vec::new(),
            effects: Vec::new(),
            requirements: Vec::new(),
            members: Vec::new(),
            lexical: None,
        };
        let mut scoped = false;
        match form {
            "unit" => arity(&block, id, args, 0)?,
            "bool" | "i64" | "constant" => {
                arity(&block, id, args, 1)?;
                node.record.fields.push(block.field(
                    args[0],
                    if form == "constant" {
                        "declaration"
                    } else {
                        "value"
                    },
                )?);
            }
            "text" | "static-text" => {
                arity(&block, id, args, 1)?;
                let SyntaxKind::Atom {
                    value,
                    quoted: true,
                } = &block.syntax[args[0]].kind
                else {
                    return Err(block.error(
                        args[0],
                        "change_block_text",
                        "text literals require double quotes",
                    ));
                };
                node.record.fields.push(CompactField {
                    name: "value".to_owned(),
                    value: value.clone(),
                    location: block.syntax[args[0]].location.clone(),
                });
            }
            "local" => {
                arity(&block, id, args, 1)?;
                if block.head(args[0]) == Some("exact") {
                    let exact = clause(&block, args[0], "exact")?;
                    arity(&block, args[0], exact, 1)?;
                    let field = block.field(exact[0], "value")?;
                    if !field.value.starts_with(ParameterId::PREFIX)
                        && !field.value.starts_with(BindingId::PREFIX)
                    {
                        return Err(block.error(
                            exact[0],
                            "change_local_reference",
                            "exact local requires param_ID or bind_ID",
                        ));
                    }
                    node.record.fields.push(field);
                } else {
                    let value = block.atom(args[0])?;
                    if value.starts_with('$') {
                        node.record.fields.push(block.field(args[0], "value")?);
                    } else {
                        let name = name(&block, args[0])?;
                        node.lexical = Some(environment.get(name.as_str()).and_then(|values| values.last()).cloned()
                            .ok_or_else(|| block.error(args[0], "change_block_local_unbound", format!("lexical local '{value}' is not bound in this block scope")))?);
                    }
                }
            }
            "if" => {
                arity(&block, id, args, 3)?;
                node.children.extend_from_slice(args);
            }
            "sequence" => node.children.extend_from_slice(args),
            "call" | "function-value" => {
                minimum(&block, id, args, 1)?;
                node.record.fields.push(block.field(args[0], "function")?);
                let rest = applications(&block, &args[1..], &mut node, true)?;
                if form == "function-value" {
                    arity(&block, id, rest, 0)?;
                }
                node.children.extend_from_slice(rest);
            }
            "invoke" | "bind" => {
                minimum(&block, id, args, 1)?;
                node.children.extend_from_slice(args);
            }
            "let" => {
                minimum(&block, id, args, 1)?;
                let body = clause(&block, args[args.len() - 1], "in")?;
                arity(&block, args[args.len() - 1], body, 1)?;
                let mut binding_actions = Vec::new();
                for binding in &args[..args.len() - 1] {
                    let parts = clause(&block, *binding, "binding")?;
                    if !(parts.len() == 2 || parts.len() == 3) {
                        return Err(block.error(
                            *binding,
                            "change_block_arity",
                            "binding requires a name, optional (type TYPE), and one initializer",
                        ));
                    }
                    let binder_name = name(&block, parts[0])?;
                    let binder_symbol = symbols.allocate(&block.syntax[parts[0]].location)?;
                    let mut record = member_record(&block, *binding);
                    record.fields.push(block.field(parts[0], "name")?);
                    record.fields.push(CompactField {
                        name: "as".to_owned(),
                        value: binder_symbol.clone(),
                        location: block.syntax[parts[0]].location.clone(),
                    });
                    if parts.len() == 3 {
                        let annotation = clause(&block, parts[1], "type")?;
                        arity(&block, parts[1], annotation, 1)?;
                        record.fields.push(block.field(annotation[0], "type")?);
                    }
                    let initializer = parts[parts.len() - 1];
                    node.children.push(initializer);
                    node.members.push(record);
                    binding_actions.push((
                        binder_name.as_str().to_owned(),
                        binder_symbol,
                        initializer,
                    ));
                }
                node.children.push(body[0]);
                // Push in reverse execution order. An initializer sees the preceding environment;
                // its binder is introduced only after the initializer has been resolved.
                for (name, _, _) in &binding_actions {
                    work.push(Work::Leave(name.clone()));
                }
                work.push(Work::Expression {
                    id: body[0],
                    depth: depth + 1,
                });
                for (name, symbol, initializer) in binding_actions.into_iter().rev() {
                    work.push(Work::Enter { name, symbol });
                    work.push(Work::Expression {
                        id: initializer,
                        depth: depth + 1,
                    });
                }
                scoped = true;
            }
            "record" => {
                minimum(&block, id, args, 1)?;
                let structural = block.atom(args[0])? == "structural";
                let fields = if structural {
                    &args[1..]
                } else {
                    node.record.fields.push(block.field(args[0], "type")?);
                    applications(&block, &args[1..], &mut node, false)?
                };
                for field in fields {
                    let parts = clause(&block, *field, "field")?;
                    arity(&block, *field, parts, 2)?;
                    let mut record = member_record(&block, *field);
                    if structural {
                        name(&block, parts[0])?;
                    }
                    record
                        .fields
                        .push(block.field(parts[0], if structural { "name" } else { "field" })?);
                    node.members.push(record);
                    node.children.push(parts[1]);
                }
            }
            "variant" => {
                minimum(&block, id, args, 1)?;
                node.record.fields.push(block.field(args[0], "case")?);
                let payload = applications(&block, &args[1..], &mut node, false)?;
                if payload.len() > 1 {
                    return Err(block.error(
                        id,
                        "change_block_arity",
                        "variant accepts at most one payload",
                    ));
                }
                node.children.extend_from_slice(payload);
            }
            "field" => {
                arity(&block, id, args, 2)?;
                node.children.push(args[0]);
                if block.head(args[1]) == Some("name") {
                    let selected = clause(&block, args[1], "name")?;
                    arity(&block, args[1], selected, 1)?;
                    name(&block, selected[0])?;
                    node.record.fields.push(block.field(selected[0], "name")?);
                } else {
                    node.record.fields.push(block.field(args[1], "field")?);
                }
            }
            "list" => {
                minimum(&block, id, args, 1)?;
                node.record.fields.push(block.field(args[0], "item")?);
                node.children.extend_from_slice(&args[1..]);
            }
            "map" => {
                minimum(&block, id, args, 2)?;
                node.record.fields.push(block.field(args[0], "key")?);
                node.record.fields.push(block.field(args[1], "value")?);
                for entry in &args[2..] {
                    let parts = clause(&block, *entry, "entry")?;
                    arity(&block, *entry, parts, 2)?;
                    node.children.extend_from_slice(parts);
                }
            }
            "match" => {
                minimum(&block, id, args, 1)?;
                node.children.push(args[0]);
                let mut arm_actions = Vec::new();
                for arm in &args[1..] {
                    let parts = clause(&block, *arm, "arm")?;
                    if !(parts.len() == 2 || parts.len() == 3) {
                        return Err(block.error(
                            *arm,
                            "change_block_arity",
                            "arm requires a case, optional payload binding, and body",
                        ));
                    }
                    let mut record = member_record(&block, *arm);
                    record.fields.push(block.field(parts[0], "case")?);
                    let binding = if parts.len() == 3 {
                        let payload = clause(&block, parts[1], "payload")?;
                        arity(&block, parts[1], payload, 2)?;
                        let binder_name = name(&block, payload[0])?;
                        let binder_symbol = symbols.allocate(&block.syntax[payload[0]].location)?;
                        record.fields.push(block.field(payload[0], "name")?);
                        record.fields.push(block.field(payload[1], "type")?);
                        record.fields.push(CompactField {
                            name: "as".to_owned(),
                            value: binder_symbol.clone(),
                            location: block.syntax[payload[0]].location.clone(),
                        });
                        Some((binder_name.as_str().to_owned(), binder_symbol))
                    } else {
                        None
                    };
                    let body = parts[parts.len() - 1];
                    node.children.push(body);
                    node.members.push(record);
                    arm_actions.push((body, binding));
                }
                for (body, binding) in arm_actions.into_iter().rev() {
                    if let Some((name, _)) = &binding {
                        work.push(Work::Leave(name.clone()));
                    }
                    work.push(Work::Expression {
                        id: body,
                        depth: depth + 1,
                    });
                    if let Some((name, symbol)) = binding {
                        work.push(Work::Enter { name, symbol });
                    }
                }
                work.push(Work::Expression {
                    id: args[0],
                    depth: depth + 1,
                });
                scoped = true;
            }
            "capability-call" => {
                minimum(&block, id, args, 2)?;
                node.record
                    .fields
                    .push(block.field(args[0], "requirement")?);
                node.record.fields.push(block.field(args[1], "operation")?);
                node.children.extend_from_slice(&args[2..]);
            }
            "transaction" | "transaction-outcome" => {
                let outcome = form == "transaction-outcome";
                arity(&block, id, args, if outcome { 5 } else { 3 })?;
                node.record
                    .fields
                    .push(block.field(args[0], "requirement")?);
                let binder_index = if outcome { 3 } else { 1 };
                if outcome {
                    let types = clause(&block, args[1], "types")?;
                    arity(&block, args[1], types, 1)?;
                    node.record.fields.push(block.field(types[0], "type")?);
                    let references = clause(&block, args[2], "outcome")?;
                    arity(&block, args[2], references, 6)?;
                    for (value, field) in references.iter().zip([
                        "outcome",
                        "abort-reason",
                        "committed",
                        "aborted",
                        "condition-failed",
                        "conflict",
                    ]) {
                        node.record.fields.push(block.field(*value, field)?);
                    }
                }
                let binder = clause(&block, args[binder_index], "binding")?;
                arity(&block, args[binder_index], binder, 1)?;
                let binder_name = name(&block, binder[0])?;
                let binder_symbol = symbols.allocate(&block.syntax[binder[0]].location)?;
                node.record.fields.push(block.field(binder[0], "name")?);
                node.record.fields.push(CompactField {
                    name: "binding".to_owned(),
                    value: binder_symbol.clone(),
                    location: block.syntax[binder[0]].location.clone(),
                });
                node.children.push(args[binder_index + 1]);
                work.push(Work::Leave(binder_name.as_str().to_owned()));
                work.push(Work::Expression {
                    id: args[binder_index + 1],
                    depth: depth + 1,
                });
                work.push(Work::Enter {
                    name: binder_name.as_str().to_owned(),
                    symbol: binder_symbol,
                });
                scoped = true;
            }
            _ => {
                return Err(block.error(
                    head,
                    "change_block_form",
                    format!("unknown structural expression form '{form}'"),
                ));
            }
        }
        if !scoped {
            for child in node.children.iter().rev() {
                work.push(Work::Expression {
                    id: *child,
                    depth: depth + 1,
                });
            }
        }
        nodes.insert(id, node);
    }
    Ok(Body {
        nodes,
        depth: maximum_depth,
        root: block.root,
    })
}

fn member_record(block: &Block, id: usize) -> CompactRecord {
    CompactRecord {
        operation: "structural.member".to_owned(),
        fields: Vec::new(),
        location: block.syntax[id].location.clone(),
    }
}

fn name(block: &Block, id: usize) -> Result<Name, Diagnostic> {
    Name::new(block.atom(id)?).map_err(|mut error| {
        error.location = Some(block.syntax[id].location.clone());
        error
    })
}

fn clause<'a>(block: &'a Block, id: usize, expected: &str) -> Result<&'a [usize], Diagnostic> {
    let items = block.list(id)?;
    if items.first().and_then(|id| block.atom(*id).ok()) != Some(expected) {
        return Err(block.error(
            id,
            "change_block_clause",
            format!("expected ({expected} ...) clause"),
        ));
    }
    Ok(&items[1..])
}

fn arity(block: &Block, id: usize, args: &[usize], expected: usize) -> Result<(), Diagnostic> {
    if args.len() != expected {
        return Err(block.error(
            id,
            "change_block_arity",
            format!("form requires {expected} operands; observed {}", args.len()),
        ));
    }
    Ok(())
}

fn minimum(block: &Block, id: usize, args: &[usize], expected: usize) -> Result<(), Diagnostic> {
    if args.len() < expected {
        return Err(block.error(
            id,
            "change_block_arity",
            format!("form requires at least {expected} operands"),
        ));
    }
    Ok(())
}

fn applications<'a>(
    block: &Block,
    args: &'a [usize],
    node: &mut Node,
    callable: bool,
) -> Result<&'a [usize], Diagnostic> {
    let mut previous = 0;
    let mut consumed = 0;
    for id in args {
        let Some(head) = block.head(*id) else { break };
        let rank = match head {
            "types" => 1,
            "effects" => 2,
            "requirements" => 3,
            _ => break,
        };
        if rank <= previous || (!callable && rank != 1) {
            return Err(block.error(*id, "change_block_application", "application clauses must occur at most once in types/effects/requirements order and only where supported"));
        }
        previous = rank;
        let (target, field) = match rank {
            1 => (&mut node.types, "type"),
            2 => (&mut node.effects, "effect"),
            _ => (&mut node.requirements, "requirement"),
        };
        for atom in clause(block, *id, head)? {
            target.push(block.field(*atom, field)?);
        }
        consumed += 1;
    }
    Ok(&args[consumed..])
}

impl Body {
    pub fn lower(self, decoder: &mut Decoder) -> Result<AuthoredExpression, Diagnostic> {
        let mut values = BTreeMap::new();
        // Syntax arena indices are preorder. Reverse order constructs only already-admitted
        // descendants; errors drop no tree deeper than the complete combined preflight permits.
        for (id, node) in self.nodes.into_iter().rev() {
            let value = lower_node(node, decoder, &mut values)?;
            values.insert(id, value);
        }
        values.remove(&self.root).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "change_block_inventory",
                "structural root was lost during lowering",
            )
        })
    }
}

fn lower_node(
    node: Node,
    decoder: &mut Decoder,
    values: &mut BTreeMap<usize, AuthoredExpression>,
) -> Result<AuthoredExpression, Diagnostic> {
    let record = &node.record;
    let mut children = node.children.iter();
    let mut child = || -> Result<AuthoredExpression, Diagnostic> {
        children
            .next()
            .and_then(|id| values.remove(id))
            .ok_or_else(|| inventory_error(record, "structural child was lost during lowering"))
    };
    let types = node
        .types
        .iter()
        .map(|field| decoder.decode_type_field(field))
        .collect::<Result<Vec<_>, _>>()?;
    let effects = node
        .effects
        .iter()
        .map(|field| {
            decoder
                .decode_effect_row(&field.value)
                .map_err(|mut error| {
                    if error.location.is_none() {
                        error.location = Some(field.location.clone());
                    }
                    error
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let requirements = node
        .requirements
        .iter()
        .map(|field| {
            decoder.parse_requirement_reference(
                &CompactRecord {
                    operation: "requirements".to_owned(),
                    fields: vec![field.clone()],
                    location: field.location.clone(),
                },
                "requirement",
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let operation = match record.operation.as_str() {
        "expression.unit" => AuthoredExpressionOperation::Unit {},
        "expression.bool" => AuthoredExpressionOperation::Bool {
            value: parse_bool(record, "value")?,
        },
        "expression.i64" => AuthoredExpressionOperation::I64 {
            value: parse_field(record, "value")?,
        },
        "expression.text" => AuthoredExpressionOperation::Text {
            value: required(record, "value")?.to_owned(),
        },
        "expression.static-text" => AuthoredExpressionOperation::StaticText {
            value: required(record, "value")?.to_owned(),
        },
        "expression.local" => AuthoredExpressionOperation::Local {
            value: match node.lexical {
                Some(symbol) => AuthoredLocalReference::Symbol { symbol },
                None => decoder.parse_local_reference(record, "value")?,
            },
        },
        "expression.constant" => AuthoredExpressionOperation::Constant {
            declaration: decoder.parse_declaration_reference(record, "declaration")?,
        },
        "expression.if" => AuthoredExpressionOperation::If {
            condition: Box::new(child()?),
            when_true: Box::new(child()?),
            when_false: Box::new(child()?),
        },
        "expression.sequence" => AuthoredExpressionOperation::Sequence {
            items: (0..node.children.len())
                .map(|_| child())
                .collect::<Result<_, _>>()?,
        },
        "expression.call" => AuthoredExpressionOperation::Call {
            function: decoder.parse_declaration_reference(record, "function")?,
            type_arguments: types,
            effect_arguments: effects,
            requirement_arguments: requirements,
            arguments: (0..node.children.len())
                .map(|_| child())
                .collect::<Result<_, _>>()?,
        },
        "expression.function-value" => AuthoredExpressionOperation::FunctionValue {
            function: decoder.parse_declaration_reference(record, "function")?,
            type_arguments: types,
            effect_arguments: effects,
            requirement_arguments: requirements,
        },
        "expression.invoke" | "expression.bind" => {
            let callee = Box::new(child()?);
            let arguments = (1..node.children.len())
                .map(|_| child())
                .collect::<Result<_, _>>()?;
            if record.operation == "expression.invoke" {
                AuthoredExpressionOperation::Invoke { callee, arguments }
            } else {
                AuthoredExpressionOperation::Bind { callee, arguments }
            }
        }
        "expression.let" => {
            let mut bindings = Vec::new();
            for member in &node.members {
                bindings.push(AuthoredLetBinding {
                    symbol: symbol(member, "as")?,
                    name: parse_name(member, "name")?,
                    declared_type: field(member, "type")
                        .map(|field| decoder.decode_type_field(field))
                        .transpose()?,
                    value: child()?,
                });
            }
            AuthoredExpressionOperation::Let {
                bindings,
                body: Box::new(child()?),
            }
        }
        "expression.record" => {
            let mut fields = Vec::new();
            for member in &node.members {
                fields.push(AuthoredRecordExpressionField {
                    selector: decoder.parse_field_selector(member)?,
                    value: child()?,
                });
            }
            AuthoredExpressionOperation::Record {
                nominal_type: optional(record, "type")
                    .map(|_| decoder.parse_declaration_reference(record, "type"))
                    .transpose()?,
                type_arguments: types,
                fields,
            }
        }
        "expression.variant" => AuthoredExpressionOperation::Variant {
            case: decoder.parse_case_reference(record, "case")?,
            type_arguments: types,
            payload: if node.children.is_empty() {
                None
            } else {
                Some(Box::new(child()?))
            },
        },
        "expression.field" => AuthoredExpressionOperation::Field {
            value: Box::new(child()?),
            selector: decoder.parse_field_selector(record)?,
        },
        "expression.list" => AuthoredExpressionOperation::List {
            item_type: decoder.decode_type_field(required_field(record, "item")?)?,
            items: (0..node.children.len())
                .map(|_| child())
                .collect::<Result<_, _>>()?,
        },
        "expression.map" => {
            let mut entries = Vec::new();
            for _ in 0..node.children.len() / 2 {
                entries.push(AuthoredMapExpressionEntry {
                    key: child()?,
                    value: child()?,
                });
            }
            AuthoredExpressionOperation::Map {
                key_type: decoder.decode_type_field(required_field(record, "key")?)?,
                value_type: decoder.decode_type_field(required_field(record, "value")?)?,
                entries,
            }
        }
        "expression.match" => {
            let value = Box::new(child()?);
            let mut arms = Vec::new();
            for member in &node.members {
                let payload_binding = field(member, "as")
                    .map(|_| -> Result<_, Diagnostic> {
                        Ok(AuthoredBindingDefinition {
                            symbol: symbol(member, "as")?,
                            name: parse_name(member, "name")?,
                            declared_type: Some(
                                decoder.decode_type_field(required_field(member, "type")?)?,
                            ),
                        })
                    })
                    .transpose()?;
                arms.push(AuthoredMatchExpressionArm {
                    case: decoder.parse_case_reference(member, "case")?,
                    payload_binding,
                    body: child()?,
                });
            }
            AuthoredExpressionOperation::Match { value, arms }
        }
        "expression.capability-call" => AuthoredExpressionOperation::CapabilityCall {
            requirement: decoder.parse_requirement_reference(record, "requirement")?,
            operation: decoder.parse_operation_reference(record, "operation")?,
            arguments: (0..node.children.len())
                .map(|_| child())
                .collect::<Result<_, _>>()?,
        },
        "expression.transaction" => AuthoredExpressionOperation::Transaction {
            requirement: decoder.parse_requirement_reference(record, "requirement")?,
            binding: AuthoredBindingDefinition {
                symbol: symbol(record, "binding")?,
                name: parse_name(record, "name")?,
                declared_type: None,
            },
            body: Box::new(child()?),
        },
        "expression.transaction-outcome" => AuthoredExpressionOperation::TransactionOutcome {
            requirement: decoder.parse_requirement_reference(record, "requirement")?,
            binding: AuthoredBindingDefinition {
                symbol: symbol(record, "binding")?,
                name: parse_name(record, "name")?,
                declared_type: None,
            },
            body: Box::new(child()?),
            type_argument: Box::new(decoder.decode_type_field(required_field(record, "type")?)?),
            outcome: decoder.decode_transaction_outcome(record)?,
        },
        _ => {
            return Err(inventory_error(
                record,
                "unrecognized lowered structural form",
            ));
        }
    };
    Ok(AuthoredExpression {
        symbol: Some(node.symbol),
        operation,
    })
}

fn required_field<'a>(
    record: &'a CompactRecord,
    name: &str,
) -> Result<&'a CompactField, Diagnostic> {
    field(record, name).ok_or_else(|| inventory_error(record, "required structural field was lost"))
}

fn inventory_error(record: &CompactRecord, message: &str) -> Diagnostic {
    let mut error = Diagnostic::new(
        DiagnosticClass::Infrastructure,
        "change_block_inventory",
        message,
    );
    error.location = Some(record.location.clone());
    error
}

pub(super) fn capacity(
    location: &SourceLocation,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    let mut error = Diagnostic::new(DiagnosticClass::Resource, code, message);
    error.location = Some(location.clone());
    error
}
