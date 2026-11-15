//! Error recovery at the isolated, source-identity type-constraint boundary.
use super::{a, definition_types};
use std::collections::BTreeMap;

pub(super) struct Report {
    pub definitions: BTreeMap<usize, definition_types::DefinitionTypes>,
    pub problems: Vec<a::Diagnostic>,
}

pub(super) fn infer_selected(
    source: &a::Program,
    names: &crate::resolve::Resolver<'_>,
    implementations: &crate::generics::ImplementationIndex,
    selected: &[usize],
) -> Report {
    let mut definitions = BTreeMap::new();
    let mut problems = Vec::new();
    for index in selected {
        let index = *index;
        if source.functions[index].body.is_none() {
            continue;
        }
        match definition_types::infer(source, names, implementations, index) {
            Ok(definition) => {
                definitions.insert(index, definition);
            }
            Err(problem) => problems.push(problem),
        }
    }
    Report {
        definitions,
        problems,
    }
}
