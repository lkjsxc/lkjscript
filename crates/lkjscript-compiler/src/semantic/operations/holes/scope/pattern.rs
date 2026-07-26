use std::collections::BTreeMap;

use super::super::site::HoleSite;
use crate::semantic::schema::{ScopeEntity, ScopeEntityKind};
use crate::source::SourceNode;

pub(super) fn add(
    pattern: &SourceNode,
    site: &HoleSite<'_>,
    program: Option<&crate::hir::Program>,
    revision: &str,
    prefix: &[usize],
    output: &mut BTreeMap<String, ScopeEntity>,
) {
    let Some(program) = program else { return };
    let mut names = Vec::new();
    collect_names(pattern, &mut names);
    for (index, name) in names.into_iter().enumerate() {
        let mut types = program
            .match_plans
            .iter()
            .flat_map(|plan| &plan.bindings)
            .filter_map(|assignment| {
                let binding = program.binding(assignment.local.binding)?;
                (binding.name == name).then(|| binding.ty.clone())
            });
        let Some(ty) = types.next() else { continue };
        if types.any(|candidate| candidate != ty) {
            continue;
        }
        super::insert(
            output,
            Some(program),
            revision,
            super::local_identity(&site.declaration_key, "pattern", prefix, index, name),
            ScopeEntityKind::ImmutableLocal,
            name.to_string(),
            ty,
        );
    }
}

fn collect_names<'a>(node: &'a SourceNode, output: &mut Vec<&'a str>) {
    if super::super::types::call_is(node, "binding") {
        if let Some(name) = node
            .children
            .first()
            .and_then(super::super::types::source_name)
        {
            output.push(name);
        }
        return;
    }
    for child in &node.children {
        collect_names(child, output);
    }
}
