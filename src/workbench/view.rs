use super::{ContextPacket, MAX_REVIEW_OUTPUT_BYTES};
use crate::diff::{ChangeKind, ScalarValue};
use crate::error::{ErrorCode, LkError, Result};
use crate::ids::NodeId;
use crate::query::NodeView;
use crate::schema::{Node, OperationKind, SemanticType, ValueRef};
use std::collections::{BTreeMap, BTreeSet};

pub fn render_context_packet(packet: &ContextPacket, include_full_ids: bool) -> Result<Vec<u8>> {
    let aliases = alias_map(packet);
    let nodes: BTreeMap<_, _> = packet
        .payload
        .nodes
        .iter()
        .map(|view| (view.summary.node, view))
        .collect();
    let mut output = Vec::new();
    push_line(&mut output, "lkjscript semantic review v2")?;
    push_line(
        &mut output,
        &format!(
            "workspace {} revision {} snapshot {}",
            packet.payload.workspace, packet.payload.revision, packet.payload.summary.hash
        ),
    )?;
    push_line(
        &mut output,
        &format!(
            "schema {} packet {} purpose {}",
            packet.payload.schema_digest,
            packet.digest,
            packet.payload.purpose.machine_name()
        ),
    )?;
    push_line(
        &mut output,
        &format!(
            "scope nodes={} workspace_nodes={} durable={} local={} anchors={} tombstones={} complete={} blockers={}",
            nodes.len(),
            packet.payload.summary.node_count,
            packet.payload.summary.durable_identity_count,
            packet.payload.summary.function_local_reference_count,
            packet.payload.summary.anchor_count,
            packet.payload.summary.tombstone_count,
            packet.payload.summary.complete,
            packet.payload.summary.blocker_count
        ),
    )?;
    push_line(&mut output, "")?;

    let roots = if packet.payload.targets.is_empty() {
        vec![packet.payload.summary.root]
    } else {
        packet.payload.targets.clone()
    };
    let mut visited = BTreeSet::new();
    for root in roots {
        render_tree(
            root,
            0,
            &nodes,
            &aliases,
            include_full_ids,
            &mut visited,
            &mut output,
        )?;
    }
    for node in nodes.keys().copied() {
        if !visited.contains(&node) {
            push_line(&mut output, "")?;
            render_tree(
                node,
                0,
                &nodes,
                &aliases,
                include_full_ids,
                &mut visited,
                &mut output,
            )?;
        }
    }

    if !packet.payload.blockers.items.is_empty() {
        push_line(&mut output, "")?;
        push_line(&mut output, "blockers")?;
        for blocker in &packet.payload.blockers.items {
            let target = blocker
                .target
                .map(|node| anchor(node, &aliases, include_full_ids))
                .unwrap_or_else(|| "<missing-definition>".to_owned());
            let expected = blocker
                .expected_type
                .map(|ty| render_type(ty, &aliases, include_full_ids))
                .unwrap_or_else(|| "<none>".to_owned());
            push_line(
                &mut output,
                &format!(
                    "  {:?} owner={} target={} expected={}",
                    blocker.category,
                    anchor(blocker.owner, &aliases, include_full_ids),
                    target,
                    expected
                ),
            )?;
        }
    }

    push_line(&mut output, "")?;
    push_line(
        &mut output,
        &format!(
            "omissions node_scope_truncated={} discovered_frontier_omitted_nodes={} blockers_truncated={} diff_truncated={}",
            packet.payload.omissions.node_scope_truncated,
            packet.payload.omissions.discovered_frontier_omitted_nodes,
            packet.payload.omissions.blockers_truncated,
            packet.payload.omissions.semantic_diff_truncated
        ),
    )?;
    Ok(output)
}

pub fn render_semantic_diff(packet: &ContextPacket, include_full_ids: bool) -> Result<Vec<u8>> {
    let aliases = alias_map(packet);
    let diff = packet.payload.semantic_diff.as_ref().ok_or_else(|| {
        LkError::new(
            ErrorCode::InvalidQuery,
            "context packet does not contain a semantic diff",
        )
    })?;
    let mut output = Vec::new();
    push_line(&mut output, "lkjscript semantic diff review v2")?;
    push_line(
        &mut output,
        &format!(
            "workspace {} revisions {}..{} change_count={} change_digest={} packet={}",
            packet.payload.workspace,
            diff.from,
            diff.to,
            diff.change_count,
            diff.change_digest,
            packet.digest
        ),
    )?;
    for change in &diff.page.items {
        let node = anchor(change.node, &aliases, include_full_ids);
        let description = match &change.kind {
            ChangeKind::Created { kind } => format!("created {}", kind.machine_name()),
            ChangeKind::Deleted { kind } => format!("deleted {}", kind.machine_name()),
            ChangeKind::Renamed { before, after } => {
                format!("renamed {} -> {}", quoted(before), quoted(after))
            }
            ChangeKind::ScalarAttributeChanged { before, after } => format!(
                "scalar {} -> {}",
                render_scalar(before, &aliases, include_full_ids),
                render_scalar(after, &aliases, include_full_ids)
            ),
            ChangeKind::ContainmentChanged {
                before_count,
                after_count,
            } => format!("containment {before_count} -> {after_count}"),
            ChangeKind::OperandChanged {
                index,
                before,
                after,
            } => format!(
                "operand[{index}] {} -> {}",
                before
                    .map(|value| render_value(value, &aliases, include_full_ids))
                    .unwrap_or_else(|| "<none>".to_owned()),
                after
                    .map(|value| render_value(value, &aliases, include_full_ids))
                    .unwrap_or_else(|| "<none>".to_owned())
            ),
            ChangeKind::DefinitionChanged { before, after } => format!(
                "definition {} -> {}",
                anchor(*before, &aliases, include_full_ids),
                anchor(*after, &aliases, include_full_ids)
            ),
            ChangeKind::EntryFunctionChanged { before, after } => format!(
                "entry {} -> {}",
                optional_anchor(*before, &aliases, include_full_ids),
                optional_anchor(*after, &aliases, include_full_ids)
            ),
            ChangeKind::CompletenessChanged { complete } => {
                format!("completeness -> {complete}")
            }
            ChangeKind::OperationRefined {
                before,
                after,
                result_type,
                replacement,
            } => format!(
                "refined {} -> {} : {} ({})",
                before.machine_name(),
                after.machine_name(),
                render_type(*result_type, &aliases, include_full_ids),
                render_operation(replacement, &aliases, include_full_ids)
            ),
            ChangeKind::AllocatedAndTombstoned => "allocated_and_tombstoned".to_owned(),
            ChangeKind::FunctionBodyChanged {
                before_items,
                after_items,
                added_items,
                removed_items,
                modified_items,
            } => format!(
                "function_body_changed items {before_items}->{after_items} added={added_items} removed={removed_items} modified={modified_items}"
            ),
            ChangeKind::BuildTargetChanged {
                before_kind,
                after_kind,
                before_digest,
                after_digest,
            } => format!(
                "build_target_changed {}:{} -> {}:{}",
                before_kind.machine_name(),
                before_digest,
                after_kind.machine_name(),
                after_digest
            ),
        };
        push_line(&mut output, &format!("{node} {description}"))?;
    }
    if diff.page.next.is_some() {
        push_line(&mut output, "... semantic diff continuation omitted")?;
    }
    Ok(output)
}

fn render_tree(
    root: NodeId,
    initial_indent: usize,
    nodes: &BTreeMap<NodeId, &NodeView>,
    aliases: &BTreeMap<NodeId, &str>,
    include_full_ids: bool,
    visited: &mut BTreeSet<NodeId>,
    output: &mut Vec<u8>,
) -> Result<()> {
    let mut stack = vec![(root, initial_indent)];
    while let Some((id, indent)) = stack.pop() {
        if !visited.insert(id) {
            continue;
        }
        let prefix = "  ".repeat(indent);
        let Some(view) = nodes.get(&id) else {
            push_line(
                output,
                &format!(
                    "{prefix}... {} omitted from packet scope",
                    anchor(id, aliases, include_full_ids)
                ),
            )?;
            continue;
        };
        let record = view.record.as_ref().ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "review packet contains an unexpanded node",
            )
        })?;
        push_line(
            output,
            &format!(
                "{prefix}{} [{}] {}",
                anchor(id, aliases, include_full_ids),
                view.summary.identity_class.machine_name(),
                render_node(record, aliases, include_full_ids)
            ),
        )?;
        for index in (0..record.owned_child_count()).rev() {
            if let Some(child) = record.owned_child(index) {
                stack.push((child, indent.saturating_add(1)));
            }
        }
    }
    Ok(())
}

fn render_node(node: &Node, aliases: &BTreeMap<NodeId, &str>, include_full_ids: bool) -> String {
    match node {
        Node::WorkspaceRoot { packages, targets } => format!(
            "workspace packages={} targets={}",
            packages.len(),
            targets.len()
        ),
        Node::BuildTarget { definition, .. } => {
            format!("build_target kind={}", definition.kind().machine_name())
        }
        Node::Package {
            name,
            modules,
            entry,
            ..
        } => format!(
            "package {} modules={} entry={}",
            quoted(name),
            modules.len(),
            optional_anchor(*entry, aliases, include_full_ids)
        ),
        Node::Module {
            name,
            types,
            functions,
            ..
        } => format!(
            "module {} types={} functions={}",
            quoted(name),
            types.len(),
            functions.len()
        ),
        Node::ProductType { name, fields, .. } => {
            format!("record {} fields={}", quoted(name), fields.len())
        }
        Node::ProductField {
            ordinal, name, ty, ..
        } => format!(
            "field #{} {}: {}",
            ordinal,
            quoted(name),
            render_type(*ty, aliases, include_full_ids)
        ),
        Node::SumType { name, variants, .. } => {
            format!("variant {} alternatives={}", quoted(name), variants.len())
        }
        Node::SumVariant {
            ordinal,
            name,
            payload,
            ..
        } => format!(
            "alternative #{} {}{}",
            ordinal,
            quoted(name),
            payload
                .map(|ty| format!("({})", render_type(ty, aliases, include_full_ids)))
                .unwrap_or_default()
        ),
        Node::SequenceType { name, element, .. } => format!(
            "sequence {} element={}",
            quoted(name),
            render_type(*element, aliases, include_full_ids)
        ),
        Node::Function {
            name,
            parameters,
            result,
            body,
            ..
        } => format!(
            "fn {}({}) -> {} body={}",
            quoted(name),
            parameters
                .iter()
                .map(|node| anchor(*node, aliases, include_full_ids))
                .collect::<Vec<_>>()
                .join(", "),
            render_type(*result, aliases, include_full_ids),
            optional_anchor(*body, aliases, include_full_ids)
        ),
        Node::Parameter {
            ordinal, name, ty, ..
        } => format!(
            "parameter #{} {}: {}",
            ordinal,
            quoted(name),
            render_type(*ty, aliases, include_full_ids)
        ),
        Node::Region { blocks, .. } => format!("region blocks={}", blocks.len()),
        Node::Block {
            arguments,
            operations,
            terminator,
            ..
        } => format!(
            "block arguments={} operations={} terminator={}",
            arguments.len(),
            operations.len(),
            optional_anchor(*terminator, aliases, include_full_ids)
        ),
        Node::BlockArgument { ordinal, ty, .. } => format!(
            "block_argument #{}: {}",
            ordinal,
            render_type(*ty, aliases, include_full_ids)
        ),
        Node::Operation { operation, .. } => {
            format!(
                "operation {}",
                render_operation(operation, aliases, include_full_ids)
            )
        }
    }
}

fn render_operation(
    operation: &OperationKind,
    aliases: &BTreeMap<NodeId, &str>,
    include_full_ids: bool,
) -> String {
    let value = |value| render_value(value, aliases, include_full_ids);
    let node = |node| anchor(node, aliases, include_full_ids);
    let ty = |ty| render_type(ty, aliases, include_full_ids);
    match operation {
        OperationKind::ConstUnit => "const_unit()".to_owned(),
        OperationKind::ConstI64(value) => format!("const_i64({value})"),
        OperationKind::ConstBool(value) => format!("const_bool({value})"),
        OperationKind::ConstBytes(bytes) => format!(
            "const_bytes({})",
            serde_json::to_string(bytes).unwrap_or_else(|_| "\"<unavailable>\"".to_owned())
        ),
        OperationKind::ConstText(text) => format!(
            "const_text({})",
            serde_json::to_string(text).unwrap_or_else(|_| "\"<unavailable>\"".to_owned())
        ),
        OperationKind::AddI64 { lhs, rhs } => format!("add_i64({}, {})", value(*lhs), value(*rhs)),
        OperationKind::LtI64 { lhs, rhs } => format!("lt_i64({}, {})", value(*lhs), value(*rhs)),
        OperationKind::EqualI64 { lhs, rhs } => {
            format!("equal_i64({}, {})", value(*lhs), value(*rhs))
        }
        OperationKind::NotBool { value: operand } => format!("not_bool({})", value(*operand)),
        OperationKind::AndBool { lhs, rhs } => {
            format!("and_bool({}, {})", value(*lhs), value(*rhs))
        }
        OperationKind::OrBool { lhs, rhs } => {
            format!("or_bool({}, {})", value(*lhs), value(*rhs))
        }
        OperationKind::BytesLen { value: operand } => format!("bytes_len({})", value(*operand)),
        OperationKind::BytesAt {
            value: operand,
            index,
        } => format!("bytes_at({}, {})", value(*operand), value(*index)),
        OperationKind::BytesSlice {
            value: operand,
            start,
            length,
        } => format!(
            "bytes_slice({}, {}, {})",
            value(*operand),
            value(*start),
            value(*length)
        ),
        OperationKind::BytesEqual { lhs, rhs } => {
            format!("bytes_equal({}, {})", value(*lhs), value(*rhs))
        }
        OperationKind::BytesConcat { lhs, rhs } => {
            format!("bytes_concat({}, {})", value(*lhs), value(*rhs))
        }
        OperationKind::TextLen { value: operand } => format!("text_len({})", value(*operand)),
        OperationKind::TextEqual { lhs, rhs } => {
            format!("text_equal({}, {})", value(*lhs), value(*rhs))
        }
        OperationKind::TextConcat { lhs, rhs } => {
            format!("text_concat({}, {})", value(*lhs), value(*rhs))
        }
        OperationKind::TextScalarLen { value: operand } => {
            format!("text_scalar_len({})", value(*operand))
        }
        OperationKind::TextGraphemeLen { value: operand } => {
            format!("text_grapheme_len({})", value(*operand))
        }
        OperationKind::TextLineCount { value: operand } => {
            format!("text_line_count({})", value(*operand))
        }
        OperationKind::TextScalarAt {
            value: operand,
            index,
        } => format!("text_scalar_at({}, {})", value(*operand), value(*index)),
        OperationKind::TextPreviousGraphemeBoundary {
            value: operand,
            index,
        } => format!(
            "text_previous_grapheme_boundary({}, {})",
            value(*operand),
            value(*index)
        ),
        OperationKind::TextNextGraphemeBoundary {
            value: operand,
            index,
        } => format!(
            "text_next_grapheme_boundary({}, {})",
            value(*operand),
            value(*index)
        ),
        OperationKind::TextLineStart {
            value: operand,
            line,
        } => format!("text_line_start({}, {})", value(*operand), value(*line)),
        OperationKind::TextLineEnd {
            value: operand,
            line,
        } => format!("text_line_end({}, {})", value(*operand), value(*line)),
        OperationKind::TextByteToLine {
            value: operand,
            index,
        } => format!("text_byte_to_line({}, {})", value(*operand), value(*index)),
        OperationKind::TextSlice {
            value: operand,
            start,
            end_exclusive,
        } => format!(
            "text_slice({}, {}, {})",
            value(*operand),
            value(*start),
            value(*end_exclusive)
        ),
        OperationKind::TextSplice {
            value: operand,
            start,
            end_exclusive,
            replacement,
        } => format!(
            "text_splice({}, {}, {}, {})",
            value(*operand),
            value(*start),
            value(*end_exclusive),
            value(*replacement)
        ),
        OperationKind::TextFindForward {
            value: operand,
            query,
            start,
        } => format!(
            "text_find_forward({}, {}, {})",
            value(*operand),
            value(*query),
            value(*start)
        ),
        OperationKind::TextFindBackward {
            value: operand,
            query,
            end_exclusive,
        } => format!(
            "text_find_backward({}, {}, {})",
            value(*operand),
            value(*query),
            value(*end_exclusive)
        ),
        OperationKind::TextLineEndingKind { value: operand } => {
            format!("text_line_ending_kind({})", value(*operand))
        }
        OperationKind::TextDisplayWidth {
            value: operand,
            start,
            end_exclusive,
            initial_column,
            tab_width,
        } => format!(
            "text_display_width({}, {}, {}, {}, {})",
            value(*operand),
            value(*start),
            value(*end_exclusive),
            value(*initial_column),
            value(*tab_width)
        ),
        OperationKind::TextCellPrefixBoundary {
            value: operand,
            start,
            end_exclusive,
            initial_column,
            maximum_cells,
            tab_width,
        } => format!(
            "text_cell_prefix_boundary({}, {}, {}, {}, {}, {})",
            value(*operand),
            value(*start),
            value(*end_exclusive),
            value(*initial_column),
            value(*maximum_cells),
            value(*tab_width)
        ),
        OperationKind::TextFromScalar { value: operand } => {
            format!("text_from_scalar({})", value(*operand))
        }
        OperationKind::TextToScalars {
            sequence,
            value: operand,
        } => format!("text_to_scalars({}, {})", node(*sequence), value(*operand)),
        OperationKind::TextFromScalars {
            sequence,
            value: operand,
        } => format!(
            "text_from_scalars({}, {})",
            node(*sequence),
            value(*operand)
        ),
        OperationKind::SequenceEmpty { sequence } => format!("sequence_empty({})", node(*sequence)),
        OperationKind::SequenceLen {
            sequence,
            value: operand,
        } => format!("sequence_len({}, {})", node(*sequence), value(*operand)),
        OperationKind::SequenceGet {
            sequence,
            value: operand,
            index,
        } => format!(
            "sequence_get({}, {}, {})",
            node(*sequence),
            value(*operand),
            value(*index)
        ),
        OperationKind::SequenceAppend {
            sequence,
            value: operand,
            element,
        } => format!(
            "sequence_append({}, {}, {})",
            node(*sequence),
            value(*operand),
            value(*element)
        ),
        OperationKind::SequenceReplace {
            sequence,
            value: operand,
            index,
            element,
        } => format!(
            "sequence_replace({}, {}, {}, {})",
            node(*sequence),
            value(*operand),
            value(*index),
            value(*element)
        ),
        OperationKind::SequenceSlice {
            sequence,
            value: operand,
            start,
            end_exclusive,
        } => format!(
            "sequence_slice({}, {}, {}, {})",
            node(*sequence),
            value(*operand),
            value(*start),
            value(*end_exclusive)
        ),
        OperationKind::SequenceConcat { sequence, lhs, rhs } => format!(
            "sequence_concat({}, {}, {})",
            node(*sequence),
            value(*lhs),
            value(*rhs)
        ),
        OperationKind::SequenceRepeat {
            sequence,
            element,
            count,
        } => format!(
            "sequence_repeat({}, {}, {})",
            node(*sequence),
            value(*element),
            value(*count)
        ),
        OperationKind::Call {
            function,
            arguments,
        } => format!(
            "call {}({})",
            node(*function),
            arguments
                .iter()
                .map(|argument| value(*argument))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        OperationKind::Hole { expected } => format!("placeholder({})", ty(*expected)),
        OperationKind::If {
            condition,
            result,
            then_region,
            else_region,
        } => format!(
            "if {} -> {} then={} else={}",
            value(*condition),
            ty(*result),
            node(*then_region),
            node(*else_region)
        ),
        OperationKind::ForI64 {
            start,
            end_exclusive,
            step,
            initial,
            carried,
            body_region,
        } => format!(
            "for_i64 {}..{} step={} initial={} carried={} body={}",
            value(*start),
            value(*end_exclusive),
            step,
            value(*initial),
            ty(*carried),
            node(*body_region)
        ),
        OperationKind::Return { value: result } => format!("return {}", value(*result)),
        OperationKind::Yield { value: result } => format!("yield {}", value(*result)),
        OperationKind::ConstructProduct { product, fields } => format!(
            "construct_product {} {{{}}}",
            node(*product),
            fields
                .iter()
                .map(|field| format!("{}={}", node(field.field), value(field.value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        OperationKind::ProjectField {
            value: product,
            field,
        } => format!("project_field({}, {})", value(*product), node(*field)),
        OperationKind::ConstructVariant { variant, payload } => format!(
            "construct_variant {}{}",
            node(*variant),
            payload
                .map(|payload| format!("({})", value(payload)))
                .unwrap_or_default()
        ),
        OperationKind::MatchSum {
            scrutinee,
            result,
            arms,
        } => format!(
            "match_sum {} -> {} [{}]",
            value(*scrutinee),
            ty(*result),
            arms.iter()
                .map(|arm| format!("{}:{}", node(arm.variant), node(arm.region)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn render_value(
    value: ValueRef,
    aliases: &BTreeMap<NodeId, &str>,
    include_full_ids: bool,
) -> String {
    match value {
        ValueRef::FunctionParameter(node) => anchor(node, aliases, include_full_ids),
        ValueRef::BlockArgument(node) => anchor(node, aliases, include_full_ids),
        ValueRef::OperationResult { operation, output } => {
            let operation = anchor(operation, aliases, include_full_ids);
            if output == 0 {
                operation
            } else {
                format!("{operation}.{output}")
            }
        }
    }
}

fn render_type(
    ty: SemanticType,
    aliases: &BTreeMap<NodeId, &str>,
    include_full_ids: bool,
) -> String {
    match ty {
        SemanticType::Unit => "unit".to_owned(),
        SemanticType::Bool => "bool".to_owned(),
        SemanticType::I64 => "i64".to_owned(),
        SemanticType::Bytes => "bytes".to_owned(),
        SemanticType::Text => "text".to_owned(),
        SemanticType::Nominal(node) => anchor(node, aliases, include_full_ids),
    }
}

fn render_scalar(
    value: &ScalarValue,
    aliases: &BTreeMap<NodeId, &str>,
    include_full_ids: bool,
) -> String {
    match value {
        ScalarValue::I64(value) => value.to_string(),
        ScalarValue::Bool(value) => value.to_string(),
        ScalarValue::Type(value) => render_type(*value, aliases, include_full_ids),
        ScalarValue::Bytes(value) => {
            serde_json::to_string(value).unwrap_or_else(|_| "\"<unavailable>\"".to_owned())
        }
        ScalarValue::Text(value) => {
            serde_json::to_string(value).unwrap_or_else(|_| "\"<unavailable>\"".to_owned())
        }
    }
}

fn alias_map(packet: &ContextPacket) -> BTreeMap<NodeId, &str> {
    packet
        .payload
        .aliases
        .iter()
        .map(|alias| (alias.node, alias.alias.as_str()))
        .collect()
}

fn anchor(node: NodeId, aliases: &BTreeMap<NodeId, &str>, include_full_ids: bool) -> String {
    match aliases.get(&node) {
        Some(alias) if include_full_ids => format!("@{alias}={node}"),
        Some(alias) => format!("@{alias}"),
        None => node.to_string(),
    }
}

fn optional_anchor(
    node: Option<NodeId>,
    aliases: &BTreeMap<NodeId, &str>,
    include_full_ids: bool,
) -> String {
    node.map(|node| anchor(node, aliases, include_full_ids))
        .unwrap_or_else(|| "<none>".to_owned())
}

fn quoted(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"<unavailable>\"".to_owned())
}

fn push_line(output: &mut Vec<u8>, line: &str) -> Result<()> {
    let needed = line.len().checked_add(1).ok_or_else(|| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic review output accounting overflowed",
        )
    })?;
    if output.len().saturating_add(needed) > MAX_REVIEW_OUTPUT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic review exceeds the output byte policy",
        ));
    }
    output.extend_from_slice(line.as_bytes());
    output.push(b'\n');
    Ok(())
}
