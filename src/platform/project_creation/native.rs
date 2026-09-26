//! Embedded native recipes use the public declaration decoder, not a graph writer.
//! Inputs are trusted executable-owned source. Bind the first-line base and, when
//! explicitly selected, an exact package-linkage preamble. The remaining native
//! application declarations are copied verbatim, never searched or generated.

use super::{creation_error, first_diagnostic, publish_recipe_request};
use crate::platform::control::decode_compact_change_in_repository;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::RevisionId;

pub(super) struct Input {
    pub path: &'static str,
    pub source: &'static str,
    pub base: &'static str,
    pub local_imports: Option<LocalImports>,
}

#[derive(Clone, Copy)]
pub(super) struct LocalImports {
    pub packaged: &'static str,
    pub vendored: &'static str,
}

// These exact maintained sources are also used by the independent package examples.
// Here they become editable local modules, not separately selected dependencies.
pub(super) const UI_INPUT: Input = Input {
    path: "docs/guides/examples/ui.lkjc",
    source: include_str!("../../../docs/guides/examples/ui.lkjc"),
    base: "LIBRARY_BASE",
    local_imports: None,
};
pub(super) const UI_TESTS_INPUT: Input = Input {
    path: "docs/guides/examples/ui-tests.lkjc",
    source: include_str!("../../../docs/guides/examples/ui-tests.lkjc"),
    base: "LIBRARY_BASE",
    local_imports: None,
};
pub(super) const WEB_INPUTS: &[Input] = &[
    UI_INPUT,
    UI_TESTS_INPUT,
    Input {
        path: "docs/guides/examples/web-starter.lkjc",
        source: include_str!("../../../docs/guides/examples/web-starter.lkjc"),
        base: "STARTER_BASE",
        local_imports: None,
    },
];

pub(super) fn apply_inputs(
    repository: &GraphRepository,
    inputs: &[Input],
) -> Result<(), Diagnostic> {
    for input in inputs {
        let source = input.bind(repository.current()?.head.revision)?;
        let normalized =
            decode_compact_change_in_repository(input.path, source.as_bytes(), repository)
                .map_err(first_diagnostic)?;
        publish_recipe_request(repository, normalized)?;
    }
    Ok(())
}

impl Input {
    fn bind(&self, revision: RevisionId) -> Result<String, Diagnostic> {
        let header = format!("request base={}\n", self.base);
        let tail = self.source.strip_prefix(&header).ok_or_else(|| {
            creation_error(
                DiagnosticClass::Corrupt,
                "new_native_recipe_header",
                format!(
                    "embedded native recipe '{}' has no exact base header",
                    self.path
                ),
            )
        })?;
        match self.local_imports {
            None => Ok(format!("request base={revision}\n{tail}")),
            Some(imports) => {
                let declarations = tail.strip_prefix(imports.packaged).ok_or_else(|| {
                    creation_error(
                        DiagnosticClass::Corrupt,
                        "new_native_recipe_imports",
                        format!(
                            "embedded native recipe '{}' has changed linkage metadata",
                            self.path
                        ),
                    )
                })?;
                Ok(format!(
                    "request base={revision}\n{}{declarations}",
                    imports.vendored
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests;
