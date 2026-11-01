//! Namespace mutations preserve framing, references and syntax ownership.
use super::*;
use kernel::NamespaceClass;

fn fixture() -> KernelSnapshot {
    byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-namespace.lkjc"
    ))
    .unwrap()
}

/// A complete source with two same-named top-level modules. The existing class
/// inventory controls use this same namespace mutation, independent of E2 forms.
pub(super) fn artifact_rejection_fixture() -> Result<KernelSnapshot, String> {
    let mut source = fixture();
    let mut modules = source
        .owners
        .iter()
        .filter_map(|(key, record)| match record {
            OwnerRecord::Module(record) => Some((*key, record.name.clone())),
            _ => None,
        });
    let (_, name) = modules
        .next()
        .ok_or_else(|| "namespace first module absent".to_owned())?;
    let (other, _) = modules
        .next()
        .ok_or_else(|| "namespace second module absent".to_owned())?;
    *name_mut(
        source
            .owners
            .get_mut(&other)
            .ok_or_else(|| "namespace module disappeared".to_owned())?,
    ) = name;
    rebind_owner_inventory(&mut source);
    Ok(source)
}

fn name_mut(record: &mut OwnerRecord) -> &mut kernel::Name {
    match record {
        OwnerRecord::Module(value) => &mut value.name,
        OwnerRecord::Declaration(value) => &mut value.name,
        OwnerRecord::TypeParameter(value) => &mut value.name,
        OwnerRecord::EffectParameter(value) => &mut value.name,
        OwnerRecord::RequirementParameter(value) => &mut value.name,
        OwnerRecord::Field(value) => &mut value.name,
        OwnerRecord::Case(value) => &mut value.name,
        OwnerRecord::Operation(value) => &mut value.name,
        OwnerRecord::Parameter(value) => &mut value.name,
        OwnerRecord::Requirement(value) => &mut value.name,
        OwnerRecord::Port(value) => &mut value.name,
        OwnerRecord::Target(value) => &mut value.name,
        _ => panic!("namespace owner"),
    }
}

fn rebind_owner_inventory(source: &mut KernelSnapshot) {
    let entries = source
        .owners
        .iter()
        .map(|(key, record)| {
            let (object, _) = kernel::encode_owner(record).unwrap();
            (
                kernel::owner_map_key(*key).to_vec(),
                kernel::encode_owner_binding(&kernel::OwnerBinding {
                    kind: record.kind(),
                    object,
                }),
            )
        })
        .collect::<Vec<_>>();
    let content = crate::platform::persistent_map::MapContentRoot::from_sorted(entries).unwrap();
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        content.digest(),
    );
}

fn expect(source: &KernelSnapshot, code: i64) {
    let bytes = graph::packet(source);
    assert_eq!(
        result("read-syntax", &bytes),
        (0, bytes[48..80].to_vec()),
        "namespace controls must pass every earlier admission layer"
    );
    assert_eq!(
        result("read-namespace", &bytes),
        (
            code,
            if code == 0 {
                bytes[48..80].to_vec()
            } else {
                vec![]
            }
        )
    );
    emitted_result("read-namespace", &bytes, code);
}

#[test]
fn selfhost_source_namespace_rejects_all_eight_decoded_class_collisions() {
    let base = fixture();
    expect(&base, 0);
    for class in [
        NamespaceClass::Module,
        NamespaceClass::Declaration,
        NamespaceClass::TypeParameter,
        NamespaceClass::Field,
        NamespaceClass::Case,
        NamespaceClass::Parameter,
        NamespaceClass::Port,
        NamespaceClass::Target,
    ] {
        let mut groups = BTreeMap::<Option<OwnerKey>, Vec<(OwnerKey, kernel::Name)>>::new();
        for (key, record) in &base.owners {
            if let Some(entry) = kernel::owner_namespace(record)
                && entry.class == class
            {
                groups
                    .entry(entry.parent)
                    .or_default()
                    .push((*key, entry.name.clone()));
            }
        }
        let pair = groups.values().find(|owners| owners.len() >= 2).unwrap();
        let mut changed = base.clone();
        *name_mut(changed.owners.get_mut(&pair[1].0).unwrap()) = pair[0].1.clone();
        rebind_owner_inventory(&mut changed);
        let errors = kernel::validate_full(&changed).unwrap_err();
        assert!(
            errors.iter().any(
                |error| error.class == crate::platform::DiagnosticClass::Semantic
                    && error.code == "kernel_full_namespace_duplicate"
            ),
            "{class:?}: {errors:?}"
        );
        expect(&changed, 16);
    }
}

#[test]
fn selfhost_source_namespace_preserves_parent_class_and_local_shadowing() {
    let mut source = fixture();
    // A type parameter and a field have one parent but different namespaces.
    let (parent, name) = source
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Field(record) if record.name.as_str() == "a" => {
                Some((record.declaration, record.name.clone()))
            }
            _ => None,
        })
        .unwrap();
    let parameter = source
        .owners
        .values_mut()
        .find_map(|record| match record {
            OwnerRecord::TypeParameter(record) if record.declaration == parent => Some(record),
            _ => None,
        })
        .unwrap();
    parameter.name = name;
    rebind_owner_inventory(&mut source);
    kernel::validate_full(&source).unwrap();
    expect(&source, 0);

    let mut lexical = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-lexical.lkjc"
    ))
    .unwrap();
    for record in lexical.owners.values_mut() {
        if let OwnerRecord::Binding(binding) = record {
            binding.name = kernel::Name::new("shadow").unwrap();
        }
    }
    rebind_owner_inventory(&mut lexical);
    kernel::validate_full(&lexical).unwrap();
    expect(&lexical, 0);
}

#[test]
fn selfhost_source_namespace_preserves_earlier_malformed_diagnostic() {
    let source = fixture();
    let mut bytes = graph::packet(&source);
    bytes[..8].copy_from_slice(b"notgraph");
    assert_eq!(result("read-namespace", &bytes), (12, vec![]));
    emitted_result("read-namespace", &bytes, 12);
}

#[test]
fn selfhost_source_namespace_respects_exact_allowance_boundary() {
    exact_budget_boundary("read-namespace-budget", &fixture());
}

#[test]
fn selfhost_source_namespace_maps_all_twelve_classes_and_both_parameter_parents() {
    let (_, program) = probes("source-namespace-probe");
    let target = program
        .function(function("source-namespace-probe", "read-namespace-key"))
        .unwrap()
        .0 as usize;
    let input = input::export(program, target).unwrap();
    let (source, compiler) = compiler();
    let (code, image) =
        compile_with_policy(source, compiler, &input, NormalizedRunPolicy::foreground());
    assert_eq!(code, 0);
    eprintln!(
        "selfhost namespace key projection bytes={} sha256={}",
        image.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&image))
    );
    // OwnerRecord variants, namespace classes and OwnerKey parent domains are
    // independently listed from the public kernel contract, not the reader.
    for (tag, parent, class, domain) in [
        (0, 0, 1, 0),
        (1, 0, 2, 1),
        (2, 0, 3, 2),
        (3, 0, 11, 2),
        (4, 0, 4, 2),
        (5, 0, 5, 2),
        (6, 0, 6, 2),
        (7, 0, 7, 2),
        (7, 1, 7, 6),
        (10, 0, 8, 2),
        (11, 0, 9, 2),
        (12, 0, 10, 0),
        (16, 0, 12, 2),
        (8, 0, 0, 0),
        (9, 0, 0, 0),
        (13, 0, 0, 0),
        (14, 0, 0, 0),
        (15, 0, 0, 0),
    ] {
        let bytes = [tag, parent];
        assert_eq!(
            result("read-namespace-key", &bytes),
            (0, vec![class, domain])
        );
        let output = native_filter_tests::execute_filter(&image, &bytes, &[], 127);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, [class, domain]);
        assert!(output.stderr.is_empty());
    }
}
