#![allow(clippy::unwrap_used)]

//! Aggregate source/HIR/SSA counts are exact post-phase checks. Existing
//! Edition 1, Foundation V1, ownership, and IR bounds protect phase allocation;
//! profile failure is checked before the next phase or executable publication.

use std::path::{Path, PathBuf};

use super::*;
use crate::{
    compile_path_with_profile, compile_path_with_profile_and_metrics, compile_source_with_profile,
    ResourceCategory, ResourceProfile, ResourceProfileName,
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
    let source = unit_main("");
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
    let diagnostic = error.compiler_resource_diagnostic().unwrap();
    assert_eq!(diagnostic.category, ResourceCategory::SourceBytes);
    assert_eq!(diagnostic.limit, exact - 1);
    assert_eq!(diagnostic.before, 0);
    assert_eq!(diagnostic.increment, exact);
}

#[test]
fn profiles_do_not_change_type_or_ownership_safety() {
    let invalid_type = "main/\nsig/\n->\nBool\n/sig\n1\n/main\n";
    let invalid_reference = "def/\nname/\nreturn-ref\n/name\nfn/\nsig/\nRef\nBuf\n->\nRef\nBuf\n/sig\nparams/\nr\nRef/\nBuf\n/Ref\n/params\nr\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
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
        "main/\nsig/\n->\nBool\n/sig\n1\n/main\n",
        "diagnostic.lkjscript",
        &Limits::default(),
        profile,
    )
    .unwrap_err();
    let diagnostic = error.compiler_resource_diagnostic().unwrap();
    assert_eq!(diagnostic.category, ResourceCategory::Diagnostics);
    assert_eq!(diagnostic.limit, 0);
    assert_eq!(diagnostic.increment, 1);
}

#[test]
fn enum_shape_categories_are_reserved_before_hir_at_exact_and_plus_one() {
    let source = concat!(
        "edition/\n2\n/edition\n",
        "enum/\nname/\nMaybe\n/name\nforall/\nT\n/forall\nvariants/\n",
        "variant/\nname/\nNone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nSome\n/name\nfields/\nvariant-field/\n",
        "name/\nvalue\n/name\ntype/\nT\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
    );
    let counts = [
        (ResourceCategory::EnumDeclarations, 1),
        (ResourceCategory::EnumVariants, 2),
        (ResourceCategory::VariantFields, 1),
        (
            ResourceCategory::EnumRecursionWork,
            u64::try_from(crate::hir::ENUM_RECURSION_MAX_WORK).unwrap(),
        ),
    ];
    let mut exact = ResourceProfile::default();
    for (category, count) in counts {
        exact = exact.lowered(category, count).unwrap();
    }
    compile_source_with_profile(source, "enum-exact.lkjscript", &Limits::default(), exact)
        .expect("exact enum reservations");
    for (category, count) in counts {
        let profile = ResourceProfile::default()
            .lowered(category, count - 1)
            .unwrap();
        let error = compile_source_with_profile(
            source,
            "enum-plus-one.lkjscript",
            &Limits::default(),
            profile,
        )
        .unwrap_err();
        let diagnostic = error.compiler_resource_diagnostic().unwrap();
        assert_eq!(diagnostic.category, category);
        assert_eq!(diagnostic.before, 0);
        assert_eq!(diagnostic.increment, count);
    }
}

#[test]
fn lowered_hir_ceiling_prevents_ssa_and_is_deterministic() {
    let source = unit_main("");
    let profile = ResourceProfile::new(ResourceProfileName::Deterministic)
        .lowered(ResourceCategory::HirFunctions, 0)
        .unwrap();
    let compile = || {
        compile_source_with_profile(&source, "lowered.lkjscript", &Limits::default(), profile)
            .unwrap_err()
    };
    let first = compile();
    let second = compile();
    assert_eq!(first, second);
    let diagnostic = first.compiler_resource_diagnostic().unwrap();
    assert_eq!(diagnostic.category, ResourceCategory::HirFunctions);
    assert_eq!(diagnostic.before, 0);
    assert_eq!(diagnostic.increment, 1);
    assert_eq!(diagnostic.profile, profile.identity());
}
