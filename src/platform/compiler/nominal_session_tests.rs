//! Rehashed command-to-session artifacts must independently pass session admission.
use super::*;
use crate::platform::package::RunnerKind;

// This bounded test packer deliberately skips encode_artifact's self-admission. Only neutral
// immutable-pack bytes and the documented envelope are shared with the strict loader.
fn hostile_bundle(manifest: &ArtifactManifest, objects: &BTreeMap<ObjectKey, Vec<u8>>) -> Vec<u8> {
    assert!(objects.len() < 10_000);
    let (digest, manifest_bytes) = manifest.encode().unwrap();
    let mut builder = PackBuilder::default();
    for (key, bytes) in objects {
        builder.insert(*key, bytes).unwrap();
    }
    let segments = builder.seal_targeted(4 * 1024 * 1024).unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&ARTIFACT_BUNDLE_MAGIC);
    bytes.extend_from_slice(&ARTIFACT_CONTRACT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&(manifest_bytes.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&(segments.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&digest.bytes());
    bytes.extend_from_slice(&manifest_bytes);
    for segment in segments {
        bytes.extend_from_slice(&(segment.bytes.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&segment.id.bytes());
        bytes.extend_from_slice(&segment.bytes);
    }
    assert!(bytes.len() < 4 * 1024 * 1024);
    let mut hash = blake3::Hasher::new_derive_key(ARTIFACT_BUNDLE_CHECKSUM_DOMAIN);
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(&bytes);
    bytes.extend_from_slice(hash.finalize().as_bytes());
    bytes.extend_from_slice(&ARTIFACT_BUNDLE_END_MAGIC);
    bytes
}

fn interactive_artifact(loaded: &LoadedArtifact) -> Vec<u8> {
    let mut manifest = loaded.manifest.clone();
    let mut objects = loaded.objects.clone();
    let package = manifest
        .packages
        .iter_mut()
        .find(|p| p.package == manifest.root_package)
        .unwrap();
    let binding = package
        .runtime_owners
        .iter_mut()
        .find(|binding| {
            if binding.kind != OwnerKind::Target {
                return false;
            }
            let record = decode_owner(
                &objects[&ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes())],
                binding.owner,
                binding.kind,
                binding.object,
            )
            .unwrap();
            matches!(record, OwnerRecord::Target(target) if target.name.as_str() == "boundary")
        })
        .unwrap();
    let target = binding.owner;
    let old = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
    let mut record = decode_owner(
        &objects.remove(&old).unwrap(),
        binding.owner,
        binding.kind,
        binding.object,
    )
    .unwrap();
    let OwnerRecord::Target(target_record) = &mut record else {
        panic!("target")
    };
    target_record.runner = RunnerKind::Interactive;
    let (digest, bytes) = encode_owner(&record).unwrap();
    binding.object = digest;
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
        bytes,
    );

    let old_compilation = package.compilation;
    let mut compilation =
        CompilationManifest::decode(&objects[&old_compilation.object_key()], old_compilation)
            .unwrap();
    let mut entries = artifact_map_entries(loaded, compilation.units);
    for (owner, value) in &mut entries {
        let owner = crate::platform::kernel::EncodedOwnerKey::decode(owner).unwrap();
        if owner != target {
            continue;
        }
        let mut binding = CompilationBinding::decode(value, owner).unwrap();
        let old = binding.object.object_key();
        let mut unit = CompilationUnit::decode(&objects.remove(&old).unwrap(), old).unwrap();
        let CompilationPayload::Target { runner, .. } = &mut unit.payload else {
            panic!("target unit")
        };
        *runner = RunnerKind::Interactive;
        let bytes = crate::platform::packed::encode(
            COMPILER_UNIT_MAGIC,
            COMPILER_UNIT_ENVELOPE_DOMAIN,
            &unit,
            super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
        )
        .unwrap();
        let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes);
        objects.insert(key, bytes);
        binding.object = CompilerUnitObjectDigest::from_bytes(key.digest.bytes());
        *value = binding.encode(owner).unwrap();
    }
    let mut old_pages = MemoryPageStore::default();
    PersistentMap::from_root(compilation.units)
        .copy_reachable(
            &ObjectPageReader::new(loaded),
            &mut old_pages,
            &mut MapWork::default(),
        )
        .unwrap();
    for (digest, _) in old_pages.objects() {
        objects.remove(&ObjectKey::from_digest(
            ObjectDomain::MapPage,
            digest.bytes(),
        ));
    }
    compilation.units = replace_artifact_map(&mut objects, entries);
    objects.remove(&old_compilation.object_key());
    let (digest, bytes) = compilation.encode().unwrap();
    package.compilation = digest;
    objects.insert(digest.object_key(), bytes);
    let (closure, count, bytes) = super::super::artifact::closure_facts(&objects).unwrap();
    manifest.closure = closure;
    manifest.object_count = count;
    manifest.object_bytes = bytes;
    hostile_bundle(&manifest, &objects)
}

#[test]
fn strict_artifact_and_deployment_reject_applied_session_state_before_readiness() {
    use crate::platform::{
        deployment::PreparedDeployment,
        project_creation::{ProjectTemplate, create_project},
    };
    for (argument, repeated, expected) in [
        ("i64", "i64", None),
        ("i64", "text", Some("session_port_state_identity")),
        ("secret", "secret", Some("session_state_live_type")),
        ("@Callable", "@Callable", Some("session_state_live_type")),
        ("@Nested", "@Nested", Some("session_state_live_type")),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let project = temporary.path().join("source");
        let created =
            create_project(&project, "session-boundary", ProjectTemplate::Command).unwrap();
        let repository = GraphRepository::open(&project).unwrap();
        let request = format!(
            "request base={}\n{}",
            created.revision,
            include_str!("../../../tests/fixtures/nominal-session-command.lkchg")
                .replace("ARGUMENT", argument)
                .replace("REPEATED", repeated)
        );
        let decoded =
            crate::platform::control::decode_compact_change("session-boundary", request.as_bytes())
                .unwrap();
        let prepared = repository
            .prepare_authored_change(&decoded.semantic, decoded.options)
            .unwrap();
        assert!(matches!(
            repository.publish(&prepared.publication).unwrap(),
            PublicationOutcome::Accepted { .. }
        ));
        let head = std::fs::read(project.join("HEAD")).unwrap();
        let application =
            crate::platform::normalized_lifecycle::prepare_repository(repository).unwrap();
        let loaded = load_artifact(&application.artifact_bytes).unwrap();
        let bytes = interactive_artifact(&loaded);
        let path = temporary.path().join("session.lkja");
        std::fs::write(path, &bytes).unwrap();
        let descriptor = serde_json::json!({
            "artifact":"session.lkja", "target":"boundary", "listen":"127.0.0.1:0",
            "runtime":crate::platform::runtime::ResidentLimits::default(),
            "execution":crate::platform::execution::RunPolicy::default(),
            "http":null, "session":crate::platform::session::SessionLimits::default(),
            "worker":null, "streams":crate::platform::stream::StreamLimits::default(),
            "configuration":{}, "secrets":[],
            "grants":[{"requirement":"streams", "sharing_domain":"isolated-session",
                "authority_revision":"11".repeat(32), "adapter":{"kind":"byte_stream"}}]
        });
        let descriptor_path = temporary.path().join("session.deployment.json");
        std::fs::write(&descriptor_path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        if let Some(expected) = expected {
            let failure = load_artifact(&bytes).unwrap_err();
            assert_eq!(
                failure.code, "artifact_session_relation",
                "{argument}/{repeated}: {failure}"
            );
            assert!(failure.message.contains(expected));
            let failure = PreparedDeployment::load(&descriptor_path, runtime.handle().clone())
                .expect_err("deployment rejected");
            assert_eq!(failure.code, "artifact_session_relation");
            assert!(failure.message.contains(expected));
            println!(
                "nominal-session-strict-negative {argument}/{repeated} {expected} before-readiness"
            );
        } else {
            load_artifact(&bytes).unwrap();
            let deployment =
                PreparedDeployment::load(&descriptor_path, runtime.handle().clone()).unwrap();
            let session = deployment.session_application().unwrap();
            assert_eq!(runtime.block_on(session.shutdown()).remaining_tasks, 0);
        }
        assert_eq!(std::fs::read(project.join("HEAD")).unwrap(), head);
    }
}
