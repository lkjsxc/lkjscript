//! Embedded native recipes use the public declaration decoder, not a graph writer.
//! Inputs are trusted executable-owned source. Only their first-line base is bound;
//! text inside a declaration is never substituted or generated.

use super::{creation_error, first_diagnostic, publish_recipe_request};
use crate::platform::control::decode_compact_change_in_repository;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::RevisionId;

pub(super) struct Input {
    pub path: &'static str,
    pub source: &'static str,
    pub base: &'static str,
}

// These exact maintained sources are also used by the independent package examples.
// Here they become editable local modules, not separately selected dependencies.
pub(super) const WEB_INPUTS: &[Input] = &[
    Input {
        path: "docs/guides/examples/ui.lkjc",
        source: include_str!("../../../docs/guides/examples/ui.lkjc"),
        base: "LIBRARY_BASE",
    },
    Input {
        path: "docs/guides/examples/ui-tests.lkjc",
        source: include_str!("../../../docs/guides/examples/ui-tests.lkjc"),
        base: "LIBRARY_BASE",
    },
    Input {
        path: "docs/guides/examples/web-starter.lkjc",
        source: include_str!("../../../docs/guides/examples/web-starter.lkjc"),
        base: "STARTER_BASE",
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
        Ok(format!("request base={revision}\n{tail}"))
    }
}

#[cfg(test)]
mod tests;
