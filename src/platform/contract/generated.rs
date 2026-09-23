//! Public pages rendered by the ordinary native guide program.
use super::native_guides;
use super::registry::capabilities_snapshot;

#[derive(Clone, Debug)]
pub struct GeneratedDocument {
    pub relative_path: &'static str,
    pub bytes: Vec<u8>,
}

pub fn generated_documents() -> Result<Vec<GeneratedDocument>, String> {
    let snapshot = capabilities_snapshot()?;
    Ok(vec![
        GeneratedDocument {
            relative_path: "operations.md",
            bytes: native_guides::operations(&snapshot)?.into_bytes(),
        },
        GeneratedDocument {
            relative_path: "diagnostics.md",
            bytes: native_guides::diagnostics(&snapshot)?.into_bytes(),
        },
        GeneratedDocument {
            relative_path: "change-grammar.md",
            bytes: native_guides::change_grammar(&snapshot)?.into_bytes(),
        },
        GeneratedDocument {
            relative_path: "function-definition.md",
            bytes: native_guides::function_definition(&snapshot)?.into_bytes(),
        },
        GeneratedDocument {
            relative_path: "deployment.md",
            bytes: native_guides::deployment(&snapshot)?.into_bytes(),
        },
        GeneratedDocument {
            relative_path: "builtin-standard.md",
            bytes: native_guides::builtin_standard(&snapshot)?.into_bytes(),
        },
        GeneratedDocument {
            relative_path: "stateful-http-authoring.md",
            bytes: native_guides::stateful_http(&snapshot)?.into_bytes(),
        },
        GeneratedDocument {
            relative_path: "nostr-relay-info-authoring.md",
            bytes: native_guides::relay_information(&snapshot)?.into_bytes(),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::project_creation::ProjectTemplate;

    const GENERATOR_COMMAND: &str = "lkjscript capabilities --generate-docs docs/generated";

    #[test]
    fn generated_documents_are_unique_and_repeatable() {
        let first = generated_documents().expect("generated documents");
        let second = generated_documents().expect("generated documents");
        assert_eq!(first.len(), 8);
        assert_eq!(first.len(), second.len());
        let names = first
            .iter()
            .map(|document| document.relative_path)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names.len(), first.len());
        for (left, right) in first.iter().zip(second.iter()) {
            assert_eq!(left.relative_path, right.relative_path);
            assert_eq!(left.bytes, right.bytes);
            assert!(left.bytes.ends_with(b"\n"));
        }
        let operations = first
            .iter()
            .find(|document| document.relative_path == "operations.md")
            .expect("generated operations document");
        let operations = std::str::from_utf8(&operations.bytes).expect("UTF-8 operations document");
        for template in ProjectTemplate::ALL {
            assert!(operations.contains(&format!("<td><code>{}</code></td>", template.name())));
        }
    }

    #[test]
    fn checked_in_generated_documents_match_executable_truth() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/generated");
        for document in generated_documents().expect("generated documents") {
            let path = root.join(document.relative_path);
            assert_eq!(
                std::fs::read(&path).expect("checked-in generated document"),
                document.bytes,
                "{} is stale; run {GENERATOR_COMMAND}",
                path.display()
            );
        }
        for removed in ["contracts.md", "protocol.schema.json", "manifest.json"] {
            assert!(
                !root.join(removed).exists(),
                "obsolete generated output must remain deleted"
            );
        }
    }
}
