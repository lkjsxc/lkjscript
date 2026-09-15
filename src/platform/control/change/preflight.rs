//! Iterative admission of the complete mixed expression forest and expanded type definitions.
//! This runs before recursive authored collection, encoding, type cloning or semantic lowering.

use super::*;
use crate::platform::kernel::contract::{MAXIMUM_EXPRESSION_DEPTH, MAXIMUM_TYPE_DEPTH};

pub(crate) const MAXIMUM_EXPANDED_INPUT_TYPE_NODES: usize =
    crate::platform::change::MAXIMUM_CHANGE_AUTHORED_TYPE_NODES as usize;

impl Decoder {
    pub(super) fn preflight(&mut self) -> Result<Vec<String>, Diagnostic> {
        for parent in self.blocks.keys() {
            if let Some(edge) = self
                .arguments
                .get(parent)
                .or_else(|| self.type_parameters.get(parent))
                .and_then(|edges| edges.first())
            {
                return Err(Diagnostic::source(
                    "change_block_private",
                    "outer child edges cannot extend an expression block",
                    edge.location.clone(),
                ));
            }
            for parents in self.record_edges.values() {
                if let Some(edge) = parents.get(parent).and_then(|edges| edges.first()) {
                    return Err(record_error(
                        &edge.record,
                        "change_block_private",
                        "outer child edges cannot extend an expression block",
                    ));
                }
            }
        }
        let mut expression_children = BTreeMap::new();
        for (symbol, record) in &self.expressions {
            let mut children = Vec::new();
            let direct: &[&str] = match record.operation.as_str() {
                "expression.if" => &["condition", "when-true", "when-false"],
                "expression.let" | "expression.transaction" => &["body"],
                "expression.bind" => &["callee"],
                "expression.invoke" => &["function"],
                "expression.field" | "expression.match" => &["value"],
                "expression.variant" => &["payload"],
                _ => &[],
            };
            for name in direct {
                if let Some(field) = field(record, name) {
                    children.push((field.value.clone(), field.location.clone()));
                }
            }
            if matches!(
                record.operation.as_str(),
                "expression.sequence"
                    | "expression.call"
                    | "expression.bind"
                    | "expression.invoke"
                    | "expression.list"
                    | "expression.capability-call"
            ) && let Some(edges) = self.arguments.get(symbol)
            {
                children.extend(
                    edges
                        .iter()
                        .map(|edge| (edge.value.clone(), edge.location.clone())),
                );
            }
            let (edge, fields): (&str, &[&str]) = match record.operation.as_str() {
                "expression.let" => ("expression.binding", &["value"]),
                "expression.record" => ("expression.record-field", &["value"]),
                "expression.map" => ("expression.map-entry", &["key", "value"]),
                "expression.match" => ("expression.match-arm", &["body"]),
                _ => ("", &[]),
            };
            if let Some(edges) = self
                .record_edges
                .get(edge)
                .and_then(|parents| parents.get(symbol))
            {
                for edge in edges {
                    for name in fields {
                        if let Some(field) = field(&edge.record, name) {
                            children.push((field.value.clone(), field.location.clone()));
                        }
                    }
                }
            }
            expression_children.insert(symbol.clone(), children);
        }
        let (order, _) = admit(
            &self.expressions,
            &expression_children,
            MAXIMUM_EXPRESSION_DEPTH,
            false,
            |name| self.blocks.get(name).map_or(1, |body| body.depth),
            |_| 1,
        )?;

        let mut type_children = BTreeMap::new();
        for (symbol, record) in &self.types {
            let mut children = Vec::new();
            let direct: &[&str] = match record.operation.as_str() {
                "type.list" | "type.option" | "type.stream" => &["item"],
                "type.map" => &["key", "value"],
                "type.result" => &["ok", "error"],
                "type.function" | "type.task-function" => &["result"],
                _ => &[],
            };
            for name in direct {
                if let Some(field) = field(record, name) {
                    children.push((field.value.clone(), field.location.clone()));
                }
            }
            if matches!(
                record.operation.as_str(),
                "type.application" | "type.function" | "type.task-function"
            ) && let Some(edges) = self.type_parameters.get(symbol)
            {
                children.extend(
                    edges
                        .iter()
                        .map(|edge| (edge.value.clone(), edge.location.clone())),
                );
            }
            if record.operation == "type.structural-record"
                && let Some(edges) = self
                    .record_edges
                    .get("type.field")
                    .and_then(|parents| parents.get(symbol))
            {
                for edge in edges {
                    if let Some(field) = field(&edge.record, "type") {
                        children.push((field.value.clone(), field.location.clone()));
                    }
                }
            }
            type_children.insert(symbol.clone(), children);
        }
        let (_, sizes) = admit(
            &self.types,
            &type_children,
            MAXIMUM_TYPE_DEPTH,
            true,
            |_| 1,
            |name| {
                let record = &self.types[name];
                1 + if record.operation == "type.task-function" {
                    optional(record, "effect").map_or(0, |row| self.effect_row_size(row))
                } else {
                    0
                }
            },
        )?;
        self.type_sizes = sizes;
        Ok(order)
    }
}

type Children = BTreeMap<String, Vec<(String, SourceLocation)>>;

fn admit(
    records: &BTreeMap<String, CompactRecord>,
    children: &Children,
    maximum_depth: usize,
    types: bool,
    own_depth: impl Fn(&str) -> usize,
    own_size: impl Fn(&str) -> usize,
) -> Result<(Vec<String>, BTreeMap<String, usize>), Diagnostic> {
    let mut states = BTreeMap::new();
    let mut depths: BTreeMap<String, usize> = BTreeMap::new();
    let mut sizes: BTreeMap<String, usize> = BTreeMap::new();
    let mut order = Vec::new();
    let mut total_size = 0_usize;
    for root in records.keys() {
        let mut stack = vec![(root.as_str(), false)];
        while let Some((name, exiting)) = stack.pop() {
            let record = &records[name];
            if exiting {
                let mut depth = own_depth(name);
                let mut size = own_size(name);
                for (child, location) in &children[name] {
                    let child_depth = depths.get(child).copied().unwrap_or(1);
                    depth = depth.max(child_depth + 1);
                    size = size
                        .checked_add(sizes.get(child).copied().unwrap_or(1))
                        .ok_or_else(|| {
                            structural::capacity(
                                location,
                                "change_input_type_capacity",
                                "expanded input accounting overflowed",
                            )
                        })?;
                    if depth > maximum_depth {
                        return Err(structural::capacity(
                            location,
                            if types {
                                "change_authored_type_depth"
                            } else {
                                "change_authored_expression_depth"
                            },
                            format!(
                                "complete normalized {} exceeds maximum depth {maximum_depth}",
                                if types { "type" } else { "expression" }
                            ),
                        ));
                    }
                    if types && size > MAXIMUM_EXPANDED_INPUT_TYPE_NODES {
                        return Err(structural::capacity(
                            location,
                            "change_input_type_capacity",
                            "expanded type definition exceeds the input adapter capacity",
                        ));
                    }
                }
                total_size = total_size.checked_add(size).ok_or_else(|| {
                    structural::capacity(
                        &record.location,
                        "change_input_type_capacity",
                        "expanded input accounting overflowed",
                    )
                })?;
                if types && total_size > MAXIMUM_EXPANDED_INPUT_TYPE_NODES {
                    return Err(structural::capacity(
                        &record.location,
                        "change_input_type_capacity",
                        "complete type cache exceeds the expanded input adapter capacity",
                    ));
                }
                depths.insert(name.to_owned(), depth);
                sizes.insert(name.to_owned(), size);
                states.insert(name, 2);
                order.push(name.to_owned());
                continue;
            }
            match states.get(name) {
                Some(2) => continue,
                Some(1) => {
                    return Err(Diagnostic::semantic(
                        if types {
                            "change_type_cycle"
                        } else {
                            "change_expression_cycle"
                        },
                        format!(
                            "{} definition cycle reaches '{name}'",
                            if types { "type" } else { "expression" }
                        ),
                        record.location.clone(),
                    ));
                }
                _ => {}
            }
            states.insert(name, 1);
            stack.push((name, true));
            for (child, location) in children[name].iter().rev() {
                if types && !child.starts_with('@') {
                    continue;
                }
                if !records.contains_key(child) {
                    return Err(Diagnostic::source(
                        if types {
                            "change_type_undefined"
                        } else {
                            "change_expression_undefined"
                        },
                        format!(
                            "{} '{child}' has no public definition",
                            if types { "type" } else { "expression" }
                        ),
                        location.clone(),
                    ));
                }
                stack.push((child.as_str(), false));
            }
        }
    }
    Ok((order, sizes))
}
