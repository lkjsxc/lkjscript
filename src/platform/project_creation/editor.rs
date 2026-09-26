//! Durable starter linkage and operator template, not application behavior.
//! The shared editor body remains at its ordinary native example source owner.

use super::native::{Input, LocalImports, UI_INPUT, UI_TESTS_INPUT};
use super::{STARTER_HTTP_ARTIFACT_PATH, creation_error};
use crate::platform::deployment::{AdapterDescriptor, decode_deployment, encode_deployment};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};

pub(super) const TARGET: &str = "editor";
pub(super) const LISTENER: &str = "127.0.0.1:8080";
pub(super) const DATA_ROOT: &str = "notes.lkjdata";
pub(super) const SECRET_VARIABLE: &str = "LKJSCRIPT_EDITOR_AUTHORIZATION";

// Replace only this exact, reviewed linkage preamble. The declarations following
// it are identical for independently packaged and locally vendored consumers.
// Any drift fails closed rather than accidentally rewriting application text.
const PACKAGED: &str = concat!(
    "add.dependency package=UI_PACKAGE semantic-revision=UI_REVISION package-revision=UI_PACKAGE_REVISION\n",
    "add.dependency package=FORMS_PACKAGE semantic-revision=FORMS_REVISION package-revision=FORMS_PACKAGE_REVISION\n",
    "\ndeclarations.begin\n(units\n  (use std builtin)\n",
    "  (use ui UI_PACKAGE UI_PACKAGE_REVISION)\n",
    "  (use forms FORMS_PACKAGE FORMS_PACKAGE_REVISION)\n",
);
const VENDORED: &str = "declarations.begin\n(units\n  (use std builtin)\n";

pub(super) const INPUTS: &[Input] = &[
    UI_INPUT,
    UI_TESTS_INPUT,
    Input {
        path: "docs/guides/examples/form-codec.lkjc",
        source: include_str!("../../../docs/guides/examples/form-codec.lkjc"),
        base: "LIBRARY_BASE",
        local_imports: None,
    },
    Input {
        path: "docs/guides/examples/editor.lkjc",
        source: include_str!("../../../docs/guides/examples/editor.lkjc"),
        base: "EDITOR_BASE",
        local_imports: Some(LocalImports {
            packaged: PACKAGED,
            vendored: VENDORED,
        }),
    },
    Input {
        path: "docs/guides/examples/editor-tests.lkjc",
        source: include_str!("../../../docs/guides/examples/editor-tests.lkjc"),
        base: "EDITOR_BASE",
        local_imports: None,
    },
];

pub(super) fn descriptor() -> Result<Vec<u8>, Diagnostic> {
    from_source(include_bytes!(
        "../../../docs/guides/examples/editor.deployment.json"
    ))
}

fn from_source(source: &[u8]) -> Result<Vec<u8>, Diagnostic> {
    let mut descriptor = decode_deployment(source)?;
    // One shared security/limit policy. Only starter-owned artifact/data placement
    // differs; decoding and encoding are the maintained strict descriptor owners.
    if descriptor.target != TARGET || descriptor.listen.as_deref() != Some(LISTENER) {
        return Err(policy_drift());
    }
    if descriptor.secrets.len() != 1 || descriptor.secrets[0].variable != SECRET_VARIABLE {
        return Err(policy_drift());
    }
    let mut data_roots = 0;
    for grant in &mut descriptor.grants {
        if let AdapterDescriptor::Data {
            root, namespace, ..
        } = &mut grant.adapter
        {
            if grant.requirement != "data" || root != "data" || namespace != "native-editor" {
                return Err(policy_drift());
            }
            data_roots += 1;
            *root = DATA_ROOT.to_owned();
        }
    }
    if data_roots != 1 {
        return Err(policy_drift());
    }
    descriptor.artifact = STARTER_HTTP_ARTIFACT_PATH.to_owned();
    encode_deployment(&descriptor)
}

fn policy_drift() -> Diagnostic {
    creation_error(
        DiagnosticClass::Corrupt,
        "new_editor_deployment",
        "the embedded editor deployment changed; review its starter linkage before use",
    )
}

#[cfg(test)]
mod tests;
