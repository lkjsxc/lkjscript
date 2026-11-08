//! Paired host-link measurement. Root orchestration binds complete files/source/Cargo images.
#![cfg(target_os = "linux")]

use crate::platform::compiler::{
    ArtifactLinkWork, CompilationManifest, gc_manifest_from_bytes, link_artifact_with_view,
};
use crate::platform::publication::GraphRepository;
use crate::platform::storage::object::{ImmutableObjectStore, ObjectDomain, StoreWork};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

type Check<T> = Result<T, String>;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    repository: String,
    package: String,
    revision: String,
    semantic_state: String,
    owners: u64,
    retirements: u64,
    compiler_units: u64,
    compilation_manifest: String,
    artifact_sha256: String,
    artifact_bytes: u64,
    artifact_manifest: String,
    artifact_bundle: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: String,
    stage: String,
    fixture: PathBuf,
    output: PathBuf,
    fixture_head_sha256: String,
    cache_head_sha256: String,
    expected: Expected,
}

fn require(ok: bool, why: &str) -> Check<()> {
    if ok { Ok(()) } else { Err(why.to_owned()) }
}

fn sha(bytes: &[u8]) -> String {
    crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes))
}

fn proc_value(path: &str) -> (Option<String>, Option<String>) {
    match fs::read_to_string(path) {
        Ok(value) => (Some(value), None),
        Err(error) => (None, Some(format!("{path}: {error}"))),
    }
}

fn observation() -> Value {
    let (status, status_error) = proc_value("/proc/self/status");
    let (io, io_error) = proc_value("/proc/self/io");
    let (stat, stat_error) = proc_value("/proc/self/stat");
    let field = |text: Option<&str>, key: &str| -> Option<String> {
        text?.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            (name == key)
                .then(|| value.split_whitespace().next().map(str::to_owned))
                .flatten()
        })
    };
    let columns = stat
        .as_deref()
        .and_then(|value| value.rsplit_once(") "))
        .map(|(_, tail)| tail.split_whitespace().collect::<Vec<_>>());
    let errors = [status_error, io_error, stat_error]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    json!({
        "pid": std::process::id(),
        "VmPeak_kib": field(status.as_deref(), "VmPeak"),
        "VmHWM_kib": field(status.as_deref(), "VmHWM"),
        "VmRSS_kib": field(status.as_deref(), "VmRSS"),
        "rchar": field(io.as_deref(), "rchar"),
        "syscr": field(io.as_deref(), "syscr"),
        "read_bytes": field(io.as_deref(), "read_bytes"),
        "wchar": field(io.as_deref(), "wchar"),
        "syscw": field(io.as_deref(), "syscw"),
        "write_bytes": field(io.as_deref(), "write_bytes"),
        "user_ticks": columns.as_ref().and_then(|v| v.get(11)).copied(),
        "system_ticks": columns.as_ref().and_then(|v| v.get(12)).copied(),
        "start_ticks": columns.as_ref().and_then(|v| v.get(19)).copied(),
        "errors": errors,
        "memory_scope": "OS process-lifetime VmPeak/VmHWM; VmRSS is current at observation; none is phase-local",
        "counter_scope": "process-wide Linux counters; snapshot interval includes small observer overhead",
        "counter_saturation": "Linux does not supply a saturation flag; checked nonnegative deltas only"
    })
}

fn deltas(before: &Value, after: &Value) -> Value {
    let mut values = serde_json::Map::new();
    for key in [
        "rchar",
        "syscr",
        "read_bytes",
        "wchar",
        "syscw",
        "write_bytes",
        "user_ticks",
        "system_ticks",
    ] {
        let parsed = |value: &Value| value[key].as_str().and_then(|s| s.parse::<u64>().ok());
        let delta = parsed(before)
            .zip(parsed(after))
            .and_then(|(old, new)| new.checked_sub(old))
            .map(|value| value.to_string());
        values.insert(key.to_owned(), json!(delta));
    }
    Value::Object(values)
}

fn measured<T>(name: &str, operation: impl FnOnce() -> Check<T>) -> Check<(T, Value)> {
    let before = observation();
    let started = Instant::now();
    let result = operation();
    let elapsed = started.elapsed().as_nanos().to_string();
    let after = observation();
    let delta = deltas(&before, &after);
    let record = json!({
        "event": "phase", "name": name,
        "status": if result.is_ok() { "passed" } else { "failed" },
        "elapsed_nanoseconds": elapsed,
        "wall_scope": "only the named call; excludes snapshots, external input/artifact hash checks and output publication",
        "process_before": before, "process_after": after,
        "process_delta": delta,
        "error": result.as_ref().err(),
        "candidate_accepted": false
    });
    eprintln!("LINK_PROJECTION_MEASUREMENT {record}");
    result.map(|value| (value, record))
}

fn work_observation(work: &ArtifactLinkWork) -> Value {
    let repository = work.repository;
    let reuse = work.read_reuse;
    json!({
        "repository": {
            "canonical_records_decoded": repository.canonical_records_decoded.to_string(),
            "witness_records_decoded": repository.witness_records_decoded.to_string(),
            "items_returned": repository.items_returned.to_string(),
            "catalog_lookups": repository.store.catalog_lookups.to_string(),
            "objects_read": repository.store.objects_read.to_string(),
            "objects_reused": repository.store.objects_reused.to_string(),
            "bytes_read": repository.store.bytes_read.to_string(),
            "pages_read": repository.map.pages_read.to_string(),
            "entries_visited": repository.map.entries_visited.to_string(),
            "store_saturated": repository.store.saturated,
            "map_saturated": repository.map.saturated,
            "complete_debug": format!("{repository:?}")
        },
        "read_reuse": {
            "rechecked_objects": reuse.rechecked_objects.to_string(),
            "work_steps": reuse.work_steps.to_string(),
            "copy_work": reuse.copy_work.to_string(),
            "peak_modeled_retained_bytes": reuse.peak_modeled_retained_bytes.to_string(),
            "saturated": reuse.saturated,
            "complete_debug": format!("{reuse:?}")
        },
        "compiler_units": work.compiler_units.to_string(),
        "reference_owners": work.reference_owners.to_string(),
        "output_bytes": work.output_bytes.to_string(),
        "complete_debug": format!("{work:?}"),
        "scope": "modeled linker work, not complete process I/O; BEFORE omits its discarded first projection"
    })
}

fn write_new(path: &Path, bytes: &[u8]) -> Check<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

#[test]
#[ignore = "explicit paired disk-link measurement; no build fallback, selfhost generations or runtime execution"]
fn artifact_link_projection_reuse_measurement() -> Check<()> {
    let config_path = PathBuf::from(
        std::env::var_os("LKJ_LINK_MEASUREMENT_CONFIG")
            .ok_or("LKJ_LINK_MEASUREMENT_CONFIG is required")?,
    );
    let config_bytes = fs::read(&config_path).map_err(|error| error.to_string())?;
    let config: Config =
        serde_json::from_slice(&config_bytes).map_err(|error| error.to_string())?;
    require(
        config.schema == "lkjscript.link-projection-measurement-input.v1",
        "wrong configuration schema",
    )?;
    require(
        matches!(config.stage.as_str(), "before" | "after"),
        "stage must be before or after",
    )?;
    require(
        config.fixture.is_absolute() && config.output.is_absolute(),
        "absolute fixture/output required",
    )?;
    require(
        !config.output.starts_with(&config.fixture),
        "output must not modify fixture",
    )?;
    fs::create_dir(&config.output).map_err(|error| format!("new output directory: {error}"))?;
    let head = fs::read(config.fixture.join("HEAD")).map_err(|error| error.to_string())?;
    let cache = fs::read(config.fixture.join("derived/compiler/CURRENT"))
        .map_err(|error| error.to_string())?;
    require(
        sha(&head) == config.fixture_head_sha256 && sha(&cache) == config.cache_head_sha256,
        "selected HEAD/CURRENT bytes differ",
    )?;
    // CURRENT is decoded for its exact physical selection, not trusted as current validation.
    let digest = gc_manifest_from_bytes(&cache).map_err(|error| format!("{error:?}"))?;
    require(
        digest.to_string() == config.expected.compilation_manifest,
        "selected manifest differs",
    )?;
    let (repository, open_phase) = measured("repository_open", || {
        GraphRepository::open(&config.fixture).map_err(|error| format!("{error:?}"))
    })?;
    let (view, admission_phase) = measured("fresh_view_current_validation", || {
        repository
            .view_current()
            .map_err(|error| format!("{error:?}"))
    })?;
    require(
        view.current().head.repository_id.to_string() == config.expected.repository
            && view.package().to_string() == config.expected.package
            && view.revision().to_string() == config.expected.revision
            && view.current().accepted.semantic_state.to_string() == config.expected.semantic_state,
        "accepted semantic input differs from selected fixture",
    )?;
    require(
        view.current().semantic_root.owners.entries() == config.expected.owners
            && view.current().semantic_root.retirements.entries() == config.expected.retirements
            && view.current().semantic_root.dependencies.entries() == 0,
        "unexpected inventory; this probe requires a dependency-free fixture",
    )?;
    // The public link wrapper's existing fresh-view and link steps, measured separately.
    // This interval includes canonical re-lowering, final rechecks and encode_artifact's strict load.
    let (linked, link_phase) = measured("link_artifact_with_view", || {
        link_artifact_with_view(&repository, &view, digest, &[])
            .map_err(|error| format!("{error:?}"))
    })?;
    let work = work_observation(&linked.work);
    // Rebind the encoder's already strict-loaded result; this is not a second strict load.
    let ((bytes, loaded), rebind_phase) = measured("consume_retained_strict_admission", || {
        linked
            .artifact
            .into_admitted()
            .map_err(|error| format!("{error:?}"))
    })?;
    require(
        u64::try_from(bytes.len()).ok() == Some(config.expected.artifact_bytes)
            && sha(&bytes) == config.expected.artifact_sha256,
        "artifact bytes/digest differ",
    )?;
    require(
        loaded.manifest_digest.to_string() == config.expected.artifact_manifest
            && loaded.bundle_digest.to_string() == config.expected.artifact_bundle,
        "strict artifact identities differ",
    )?;
    let mut read_work = StoreWork::default();
    let manifest_bytes = repository
        .object_store()
        .map_err(|error| format!("{error:?}"))?
        .read(
            digest.object_key(),
            ObjectDomain::CompilationManifest.maximum_bytes(),
            &mut read_work,
        )
        .map_err(|error| format!("{error:?}"))?
        .ok_or("manifest absent after linking")?;
    let manifest = CompilationManifest::decode(&manifest_bytes, digest)
        .map_err(|error| format!("{error:?}"))?;
    require(
        manifest.units.entries() == config.expected.compiler_units,
        "manifest unit inventory differs",
    )?;
    let package = loaded.root_package().ok_or("strict root package missing")?;
    let semantic = json!({
        "repository": view.current().head.repository_id.to_string(), "package": view.package().to_string(),
        "revision": view.revision().to_string(), "semantic_state": view.current().accepted.semantic_state.to_string(),
        "package_revision": package.package_revision.to_string(), "interface": package.interface.to_string(),
        "compilation_manifest": digest.to_string(), "manifest_object_sha256": sha(&manifest_bytes),
        "compiler_units": manifest.units.entries().to_string()
    });
    let load_work = format!("{:?}", loaded.work);
    let artifact_path = config.output.join("artifact.lkja");
    let publication = crate::platform::owned_output::publish_create_new(
        &artifact_path,
        &bytes,
        None,
        "paired host-link measurement artifact",
    )
    .map_err(|error| format!("{error:?}"))?;
    let artifact = json!({
        "path": artifact_path, "bytes": publication.bytes.to_string(),
        "sha256": sha(&bytes), "manifest": loaded.manifest_digest.to_string(),
        "bundle": loaded.bundle_digest.to_string()
    });
    let (_, cleanup_phase) = measured("drop_owned_result_view_repository", || {
        drop((loaded, bytes, view, repository, manifest, manifest_bytes));
        Ok(())
    })?;
    require(
        fs::read(config.fixture.join("HEAD")).map_err(|error| error.to_string())? == head
            && fs::read(config.fixture.join("derived/compiler/CURRENT"))
                .map_err(|error| error.to_string())?
                == cache,
        "HEAD/CURRENT changed during measurement",
    )?;
    require(
        fs::read(&config_path).map_err(|error| error.to_string())? == config_bytes,
        "configuration changed",
    )?;
    let report = json!({
        "schema": "lkjscript.link-projection-measurement.v1", "status": "passed",
        "candidate_accepted": false, "stage": config.stage,
        "config": {"path": config_path, "sha256": sha(&config_bytes)},
        "executable": std::env::current_exe().map_err(|error| error.to_string())?,
        "fixture": config.fixture, "expected": config.expected, "semantic": semantic,
        "head_sha256": sha(&head), "cache_head_sha256": sha(&cache),
        "head_and_cache_bytes_unchanged": true, "artifact": artifact,
        "phases": [open_phase, admission_phase, link_phase, rebind_phase, cleanup_phase],
        "link_work": work, "artifact_load_work_debug": load_work,
        "scope": "one real revision-pinned link including strict loading; no new build, source export, selfhost generation, M3, or runtime execution",
        "external_obligations": "root driver binds complete source/Cargo/image/input files before and after both arms, and compares complete artifact bytes",
        "memory_scope": "VmPeak/VmHWM are process-lifetime; VmRSS is current; cleanup drops owned state, not process caches"
    });
    let mut report_bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    report_bytes.push(b'\n');
    let receipt = config.output.join("receipt.json");
    write_new(&receipt, &report_bytes)?;
    eprintln!(
        "LINK_PROJECTION_MEASUREMENT {}",
        json!({
            "event": "completed", "receipt": receipt, "sha256": sha(&report_bytes)
        })
    );
    Ok(())
}
