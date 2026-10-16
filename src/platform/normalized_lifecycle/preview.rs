//! Read-only candidate execution. This type grants no accepted-repository authority.

use super::*;
use crate::platform::execution::normalized::run_selected_graph_tests;
use crate::platform::kernel::{DeclarationReference, OwnerKey};
use crate::platform::publication::{PreparedPublication, RepositoryView};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) struct PreparedPreview {
    binding: crate::platform::publication::AcceptedBinding,
    package: PackageId,
    program: Arc<NormalizedProgram>,
    reference: PackageReference,
}

/// Only the complete canonical inventory and both evaluators can mint this evidence.
pub(crate) struct VerifiedTransactionTests {
    binding: crate::platform::publication::AcceptedBinding,
    selected: BTreeSet<OwnerKey>,
    receipt: NormalizedTestReceipt,
}

impl VerifiedTransactionTests {
    pub(crate) fn receipt(&self) -> &NormalizedTestReceipt {
        &self.receipt
    }

    pub(crate) fn verify_binding(
        &self,
        binding: crate::platform::publication::AcceptedBinding,
        selected: &BTreeSet<OwnerKey>,
    ) -> Result<u64, Diagnostic> {
        if self.binding != binding
            || &self.selected != selected
            || self.receipt.revision != Some(binding.head.revision)
            || self.receipt.passed != selected.len() as u64
            || self.receipt.failed != 0
            || self.receipt.differential != "equal"
        {
            return Err(lifecycle_error(
                DiagnosticClass::Corrupt,
                "transaction_test_evidence",
                "differential evidence differs from the exact candidate or selection",
            ));
        }
        Ok(self.receipt.passed)
    }
}

impl PreparedPreview {
    pub(crate) fn prepare(
        view: RepositoryView,
        prepared: &PreparedPublication,
        control: &ExecutionControl,
    ) -> Result<Self, Diagnostic> {
        control
            .check()
            .map_err(super::super::execution::normalized::execution_diagnostic)?;
        let package = view.package();
        let closure = view.preview_source(prepared)?;
        Self::from_source(closure, prepared.accepted, package, control)
    }

    pub(crate) fn current(
        view: RepositoryView,
        control: &ExecutionControl,
    ) -> Result<Self, Diagnostic> {
        let binding = view.current().accepted;
        let package = view.package();
        Self::from_source(view.preview_current_source()?, binding, package, control)
    }

    fn from_source(
        closure: crate::platform::package_transport::source::AdmittedClosure,
        binding: crate::platform::publication::AcceptedBinding,
        package: PackageId,
        control: &ExecutionControl,
    ) -> Result<Self, Diagnostic> {
        let reference = reference_for_source(&closure, binding, package, control)?;
        let mut compiled = BTreeMap::new();
        for revision in &closure.dependency_order {
            control
                .check()
                .map_err(super::super::execution::normalized::execution_diagnostic)?;
            let source = &closure.packages[revision];
            let dependencies = source
                .revision
                .dependencies
                .iter()
                .map(|dependency| {
                    compiled
                        .get(&dependency.package_revision)
                        .cloned()
                        .ok_or_else(|| {
                            lifecycle_error(
                                DiagnosticClass::Corrupt,
                                "transaction_dependency_order",
                                "candidate dependency order is incomplete",
                            )
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if *revision == closure.container.root.package_revision {
                let linked = super::super::compiler::compile_immutable(
                    source,
                    &closure.container.objects,
                    &dependencies,
                )?;
                let (bytes, artifact) = linked.artifact.into_admitted()?;
                let program = super::super::builtin_standard::prepare_admitted_program(
                    &bytes, artifact, control,
                )?;
                return Ok(Self {
                    binding,
                    package,
                    program,
                    reference,
                });
            }
            let supplier = supplier_cache::prepare(&closure, *revision, &dependencies, control)?;
            compiled.insert(*revision, (*supplier.artifact).clone());
        }
        Err(lifecycle_error(
            DiagnosticClass::Corrupt,
            "transaction_root_missing",
            "candidate dependency order omitted its root",
        ))
    }

    pub(crate) fn check(
        &self,
        selected: &BTreeSet<OwnerKey>,
        control: &ExecutionControl,
    ) -> Result<VerifiedTransactionTests, Diagnostic> {
        let declarations = selected
            .iter()
            .map(|owner| {
                let OwnerKey::Declaration(declaration) = owner else {
                    return Err(lifecycle_error(
                        DiagnosticClass::Corrupt,
                        "transaction_test_selection",
                        "selected test is not a declaration",
                    ));
                };
                Ok(DeclarationReference {
                    package: self.package,
                    declaration: *declaration,
                })
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let receipt = run_selected_graph_tests(
            &self.reference,
            &self.program,
            &declarations,
            Default::default(),
            control,
        )?;
        Ok(VerifiedTransactionTests {
            binding: self.binding,
            selected: selected.clone(),
            receipt,
        })
    }

    pub(crate) fn run(
        &self,
        target: &Name,
        control: &ExecutionControl,
    ) -> Result<NormalizedCommandReceipt, Diagnostic> {
        run_pure_command(
            &self.reference,
            &self.program,
            target,
            b"[]",
            Default::default(),
            control,
        )
    }
}
