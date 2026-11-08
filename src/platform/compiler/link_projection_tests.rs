//! Projection-only canonical reads stay inside the link's physical reread scope.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::kernel::OwnerObjectDigest;
use crate::platform::publication::PreparedAuthoredPublication;
use crate::platform::storage::object::StageOutcome;
use std::path::PathBuf;

fn prepare(repository: &GraphRepository, body: &str) -> PreparedAuthoredPublication {
    let input = format!(
        "request base={}\n{body}",
        repository.current().unwrap().head.revision
    );
    let request =
        crate::platform::control::decode_compact_change("link-projection", input.as_bytes())
            .unwrap();
    repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap()
}

struct Fixture {
    _directory: tempfile::TempDir,
    repository: GraphRepository,
    compilation: CompilationManifestDigest,
    victim: ObjectKey,
    pack: PathBuf,
    absent_pack: PathBuf,
    pack_bytes: Vec<u8>,
    payload_offset: usize,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(
            &directory.path().join("repository"),
            &structurally_empty_snapshot(b"link-projection-physical-reread"),
            None,
        )
        .unwrap()
        .repository;
        let prepared = prepare(
            &repository,
            r#"create.module as=$unused name=projection_only
create.module as=$app name=app
expression.block as=$body
  (i64 41)
expression.end
create.function as=$value module=$app name=value visibility=public result=i64 effect=pure body=$body
"#,
        );
        let owner = prepared.allocated["$unused"];
        assert!(matches!(owner, OwnerKey::Module(_)));
        let (victim, victim_bytes) = prepared
            .publication
            .objects
            .iter()
            .find_map(|(key, bytes)| {
                if key.domain != ObjectDomain::Owner {
                    return None;
                }
                decode_owner(
                    bytes,
                    owner,
                    OwnerKind::Module,
                    OwnerObjectDigest::from_bytes(key.digest.bytes()),
                )
                .ok()
                .map(|_| (*key, bytes))
            })
            .expect("prepared canonical unused module");

        // Isolate physical storage without adopting meaning. The unchanged public
        // publication below remains the sole operation that installs the owner.
        let before = repository.current().unwrap().head;
        {
            let mut store = repository.object_store().unwrap();
            let mut work = StoreWork::default();
            assert_eq!(
                store.stage(victim, victim_bytes, &mut work).unwrap(),
                StageOutcome::Inserted
            );
            assert_eq!(store.staged_len(), 1);
            let sealed = store
                .seal_staged(
                    crate::platform::storage::contract::TARGET_PACK_BYTES,
                    &mut work,
                )
                .unwrap();
            assert_eq!(store.staged_len(), 0);
            assert_eq!(sealed.packs.len(), 1);
        }
        assert_eq!(repository.current().unwrap().head, before);
        assert!(matches!(
            repository.publish(&prepared.publication).unwrap(),
            PublicationOutcome::Accepted { .. }
        ));
        let accepted = repository.view_current().unwrap().owner(owner).unwrap();
        let (digest, bytes) = encode_owner(&accepted.value.unwrap()).unwrap();
        assert_eq!(
            victim,
            ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes())
        );
        assert_eq!(&bytes, victim_bytes);

        let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline)
            .unwrap()
            .manifest_digest;
        let location = repository
            .object_store()
            .unwrap()
            .catalog_location(victim)
            .unwrap()
            .unwrap();
        let pack = repository
            .root()
            .join("packs")
            .join(location.pack.file_name());
        let pack_bytes = std::fs::read(&pack).unwrap();
        let metadata = PackMetadata::decode(&pack_bytes, true).unwrap();
        assert_eq!(
            metadata.entries.len(),
            1,
            "the missing pack must hide only the victim"
        );
        assert_eq!(metadata.entries[0].key, victim);
        assert_eq!(metadata.entries[0].offset, location.offset);
        assert!(metadata.entries[0].encoded_length > 0);
        let payload_offset = usize::try_from(location.offset).unwrap();
        let absent_pack = directory.path().join("temporarily-absent.lkjp");
        Self {
            _directory: directory,
            repository,
            compilation,
            victim,
            pack,
            absent_pack,
            pack_bytes,
            payload_offset,
        }
    }

    fn inject(&self, fault: Fault) {
        match fault {
            Fault::Corrupt => {
                let mut bytes = self.pack_bytes.clone();
                bytes[self.payload_offset] ^= 0x80;
                std::fs::write(&self.pack, bytes).unwrap();
            }
            Fault::Missing => std::fs::rename(&self.pack, &self.absent_pack).unwrap(),
        }
    }

    fn restore(&self, fault: Fault) {
        match fault {
            Fault::Corrupt => std::fs::write(&self.pack, &self.pack_bytes).unwrap(),
            Fault::Missing => std::fs::rename(&self.absent_pack, &self.pack).unwrap(),
        }
    }
}

#[derive(Clone, Copy)]
enum Fault {
    Corrupt,
    Missing,
}

fn projection_fault_is_rechecked(fault: Fault) {
    let fixture = Fixture::new();
    let view = fixture.repository.view_current().unwrap();
    let head = std::fs::read(fixture.repository.root().join("HEAD")).unwrap();
    let healthy = super::super::link::link_artifact_with_view(
        &fixture.repository,
        &view,
        fixture.compilation,
        &[],
    )
    .unwrap();
    let admitted = load_artifact(&healthy.artifact.bytes).unwrap();
    assert!(
        !admitted.objects.contains_key(&fixture.victim),
        "the unused module must not enter the runtime or reference artifact closure"
    );

    // Discriminating control: with the projection deliberately outside the scope,
    // link_prepared does not capture this object. Its final reread must succeed
    // despite the isolated fault, proving another captured key cannot mask it.
    let (cached, exported) = super::super::cache::load_exact_current_compilation_with_view(
        &fixture.repository,
        &view,
        fixture.compilation,
    )
    .unwrap();
    let mut control_link_completed = false;
    let control = view.with_link_read_reuse(|| {
        let store = fixture.repository.object_store()?;
        let linked = super::super::link::link_prepared(&view, &store, &cached, &exported, &[])?;
        assert_eq!(linked.artifact.bytes, healthy.artifact.bytes);
        fixture.inject(fault);
        control_link_completed = true;
        Ok(linked)
    });
    if control_link_completed {
        fixture.restore(fault);
    }
    let (control, _, control_observation) =
        control.expect("only package projection reads the isolated canonical victim");
    assert!(control_link_completed);
    assert!(control_observation.rechecked_objects > 0);
    assert_eq!(control.artifact.bytes, healthy.artifact.bytes);

    // The real preparation order captures the projection before using its exact
    // checked result in link_prepared. The fault is injected only after that link
    // has successfully produced the same strictly admitted artifact.
    let mut link_completed = false;
    let result = view.with_link_read_reuse(|| {
        let (cached, exported) = super::super::cache::load_exact_current_compilation_with_view(
            &fixture.repository,
            &view,
            fixture.compilation,
        )?;
        let store = fixture.repository.object_store()?;
        let linked = super::super::link::link_prepared(&view, &store, &cached, &exported, &[])?;
        assert_eq!(linked.artifact.bytes, healthy.artifact.bytes);
        fixture.inject(fault);
        link_completed = true;
        Ok(linked)
    });
    if link_completed {
        fixture.restore(fault);
    }
    let error = result.expect_err("final physical reread must reject the projection-only fault");
    assert!(
        link_completed,
        "failure must follow successful link_prepared"
    );
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(
        error.code,
        match fault {
            Fault::Corrupt => "pack_entry_checksum",
            Fault::Missing => "pack_file_missing",
        }
    );
    assert_eq!(
        std::fs::read(fixture.repository.root().join("HEAD")).unwrap(),
        head
    );

    // A new scope on the same admitted view detects leaked recorder/reuse state.
    let retried = super::super::link::link_artifact_with_view(
        &fixture.repository,
        &view,
        fixture.compilation,
        &[],
    )
    .expect("physical refusal must leave the same view reusable after restoration");
    assert_eq!(retried.artifact.bytes, healthy.artifact.bytes);
    assert!(retried.work.read_reuse.rechecked_objects > control_observation.rechecked_objects);
}

#[test]
fn link_projection_rereads_corrupt_canonical_object_after_successful_link() {
    projection_fault_is_rechecked(Fault::Corrupt);
}

#[test]
fn link_projection_rereads_missing_isolated_canonical_object_after_successful_link() {
    projection_fault_is_rechecked(Fault::Missing);
}

#[test]
fn link_projection_public_oracle_and_pinned_view_survive_head_advance() {
    let fixture = Fixture::new();
    let closure = fixture.repository.export_package_container().unwrap();
    let package = &closure.packages[&closure.container.root.package_revision];
    let expected = compile_immutable(package, &closure.container.objects, &[]).unwrap();
    let public = link_artifact(&fixture.repository, fixture.compilation, &[]).unwrap();
    assert_eq!(public.artifact.bytes, expected.artifact.bytes);
    load_artifact(&public.artifact.bytes).unwrap();

    let pinned = fixture.repository.view_current().unwrap();
    let original_revision = pinned.revision();
    let later = prepare(&fixture.repository, "create.module as=$later name=later\n");
    assert!(matches!(
        fixture.repository.publish(&later.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert_ne!(
        fixture.repository.current().unwrap().head.revision,
        original_revision
    );
    let linked = super::super::link::link_artifact_with_view(
        &fixture.repository,
        &pinned,
        fixture.compilation,
        &[],
    )
    .expect("exact old compilation remains valid for its still-admitted pinned view");
    assert_eq!(linked.artifact.bytes, expected.artifact.bytes);
    assert!(linked.work.read_reuse.rechecked_objects > 0);
    let error = link_artifact(&fixture.repository, fixture.compilation, &[])
        .expect_err("a new current view must reject the old revision's compilation");
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "compilation_manifest_revision_binding");
}
