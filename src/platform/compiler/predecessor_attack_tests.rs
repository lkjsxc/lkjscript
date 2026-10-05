//! Test-only transport of a frozen adversarial graph-14 artifact into current derived envelopes.
//! Canonical source and instructions are untouched. This does not admit or execute the fixture.
use super::*;
use crate::platform::kernel::{EncodedOwnerKey, decode_owner_binding};
use crate::platform::persistent_map::PageStore;

pub(crate) fn current_derived_fixture(bytes: &[u8]) -> Vec<u8> {
    current_derived_fixture_from_source(bytes, None)
}

fn current_derived_fixture_from_source(
    bytes: &[u8],
    source: Option<&crate::platform::package_transport::oracle::OracleClosure>,
) -> Vec<u8> {
    let (magic, domain) = match &bytes[..8] {
        b"LKJART18" => (*b"LKJAMF18", "lkjscript.artifact-manifest-envelope.v18"),
        b"LKJART19" => (*b"LKJAMF19", "lkjscript.artifact-manifest-envelope.v19"),
        b"LKJART20" => (*b"LKJAMF20", "lkjscript.artifact-manifest-envelope.v20"),
        b"LKJART21" => (*b"LKJAMF21", "lkjscript.artifact-manifest-envelope.v21"),
        _ => panic!("fixture generation"),
    };
    let number = |at| u64::from_be_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
    let end = 60 + number(12);
    let mut manifest: ArtifactManifest = crate::platform::packed::decode(
        &bytes[60..end],
        magic,
        domain,
        super::super::artifact::MAXIMUM_ARTIFACT_MANIFEST_BYTES,
    )
    .unwrap();
    let mut objects = BTreeMap::new();
    let mut at = end;
    for _ in 0..number(20) {
        let end = at + 40 + number(at);
        let segment = &bytes[at + 40..end];
        let pack = PackMetadata::decode(segment, true).unwrap();
        for entry in &pack.entries {
            let value = pack
                .read(segment, entry.key, entry.key.domain.maximum_bytes())
                .unwrap()
                .unwrap();
            assert!(objects.insert(entry.key, value).is_none());
        }
        at = end;
    }
    assert_eq!(at + 40, bytes.len());
    let mut pages = MemoryPageStore::default();
    for (key, value) in &objects {
        if key.domain == ObjectDomain::MapPage {
            pages
                .write_page(PageDigest::from_bytes(key.digest.bytes()), value)
                .unwrap();
        }
    }
    let mut replacements = BTreeMap::new();
    let mut current_units = BTreeMap::new();
    for (key, value) in objects.clone() {
        if key.domain != ObjectDomain::CompilerUnit {
            continue;
        }
        let mut unit: CompilationUnit = if value.starts_with(b"LKJCUN10") {
            crate::platform::packed::decode::<super::super::wire10::CompilationUnit10>(
                &value,
                *b"LKJCUN10",
                "lkjscript.compiler-unit-envelope.v10",
                super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
            )
            .unwrap()
            .into()
        } else {
            let (magic, domain) = match &value[..8] {
                b"LKJCUN11" => (*b"LKJCUN11", "lkjscript.compiler-unit-envelope.v11"),
                b"LKJCUN12" => (*b"LKJCUN12", "lkjscript.compiler-unit-envelope.v12"),
                b"LKJCUN13" => (*b"LKJCUN13", "lkjscript.compiler-unit-envelope.v13"),
                b"LKJCUN14" => (*b"LKJCUN14", "lkjscript.compiler-unit-envelope.v14"),
                _ => panic!("frozen unit generation"),
            };
            crate::platform::packed::decode::<super::predecessor_units::Unit>(
                &value,
                magic,
                domain,
                super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
            )
            .unwrap()
            .current()
        };
        unit.contract_version = super::super::unit::COMPILER_UNIT_CONTRACT_VERSION;
        unit.graph_contract_version = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
        unit.bytecode_contract_version = super::super::unit::BYTECODE_CONTRACT_VERSION;
        unit.key = CompilationUnitKey::derive(&unit.source, unit.optimization).unwrap();
        let (new, encoded) = unit.encode().unwrap();
        objects.remove(&key).unwrap();
        objects.insert(new, encoded);
        replacements.insert(key, (new, unit.key));
        assert!(
            current_units
                .insert((unit.source.package, unit.source.owner), unit)
                .is_none()
        );
    }
    if source.is_some() {
        // Runtime metadata is derived too. Its current exact owner inventory may
        // differ, but every selected record must already exist in the frozen source
        // or runtime map. Never synthesize or edit a canonical owner to admit a fixture.
        let expected = super::super::artifact::runtime_owner_expectations(
            &current_units,
            &mut 0,
            |ty| {
                let key = ObjectKey::from_digest(ObjectDomain::Type, ty.bytes());
                crate::platform::kernel::decode_type_object(&objects[&key], ty)
            },
            || Ok(()),
        )
        .unwrap();
        for package in &mut manifest.packages {
            let mut available = package
                .runtime_owners
                .iter()
                .map(|binding| (binding.owner, *binding))
                .collect::<BTreeMap<_, _>>();
            let mut reference_bindings = BTreeMap::new();
            PersistentMap::from_root(package.reference_owners)
                .for_each(&pages, &mut MapWork::default(), |key, value| {
                    let owner = EncodedOwnerKey::decode(key).unwrap();
                    let binding = decode_owner_binding(value, owner).unwrap();
                    reference_bindings.insert(owner, binding.clone());
                    let original = super::super::artifact::ArtifactRuntimeOwner {
                        owner,
                        kind: binding.kind,
                        object: binding.object,
                    };
                    if let Some(previous) = available.insert(owner, original) {
                        assert_eq!(previous, original, "frozen runtime/reference owner differs");
                    }
                    Ok(())
                })
                .unwrap();
            if let Some(snapshot) = source.and_then(|source| source.snapshots.get(&package.package))
            {
                for ((id, owner), expectation) in &expected {
                    if *id != package.package || available.contains_key(owner) {
                        continue;
                    }
                    let record = snapshot.owners.get(owner).unwrap_or_else(|| {
                        panic!("transport omitted current boundary owner {owner}")
                    });
                    let (object, encoded) = crate::platform::kernel::encode_owner(record).unwrap();
                    assert_eq!(record.kind(), expectation.kind());
                    let key = ObjectKey::from_digest(ObjectDomain::Owner, object.bytes());
                    if let Some(previous) = objects.insert(key, encoded.clone()) {
                        assert_eq!(previous, encoded, "original owner bytes cannot change");
                    }
                    available.insert(
                        *owner,
                        super::super::artifact::ArtifactRuntimeOwner {
                            owner: *owner,
                            kind: record.kind(),
                            object,
                        },
                    );
                }
            }
            if let Some(snapshot) = source.and_then(|source| source.snapshots.get(&package.package))
            {
                for ((id, owner), unit) in &current_units {
                    if *id != package.package
                        || reference_bindings.contains_key(owner)
                        || !matches!(unit.payload, CompilationPayload::External { .. })
                    {
                        continue;
                    }
                    let record = snapshot
                        .owners
                        .get(owner)
                        .expect("original external declaration");
                    let (object, encoded) = crate::platform::kernel::encode_owner(record).unwrap();
                    let key = ObjectKey::from_digest(ObjectDomain::Owner, object.bytes());
                    if let Some(previous) = objects.insert(key, encoded.clone()) {
                        assert_eq!(previous, encoded, "original external bytes cannot change");
                    }
                    reference_bindings.insert(
                        *owner,
                        crate::platform::kernel::OwnerBinding {
                            kind: record.kind(),
                            object,
                        },
                    );
                }
                let entries = reference_bindings
                    .iter()
                    .map(|(owner, binding)| {
                        (
                            EncodedOwnerKey::new(*owner).bytes().to_vec(),
                            crate::platform::kernel::encode_owner_binding(binding),
                        )
                    })
                    .collect();
                package.reference_owners = replace_artifact_map(&mut objects, entries);
            }
            let previous = package.runtime_owners.len();
            package.runtime_owners = expected
                .iter()
                .filter(|((id, _), _)| *id == package.package)
                .map(|((_, owner), expectation)| {
                    let binding = *available
                        .get(owner)
                        .unwrap_or_else(|| panic!("missing original runtime owner {owner}"));
                    assert_eq!(binding.kind, expectation.kind());
                    binding
                })
                .collect();
            if previous != package.runtime_owners.len() {
                println!(
                    "derived runtime inventory {}: {previous} -> {}",
                    package.package,
                    package.runtime_owners.len()
                );
            }
        }
    }
    for package in &mut manifest.packages {
        let old = package.compilation.object_key();
        let mut compilation =
            CompilationManifest::decode(&objects.remove(&old).unwrap(), package.compilation)
                .unwrap();
        let map = PersistentMap::from_root(compilation.units);
        let mut entries = Vec::new();
        map.for_each(&pages, &mut MapWork::default(), |owner, value| {
            let owner_key = crate::platform::kernel::EncodedOwnerKey::decode(owner).unwrap();
            let mut binding = CompilationBinding::decode(value, owner_key).unwrap();
            let (object, key) = replacements[&binding.object.object_key()];
            binding.object = CompilerUnitObjectDigest::from_bytes(object.digest.bytes());
            binding.key = key;
            entries.push((owner.to_vec(), binding.encode(owner_key).unwrap()));
            Ok(())
        })
        .unwrap();
        let mut old_pages = MemoryPageStore::default();
        map.copy_reachable(&pages, &mut old_pages, &mut MapWork::default())
            .unwrap();
        for (digest, _) in old_pages.objects() {
            objects.remove(&ObjectKey::from_digest(
                ObjectDomain::MapPage,
                digest.bytes(),
            ));
        }
        compilation.units = replace_artifact_map(&mut objects, entries);
        compilation.graph_contract_version =
            crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
        compilation.compiler_contract_version = super::super::unit::COMPILER_UNIT_CONTRACT_VERSION;
        compilation.bytecode_contract_version = super::super::unit::BYTECODE_CONTRACT_VERSION;
        let (digest, encoded) = compilation.encode().unwrap();
        package.compilation = digest;
        objects.insert(digest.object_key(), encoded);
    }
    if source.is_some() {
        let mut all_pages = MemoryPageStore::default();
        for (key, bytes) in &objects {
            if key.domain == ObjectDomain::MapPage {
                all_pages
                    .write_page(PageDigest::from_bytes(key.digest.bytes()), bytes)
                    .unwrap();
            }
        }
        let mut live = MemoryPageStore::default();
        for package in &manifest.packages {
            let compilation = CompilationManifest::decode(
                &objects[&package.compilation.object_key()],
                package.compilation,
            )
            .unwrap();
            for root in [
                package.interface_owners,
                package.reference_owners,
                compilation.units,
            ] {
                PersistentMap::from_root(root)
                    .copy_reachable(&all_pages, &mut live, &mut MapWork::default())
                    .unwrap();
            }
        }
        let retained = live
            .objects()
            .map(|(digest, _)| digest.bytes())
            .collect::<BTreeSet<_>>();
        objects.retain(|key, _| {
            key.domain != ObjectDomain::MapPage || retained.contains(&key.digest.bytes())
        });
    }
    manifest.contract_version = super::super::artifact::ARTIFACT_CONTRACT_VERSION;
    manifest.graph_contract_version = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    manifest.compiler_contract_version = super::super::unit::COMPILER_UNIT_CONTRACT_VERSION;
    manifest.bytecode_contract_version = super::super::unit::BYTECODE_CONTRACT_VERSION;
    let (closure, count, length) = super::super::artifact::closure_facts(&objects).unwrap();
    manifest.closure = closure;
    manifest.object_count = count;
    manifest.object_bytes = length;
    nominal_session_tests::hostile_bundle(&manifest, &objects)
}

#[test]
fn current_predecessor_controls_retain_exact_source_and_instructions() {
    use std::io::Write;
    // Do not overwrite the official/historical fixtures or confuse their old
    // admission with current permission. Only derived envelopes are replaced.
    let mut expected_files = Vec::new();
    for (name, original, transport, transport_id, rejection) in [
        (
            "requirements",
            include_bytes!("../../../tests/fixtures/requirement-predecessor/predecessor.lkja")
                .as_slice(),
            include_bytes!("../../../tests/fixtures/requirement-predecessor/predecessor.lkjp")
                .as_slice(),
            "package_transport_51f58676d0a4bc944c78c97370d945fe2a7fc575709cf46b34dcac7c4aae2d1e",
            "compiler_unit_contract",
        ),
        (
            "transactions",
            include_bytes!(
                "../../../tests/fixtures/transaction-outcome-predecessor/predecessor.lkja"
            )
            .as_slice(),
            include_bytes!(
                "../../../tests/fixtures/transaction-outcome-predecessor/predecessor.lkjp"
            )
            .as_slice(),
            "package_transport_3abb212b71ff3e56bdba07c0977bebffb1b8e336cc8092e982ab2398bb81a74d",
            "compiler_unit_contract",
        ),
        (
            "participation",
            include_bytes!(
                "../../../tests/fixtures/cell-participation-predecessor/predecessor.lkja"
            )
            .as_slice(),
            include_bytes!(
                "../../../tests/fixtures/cell-participation-predecessor/predecessor.lkjp"
            )
            .as_slice(),
            "package_transport_4c3f2f8387ee3ed16864681d951fc35317179d81d895bd60a43f1e9b603eed56",
            "artifact_bundle_contract",
        ),
    ] {
        let error = load_artifact(original).unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::diagnostic::DiagnosticClass::Source,
            "{name}: {error:?}"
        );
        assert_eq!(error.code, rejection, "{name}: {error:?}");
        let container = crate::platform::package_transport::source::PackageContainer::decode(
            transport,
            transport_id.parse().unwrap(),
        )
        .unwrap();
        let source = crate::platform::package_transport::oracle::reconstruct(&container).unwrap();
        let current = current_derived_fixture_from_source(original, Some(&source));
        let loaded = load_artifact(&current).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        crate::platform::contributor::strict_artifact_source_probe(
            &current,
            transport,
            transport_id,
        )
        .unwrap_or_else(|e| panic!("{name} source binding: {e:?}"));
        println!("current predecessor {name}: {}", loaded.bundle_digest);
        if let Some(directory) = std::env::var_os("LKJSCRIPT_WRITE_PREDECESSOR_FIXTURES") {
            let destination = std::path::Path::new(&directory).join(format!("{name}.lkja"));
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .unwrap()
                .write_all(&current)
                .unwrap();
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/owned-predecessor-compiler26")
            .join(format!("{name}.lkja"));
        expected_files.push((name, path, current));
    }
    for (name, path, current) in expected_files {
        let retained = std::fs::read(path).expect("retained current-envelope fixture");
        assert!(
            current == retained,
            "{name}: canonical source and instructions changed"
        );
    }
}
