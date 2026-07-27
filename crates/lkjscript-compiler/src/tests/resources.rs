#![allow(clippy::unwrap_used)]

//! Typed reservations measure immutable phase input before the named target
//! phase allocates. Fixed parser, HIR, IR, and bytecode limits remain active.

use std::path::{Path, PathBuf};

use super::*;
use crate::{
    compile_path_with_profile, compile_path_with_profile_and_metrics, ResourceCategory,
    ResourceProfile, ResourceProfileName,
};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn canonical_corpus_compiles_under_every_registered_profile() {
    let roots = [
        "src/examples/bench/main.lkjscript",
        "src/examples/brainfuck/main.lkjscript",
        "src/examples/bulk-bytes/main.lkjscript",
        "src/examples/durable-files/main.lkjscript",
        "src/examples/hello/main.lkjscript",
        "src/examples/http/hello.lkjscript",
        "src/examples/jit-optimizing/main.lkjscript",
        "src/examples/jit-scalar/main.lkjscript",
        "src/examples/lkjedit/main.lkjscript",
        "src/examples/mandel/main.lkjscript",
        "src/examples/sha256/main.lkjscript",
        "src/examples/sqlite/main.lkjscript",
    ];
    for name in ResourceProfileName::ALL {
        let profile = ResourceProfile::new(name);
        for root in roots {
            compile_path_with_profile(&workspace().join(root), &Limits::default(), profile)
                .unwrap_or_else(|error| panic!("{} {root}: {error}", name.as_str()));
        }
    }
}

#[test]
fn profile_identity_and_nested_phase_usage_reach_metrics_and_output() {
    let profile = ResourceProfile::new(ResourceProfileName::Deterministic);
    let path = workspace().join("src/examples/hello/main.lkjscript");
    let (program, metrics) =
        compile_path_with_profile_and_metrics(&path, &Limits::default(), profile).unwrap();
    assert_eq!(program.profile(), profile.identity());
    assert_eq!(metrics.profile, profile.identity());
    assert_eq!(metrics.resources.profile(), profile.identity());
    for category in [
        ResourceCategory::SourceBytes,
        ResourceCategory::ParserWork,
        ResourceCategory::ValidationWork,
        ResourceCategory::HirExpressions,
        ResourceCategory::TypeWork,
        ResourceCategory::OwnershipExpressions,
        ResourceCategory::SsaFunctions,
        ResourceCategory::SsaBlocks,
        ResourceCategory::SsaValues,
    ] {
        assert!(metrics.resources.used(category) > 0, "{category:?}");
    }
}

#[test]
fn exact_source_ceiling_succeeds_and_plus_one_is_structured() {
    let source = canonical_source(&unit_main(""));
    let exact = u64::try_from(source.len()).unwrap();
    let profile = ResourceProfile::default()
        .lowered(ResourceCategory::SourceBytes, exact)
        .unwrap();
    compile_source_with_profile(&source, "exact.lkjscript", &Limits::default(), profile).unwrap();

    let lowered = ResourceProfile::default()
        .lowered(ResourceCategory::SourceBytes, exact - 1)
        .unwrap();
    let error =
        compile_source_with_profile(&source, "too-large.lkjscript", &Limits::default(), lowered)
            .unwrap_err();
    let rejection = error.budget_error().unwrap();
    assert_eq!(rejection.category, ResourceCategory::SourceBytes);
    assert_eq!(
        rejection.authority,
        Some(crate::BudgetAuthority::SourceLoading)
    );
    assert_eq!(rejection.limit, exact - 1);
    assert_eq!(rejection.observed, 0);
    assert_eq!(rejection.attempted, exact);
    assert!(!rejection.allocated_before_rejection);
}

#[test]
fn profiles_do_not_change_type_or_ownership_safety() {
    let invalid_type = "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n1\n/main\n";
    let invalid_reference = "def/\nname/\nreturn-ref\n/name\nfn/\nsig/\ninputs/\nbyte-slice\n/inputs\noutput/\nbyte-slice\n/output\n/sig\nparams/\nr\nbyte-slice\n/params\nr\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    for name in ResourceProfileName::ALL {
        let profile = ResourceProfile::new(name);
        let type_error = compile_source_with_profile(
            invalid_type,
            "invalid-type.lkjscript",
            &Limits::default(),
            profile,
        )
        .unwrap_err();
        assert!(type_error.compiler_resource_diagnostic().is_none());
        assert!(type_error.budget_error().is_none());
        let ownership_error = compile_source_with_profile(
            invalid_reference,
            "invalid-reference.lkjscript",
            &Limits::default(),
            profile,
        )
        .unwrap_err();
        assert!(ownership_error.to_string().contains("cannot be returned"));
    }
}

#[test]
fn diagnostic_publication_is_profile_charged() {
    let profile = ResourceProfile::default()
        .lowered(ResourceCategory::Diagnostics, 0)
        .unwrap();
    let error = compile_source_with_profile(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n1\n/main\n",
        "diagnostic.lkjscript",
        &Limits::default(),
        profile,
    )
    .unwrap_err();
    let rejection = error.budget_error().unwrap();
    assert_eq!(rejection.category, ResourceCategory::Diagnostics);
    assert_eq!(
        rejection.authority,
        Some(crate::BudgetAuthority::Diagnostics)
    );
    assert_eq!(rejection.limit, 0);
    assert_eq!(rejection.attempted, 1);
}
