use super::model::Gate;
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof, VerificationDigest};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MAXIMUM_UNITS: usize = 256;
const MAXIMUM_FILES_PER_UNIT: usize = 64;
const MAXIMUM_FILES: usize = 4_096;
const MAXIMUM_FEATURES: usize = 256;
const MAXIMUM_MANIFESTS: usize = 64;
const MAXIMUM_RECORDS: usize = 16_384;
const MAXIMUM_RECORD_BYTES: usize = 1024 * 1024;
const MAXIMUM_UNIT_METADATA_BYTES: usize = 256 * 1024;
const MAXIMUM_METADATA_BYTES: usize = 8 * 1024 * 1024;
const MAXIMUM_JSON_DEPTH: usize = 32;
const MAXIMUM_JSON_NODES: usize = 4_096;
const MAXIMUM_STRING_BYTES: usize = 8 * 1024;
// Origin artifacts are streamed separately from bounded JSON evidence. These are
// preparation limits, not measured adequacy claims or increases to evidence caps.
pub(super) const MAXIMUM_ORIGIN_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub(super) const MAXIMUM_ORIGIN_INVENTORY_BYTES: u64 = 8 * 1024 * 1024 * 1024;

// Source/context admission and original-producer authority belong to the ledger.
// These records preserve every selected member unit, including libraries and tests.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Unit {
    pub(super) package_id: String,
    pub(super) manifest_path: String,
    pub(super) target: Value,
    pub(super) profile: Value,
    pub(super) features: Vec<String>,
    pub(super) filenames: Vec<FileProof>,
    pub(super) retained: Vec<FileProof>,
    pub(super) fresh: bool,
}

pub(super) fn read_units(
    repository: &Path,
    target: &Path,
    workspace_manifests: &[String],
    gate: &Gate,
    stdout_path: &Path,
    stdout: &FileProof,
) -> Result<Vec<Unit>, DevError> {
    canonical_directory(repository, None)?;
    canonical_directory(target, None)?;
    let manifests = manifest_set(repository, workspace_manifests, true)?;
    let bytes = producer_bytes(gate, stdout_path, stdout)?;
    let mut artifact_bytes = 0_u64;
    parse_units(
        repository,
        target,
        &manifests,
        gate,
        &bytes,
        true,
        &mut |path| {
            canonical_regular(path, Some(target))?;
            let length = artifact_length(path)?;
            charge_artifact_bytes(&mut artifact_bytes, length)?;
            let proof = evidence::proof(path, path_string(path)?)?;
            regular_proof(&proof)?;
            if proof.bytes != Some(length) {
                return Err(refusal(
                    "workspace artifact length changed during observation",
                ));
            }
            Ok(proof)
        },
    )
}

pub(super) fn verify_records(
    repository: &Path,
    target: &Path,
    workspace_manifests: &[String],
    gate: &Gate,
    stdout_path: &Path,
    stdout: &FileProof,
    units: &[Unit],
) -> Result<(), DevError> {
    safe_absolute(repository)?;
    safe_absolute(target)?;
    if units.is_empty() || units.len() > MAXIMUM_UNITS {
        return Err(refusal("invalid recorded workspace unit count"));
    }
    validate_inventory_bytes(units)?;
    let manifests = manifest_set(repository, workspace_manifests, false)?;
    let bytes = producer_bytes(gate, stdout_path, stdout)?;
    let mut proofs = BTreeMap::new();
    let mut file_count = 0_usize;
    let mut artifact_bytes = 0_u64;
    for unit in units {
        if unit.retained.len() != unit.filenames.len() {
            return Err(refusal("incomplete retained workspace artifact inventory"));
        }
        for (original, retained) in unit.filenames.iter().zip(&unit.retained) {
            let (_, length) = retained_artifact_length(repository, original, retained)?;
            charge_artifact_bytes(&mut artifact_bytes, length)?;
        }
    }
    for unit in units {
        verify_retained(repository, unit)?;
        file_count = checked_total(
            file_count,
            unit.filenames.len(),
            MAXIMUM_FILES,
            "recorded workspace files",
        )?;
        for proof in &unit.filenames {
            lexical_within(Path::new(&proof.path), target)?;
            if let Some(previous) = proofs.get(&proof.path)
                && previous != proof
            {
                return Err(refusal("recorded units disagree about a shared artifact"));
            }
            proofs.insert(proof.path.clone(), proof.clone());
        }
    }
    let observed = parse_units(
        repository,
        target,
        &manifests,
        gate,
        &bytes,
        false,
        &mut |path| {
            proofs
                .get(&path_string(path)?)
                .cloned()
                .ok_or_else(|| refusal("Cargo record names an unrecorded workspace artifact"))
        },
    )?;
    let expected: Vec<_> = units
        .iter()
        .cloned()
        .map(|mut unit| {
            unit.retained.clear();
            unit
        })
        .collect();
    if observed != expected {
        return Err(refusal(
            "Cargo records do not match the exact recorded workspace inventory",
        ));
    }
    Ok(())
}

fn producer_bytes(
    gate: &Gate,
    stdout_path: &Path,
    stdout: &FileProof,
) -> Result<Vec<u8>, DevError> {
    if gate.command.first().map(String::as_str) != Some("cargo")
        || !matches!(
            gate.command.get(1).map(String::as_str),
            Some("build" | "test")
        )
        || !gate
            .command
            .iter()
            .take_while(|argument| *argument != "--")
            .any(|argument| argument == "--message-format=json")
    {
        return Err(refusal("unit producer requires Cargo build/test JSON"));
    }
    canonical_regular(stdout_path, None)?;
    let expected_bytes = stdout
        .bytes
        .ok_or_else(|| refusal("missing producer stdout length"))?;
    if stdout.kind != FileKind::File
        || stdout.link_target.is_some()
        || stdout.digest.is_none()
        || expected_bytes > gate.maximum_stdout_bytes
    {
        return Err(refusal("unsafe or oversized producer stdout proof"));
    }
    let bytes = super::cache::read_bounded(stdout_path, expected_bytes)?;
    if bytes.len() as u64 != expected_bytes
        || stdout.digest.as_ref() != Some(&VerificationDigest::of(&bytes))
        || evidence::proof(stdout_path, stdout.path.clone())? != *stdout
    {
        return Err(refusal("producer stdout changed after process observation"));
    }
    Ok(bytes)
}

fn parse_units(
    repository: &Path,
    target: &Path,
    manifests: &BTreeSet<String>,
    gate: &Gate,
    bytes: &[u8],
    live: bool,
    file_proof: &mut dyn FnMut(&Path) -> Result<FileProof, DevError>,
) -> Result<Vec<Unit>, DevError> {
    let mut units = Vec::new();
    let mut keys = BTreeSet::new();
    let mut package_manifests = BTreeMap::new();
    let mut manifest_packages = BTreeMap::new();
    let mut listed_files = BTreeMap::new();
    let mut build_scripts = BTreeSet::new();
    let mut records = 0_usize;
    let mut file_count = 0_usize;
    let mut metadata_bytes = 0_usize;
    for line in bytes.split(|byte| *byte == b'\n') {
        // Cargo permits ordinary program stdout; only its JSON records are parsed.
        if line.first() != Some(&b'{') {
            continue;
        }
        records = checked_total(records, 1, MAXIMUM_RECORDS, "Cargo records")?;
        if line.len() > MAXIMUM_RECORD_BYTES {
            return Err(refusal("Cargo record exceeds the metadata limit"));
        }
        let UniqueValue(message) = serde_json::from_slice(line)
            .map_err(|error| refusal(format!("malformed Cargo record: {error}")))?;
        match message.get("reason").and_then(Value::as_str) {
            Some("compiler-artifact") => {
                let manifest = string_field(&message, "manifest_path")?;
                safe_absolute(Path::new(manifest))?;
                let package = string_field(&message, "package_id")?;
                if let Some(previous) = package_manifests.get(package)
                    && previous != manifest
                {
                    return Err(refusal("package identity crosses Cargo manifests"));
                }
                package_manifests.insert(package.to_owned(), manifest.to_owned());
                if !manifests.contains(manifest) {
                    continue;
                }
                if let Some(previous) = manifest_packages.get(manifest)
                    && previous != package
                {
                    return Err(refusal("workspace manifest has ambiguous package identity"));
                }
                manifest_packages.insert(manifest.to_owned(), package.to_owned());
                if units.len() >= MAXIMUM_UNITS {
                    return Err(refusal("workspace unit inventory exceeds its limit"));
                }
                metadata_bytes = checked_total(
                    metadata_bytes,
                    line.len(),
                    MAXIMUM_METADATA_BYTES,
                    "workspace metadata",
                )?;
                if line.len() > MAXIMUM_UNIT_METADATA_BYTES {
                    return Err(refusal("workspace unit metadata exceeds its limit"));
                }
                let unit = artifact_unit(repository, target, &message, live, file_proof)?;
                file_count = checked_total(
                    file_count,
                    unit.filenames.len(),
                    MAXIMUM_FILES,
                    "workspace files",
                )?;
                let key = unit_key(&unit)?;
                if !keys.insert(key.as_str().to_owned()) {
                    return Err(refusal("duplicate exact workspace unit"));
                }
                for proof in &unit.filenames {
                    if let Some(previous) = listed_files.get(&proof.path)
                        && previous != proof
                    {
                        return Err(refusal("workspace units disagree about a shared artifact"));
                    }
                    listed_files.insert(proof.path.clone(), proof.clone());
                }
                units.push(unit);
            }
            Some("build-script-executed") => {
                build_scripts.insert(string_field(&message, "package_id")?.to_owned());
            }
            Some("build-finished") => {
                if message.get("success").and_then(Value::as_bool) != Some(true) || units.is_empty()
                {
                    return Err(refusal(
                        "Cargo did not complete a workspace artifact inventory",
                    ));
                }
                if units
                    .iter()
                    .any(|unit| build_scripts.contains(&unit.package_id))
                {
                    return Err(refusal("workspace build-script outputs are unsupported"));
                }
                for output in &gate.required_outputs {
                    lexical_within(output, target)?;
                    let label = path_string(output)?;
                    if !listed_files.contains_key(&label) {
                        return Err(refusal(
                            "declared output is missing from workspace inventory",
                        ));
                    }
                }
                // Later test/program output cannot provide a missing build artifact.
                return Ok(units);
            }
            _ => {}
        }
    }
    Err(refusal("missing Cargo build-finished record"))
}

pub(super) fn retain_units(
    repository: &Path,
    run_directory: &Path,
    gate: &Gate,
    units: &mut [Unit],
) -> Result<(), DevError> {
    canonical_directory(repository, None)?;
    canonical_directory(run_directory, Some(repository))?;
    component_name(&gate.name)?;
    if units.is_empty() || units.len() > MAXIMUM_UNITS {
        return Err(refusal("invalid workspace unit retention count"));
    }
    validate_inventory_bytes(units)?;
    let root = create_directory(run_directory, "retained-cargo-origin")?;
    let gate_directory = create_directory(&root, &gate.name)?;
    let mut file_count = 0_usize;
    let mut observed_bytes = 0_u64;
    let mut copied_bytes = 0_u64;
    let mut retained_bytes = 0_u64;
    for (unit_index, unit) in units.iter_mut().enumerate() {
        unit_key(unit)?;
        if !unit.retained.is_empty() {
            return Err(refusal(
                "workspace unit already has retained artifact evidence",
            ));
        }
        file_count = checked_total(
            file_count,
            unit.filenames.len(),
            MAXIMUM_FILES,
            "retained workspace files",
        )?;
        let directory = create_directory(&gate_directory, &unit_index.to_string())?;
        let mut retained = Vec::with_capacity(unit.filenames.len());
        for (file_index, original) in unit.filenames.iter().enumerate() {
            let source = Path::new(&original.path);
            canonical_regular(source, None)?;
            let expected = proof_artifact_length(original)?;
            let length = artifact_length(source)?;
            if length != expected {
                return Err(refusal(
                    "workspace artifact length changed before retention",
                ));
            }
            charge_artifact_bytes(&mut observed_bytes, length)?;
            if evidence::proof(source, original.path.clone())? != *original {
                return Err(refusal("workspace artifact changed before retention"));
            }
            let path = directory.join(file_index.to_string());
            let mut input = File::open(source)?;
            let input_metadata = input.metadata()?;
            if !input_metadata.is_file() || input_metadata.len() != expected {
                return Err(refusal("workspace artifact changed before copying"));
            }
            charge_artifact_bytes(&mut copied_bytes, input_metadata.len())?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            let maximum = expected
                .checked_add(1)
                .ok_or_else(|| refusal("workspace artifact length overflow"))?;
            let copied = std::io::copy(&mut input.by_ref().take(maximum), &mut output)?;
            if copied != expected {
                return Err(refusal("workspace artifact changed while copying"));
            }
            output.set_permissions(input.metadata()?.permissions())?;
            output.sync_all()?;
            drop(output);
            canonical_regular(&path, Some(repository))?;
            let retained_length = artifact_length(&path)?;
            if retained_length != expected {
                return Err(refusal(
                    "retained workspace artifact has a different length",
                ));
            }
            charge_artifact_bytes(&mut retained_bytes, retained_length)?;
            let proof = evidence::proof(&path, retained_label(repository, &path)?)?;
            if !same_artifact(&proof, original) {
                return Err(refusal(
                    "workspace artifact changed while retaining evidence",
                ));
            }
            retained.push(proof);
        }
        File::open(&directory)?.sync_all()?;
        unit.retained = retained;
    }
    File::open(&gate_directory)?.sync_all()?;
    File::open(&root)?.sync_all()?;
    File::open(run_directory)?.sync_all()?;
    Ok(())
}

pub(super) fn unit_key(unit: &Unit) -> Result<VerificationDigest, DevError> {
    validate_identity(unit)?;
    #[derive(Serialize)]
    struct Identity<'a> {
        domain: &'static str,
        package_id: &'a str,
        manifest_path: &'a str,
        target: Value,
        profile: Value,
        features: &'a [String],
        filenames: Vec<&'a str>,
    }
    let bytes = serde_json::to_vec(&Identity {
        domain: "lkjscript.dev.cargo-origin-unit.v1",
        package_id: &unit.package_id,
        manifest_path: &unit.manifest_path,
        target: canonical_value(&unit.target)?,
        profile: canonical_value(&unit.profile)?,
        features: &unit.features,
        filenames: unit
            .filenames
            .iter()
            .map(|proof| proof.path.as_str())
            .collect(),
    })?;
    if bytes.len() > MAXIMUM_UNIT_METADATA_BYTES {
        return Err(refusal("workspace unit key exceeds its metadata limit"));
    }
    Ok(VerificationDigest::of(&bytes))
}

pub(super) fn verify_retained(repository: &Path, unit: &Unit) -> Result<(), DevError> {
    validate_inventory_bytes(std::slice::from_ref(unit))?;
    canonical_directory(repository, None)?;
    unit_key(unit)?;
    if unit.retained.len() != unit.filenames.len() {
        return Err(refusal("incomplete retained workspace artifact inventory"));
    }
    let mut paths = BTreeSet::new();
    let mut artifact_bytes = 0_u64;
    for (original, retained) in unit.filenames.iter().zip(&unit.retained) {
        let (path, length) = retained_artifact_length(repository, original, retained)?;
        if !paths.insert(path.clone()) {
            return Err(refusal("duplicate retained workspace artifact path"));
        }
        charge_artifact_bytes(&mut artifact_bytes, length)?;
        if evidence::proof(&path, retained.path.clone())? != *retained
            || !same_artifact(retained, original)
        {
            return Err(refusal(
                "retained workspace artifact proof does not match its original",
            ));
        }
    }
    Ok(())
}

fn retained_artifact_length(
    repository: &Path,
    original: &FileProof,
    retained: &FileProof,
) -> Result<(PathBuf, u64), DevError> {
    regular_proof(retained)?;
    let path = retained_path(repository, &retained.path)?;
    canonical_regular(&path, Some(repository))?;
    let length = artifact_length(&path)?;
    if retained.bytes != Some(length) || original.bytes != Some(length) {
        return Err(refusal(
            "retained workspace artifact length differs from its proof",
        ));
    }
    Ok((path, length))
}

fn artifact_unit(
    repository: &Path,
    target: &Path,
    message: &Value,
    live: bool,
    file_proof: &mut dyn FnMut(&Path) -> Result<FileProof, DevError>,
) -> Result<Unit, DevError> {
    let target_value = message
        .get("target")
        .ok_or_else(|| refusal("missing workspace target identity"))?;
    let profile = message
        .get("profile")
        .ok_or_else(|| refusal("missing workspace profile identity"))?;
    let features = string_array(message.get("features"), MAXIMUM_FEATURES, true)?;
    let names = string_array(message.get("filenames"), MAXIMUM_FILES_PER_UNIT, false)?;
    let mut filenames = Vec::with_capacity(names.len());
    for name in names {
        let path = Path::new(&name);
        lexical_within(path, target)?;
        let proof = file_proof(path)?;
        if proof.path != name {
            return Err(refusal(
                "artifact proof path differs from the Cargo filename",
            ));
        }
        regular_proof(&proof)?;
        filenames.push(proof);
    }
    let unit = Unit {
        package_id: string_field(message, "package_id")?.to_owned(),
        manifest_path: string_field(message, "manifest_path")?.to_owned(),
        target: canonical_value(target_value)?,
        profile: canonical_value(profile)?,
        features,
        filenames,
        retained: Vec::new(),
        fresh: message
            .get("fresh")
            .and_then(Value::as_bool)
            .ok_or_else(|| refusal("missing workspace artifact freshness"))?,
    };
    validate_identity(&unit)?;
    let manifest = Path::new(&unit.manifest_path);
    let source = Path::new(string_field(&unit.target, "src_path")?);
    lexical_within(manifest, repository)?;
    lexical_within(source, repository)?;
    if live {
        canonical_regular(manifest, Some(repository))?;
        canonical_regular(source, Some(repository))?;
    }
    match message.get("executable") {
        Some(Value::Null) => {}
        Some(Value::String(path)) => {
            safe_absolute(Path::new(path))?;
            if !unit.filenames.iter().any(|proof| proof.path == *path) {
                return Err(refusal(
                    "workspace executable is not among its artifact filenames",
                ));
            }
        }
        _ => {
            return Err(refusal(
                "missing or malformed workspace executable identity",
            ));
        }
    }
    Ok(unit)
}

fn validate_identity(unit: &Unit) -> Result<(), DevError> {
    bounded_string(&unit.package_id)?;
    safe_absolute(Path::new(&unit.manifest_path))?;
    if !unit.target.is_object() || !unit.profile.is_object() {
        return Err(refusal(
            "workspace target/profile identity is not an object",
        ));
    }
    string_field(&unit.target, "name")?;
    string_field(&unit.target, "edition")?;
    safe_absolute(Path::new(string_field(&unit.target, "src_path")?))?;
    let kinds = string_array(unit.target.get("kind"), 16, false)?;
    if kinds.iter().any(|kind| kind == "custom-build") {
        return Err(refusal("workspace build-script outputs are unsupported"));
    }
    let crate_types = string_array(unit.target.get("crate_types"), 16, false)?;
    if kinds
        .iter()
        .chain(&crate_types)
        .any(|kind| kind == "proc-macro")
    {
        return Err(refusal(
            "workspace proc-macro generated closures are unsupported",
        ));
    }
    let supported = match (kinds.as_slice(), crate_types.as_slice()) {
        ([kind], [crate_type]) => matches!(
            (kind.as_str(), crate_type.as_str()),
            ("lib", "lib") | ("rlib", "rlib") | ("bin" | "test" | "example" | "bench", "bin")
        ),
        _ => false,
    };
    if !supported {
        return Err(refusal(
            "unsupported workspace target kind/crate-type closure",
        ));
    }
    for name in ["doc", "doctest", "test"] {
        boolean_field(&unit.target, name)?;
    }
    string_field(&unit.profile, "opt_level")?;
    for name in ["debug_assertions", "overflow_checks", "test"] {
        boolean_field(&unit.profile, name)?;
    }
    match unit.profile.get("debuginfo") {
        Some(Value::Null) => {}
        Some(Value::Number(value)) if value.as_u64().is_some() => {}
        Some(Value::String(value)) => bounded_string(value)?,
        _ => return Err(refusal("missing or malformed workspace debuginfo identity")),
    }
    if unit.features.len() > MAXIMUM_FEATURES {
        return Err(refusal("workspace features exceed their limit"));
    }
    let mut features = BTreeSet::new();
    for feature in &unit.features {
        bounded_string(feature)?;
        if !features.insert(feature) {
            return Err(refusal("duplicate workspace feature identity"));
        }
    }
    if unit.filenames.is_empty() || unit.filenames.len() > MAXIMUM_FILES_PER_UNIT {
        return Err(refusal("invalid workspace artifact filename count"));
    }
    let mut paths = BTreeSet::new();
    for proof in &unit.filenames {
        regular_proof(proof)?;
        safe_absolute(Path::new(&proof.path))?;
        if !paths.insert(&proof.path) {
            return Err(refusal("duplicate workspace artifact filename"));
        }
    }
    canonical_value(&unit.target)?;
    canonical_value(&unit.profile)?;
    Ok(())
}

fn canonical_value(value: &Value) -> Result<Value, DevError> {
    fn visit(value: &Value, depth: usize, nodes: &mut usize) -> Result<Value, DevError> {
        if depth > MAXIMUM_JSON_DEPTH {
            return Err(refusal("workspace JSON identity exceeds its depth limit"));
        }
        *nodes = checked_total(*nodes, 1, MAXIMUM_JSON_NODES, "workspace JSON nodes")?;
        match value {
            Value::Object(object) => {
                let mut ordered = BTreeMap::new();
                for (name, value) in object {
                    bounded_string(name)?;
                    ordered.insert(name.clone(), visit(value, depth + 1, nodes)?);
                }
                Ok(Value::Object(ordered.into_iter().collect()))
            }
            Value::Array(values) => values
                .iter()
                .map(|value| visit(value, depth + 1, nodes))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            Value::String(value) => {
                if value.len() > MAXIMUM_STRING_BYTES || value.chars().any(char::is_control) {
                    return Err(refusal("unsafe workspace JSON string"));
                }
                Ok(Value::String(value.clone()))
            }
            _ => Ok(value.clone()),
        }
    }
    visit(value, 0, &mut 0)
}

fn manifest_set(
    repository: &Path,
    labels: &[String],
    live: bool,
) -> Result<BTreeSet<String>, DevError> {
    if labels.is_empty() || labels.len() > MAXIMUM_MANIFESTS {
        return Err(refusal("invalid workspace manifest inventory"));
    }
    let mut manifests = BTreeSet::new();
    for label in labels {
        bounded_string(label)?;
        safe_components(Path::new(label))?;
        let path = repository.join(label);
        lexical_within(&path, repository)?;
        if live {
            canonical_regular(&path, Some(repository))?;
        }
        if path.file_name().and_then(|name| name.to_str()) != Some("Cargo.toml")
            || !manifests.insert(path_string(&path)?)
        {
            return Err(refusal("ambiguous workspace manifest inventory"));
        }
    }
    Ok(manifests)
}

fn regular_proof(proof: &FileProof) -> Result<(), DevError> {
    bounded_string(&proof.path)?;
    if proof.kind != FileKind::File
        || proof.bytes.is_none()
        || proof
            .bytes
            .is_some_and(|bytes| bytes > MAXIMUM_ORIGIN_ARTIFACT_BYTES)
        || proof.digest.is_none()
        || proof.link_target.is_some()
        || proof.mode.is_some_and(|mode| mode & !0o7777 != 0)
    {
        return Err(refusal("incomplete or unsafe workspace regular-file proof"));
    }
    Ok(())
}

fn validate_inventory_bytes(units: &[Unit]) -> Result<(), DevError> {
    if units.is_empty() || units.len() > MAXIMUM_UNITS {
        return Err(refusal("invalid workspace artifact inventory count"));
    }
    let mut files = 0_usize;
    let mut bytes = 0_u64;
    for unit in units {
        unit_key(unit)?;
        files = checked_total(
            files,
            unit.filenames.len(),
            MAXIMUM_FILES,
            "workspace artifact files",
        )?;
        for proof in &unit.filenames {
            // Each listed occurrence is copied/hashed separately, including aliases.
            charge_artifact_bytes(&mut bytes, proof_artifact_length(proof)?)?;
        }
    }
    Ok(())
}

fn proof_artifact_length(proof: &FileProof) -> Result<u64, DevError> {
    regular_proof(proof)?;
    proof
        .bytes
        .ok_or_else(|| refusal("missing workspace artifact length"))
}

fn artifact_length(path: &Path) -> Result<u64, DevError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAXIMUM_ORIGIN_ARTIFACT_BYTES
    {
        return Err(refusal(
            "workspace artifact is unsafe or exceeds its byte limit",
        ));
    }
    Ok(metadata.len())
}

fn charge_artifact_bytes(total: &mut u64, addition: u64) -> Result<(), DevError> {
    if addition > MAXIMUM_ORIGIN_ARTIFACT_BYTES {
        return Err(refusal("workspace artifact exceeds its byte limit"));
    }
    let next = total
        .checked_add(addition)
        .ok_or_else(|| refusal("workspace artifact byte count overflow"))?;
    if next > MAXIMUM_ORIGIN_INVENTORY_BYTES {
        return Err(refusal(
            "workspace artifact inventory exceeds its byte limit",
        ));
    }
    *total = next;
    Ok(())
}

fn same_artifact(left: &FileProof, right: &FileProof) -> bool {
    left.kind == right.kind
        && left.mode == right.mode
        && left.bytes == right.bytes
        && left.digest == right.digest
        && left.link_target == right.link_target
}

fn canonical_regular(path: &Path, root: Option<&Path>) -> Result<(), DevError> {
    canonical_path(path, root, false)
}

fn canonical_directory(path: &Path, root: Option<&Path>) -> Result<(), DevError> {
    canonical_path(path, root, true)
}

fn canonical_path(path: &Path, root: Option<&Path>, directory: bool) -> Result<(), DevError> {
    safe_absolute(path)?;
    if root.is_some_and(|root| !path.starts_with(root))
        || path_string(&fs::canonicalize(path)?)? != path_string(path)?
    {
        return Err(refusal(
            "workspace proof path is outside its root or is not canonical",
        ));
    }
    let mut prefix = PathBuf::new();
    for component in path.components() {
        prefix.push(component.as_os_str());
        if fs::symlink_metadata(&prefix)?.file_type().is_symlink() {
            return Err(refusal("workspace proof path traverses a symlink"));
        }
    }
    let metadata = fs::symlink_metadata(path)?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(refusal("workspace proof path has the wrong file kind"));
    }
    Ok(())
}

fn safe_absolute(path: &Path) -> Result<(), DevError> {
    if !path.is_absolute() {
        return Err(refusal("workspace identity path is not absolute"));
    }
    safe_components(path)
}

fn lexical_within(path: &Path, root: &Path) -> Result<(), DevError> {
    safe_absolute(path)?;
    safe_absolute(root)?;
    if !path.starts_with(root) || path == root {
        return Err(refusal(
            "workspace identity path is outside its recorded root",
        ));
    }
    Ok(())
}

fn safe_components(path: &Path) -> Result<(), DevError> {
    let label = path_string(path)?;
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        || label
            .split(std::path::MAIN_SEPARATOR)
            .any(|part| part == "." || part == "..")
    {
        return Err(refusal(
            "workspace identity path contains traversal components",
        ));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        normalized.push(component.as_os_str());
    }
    if normalized.as_os_str() != path.as_os_str() {
        return Err(refusal(
            "workspace identity path is not lexically canonical",
        ));
    }
    Ok(())
}

fn path_string(path: &Path) -> Result<String, DevError> {
    let label = path
        .to_str()
        .ok_or_else(|| refusal("workspace identity path is not portable UTF-8"))?;
    bounded_string(label)?;
    Ok(label.to_owned())
}

fn retained_label(repository: &Path, path: &Path) -> Result<String, DevError> {
    let relative = path
        .strip_prefix(repository)
        .map_err(|_| refusal("retained workspace artifact is outside the repository"))?;
    safe_components(relative)?;
    path_string(relative)
}

fn retained_path(repository: &Path, label: &str) -> Result<PathBuf, DevError> {
    let relative = Path::new(label);
    safe_components(relative)?;
    if relative.is_absolute() {
        return Err(refusal(
            "retained workspace proof requires a repository-relative path",
        ));
    }
    let components: Vec<_> = relative.components().collect();
    let marker = components
        .iter()
        .position(|component| component.as_os_str() == "retained-cargo-origin")
        .ok_or_else(|| refusal("retained workspace proof has no origin retention directory"))?;
    if marker + 4 != components.len() {
        return Err(refusal("malformed retained workspace artifact path"));
    }
    let gate = components
        .get(marker + 1)
        .and_then(|component| component.as_os_str().to_str())
        .ok_or_else(|| refusal("invalid retained gate directory"))?;
    component_name(gate)?;
    for offset in [2, 3] {
        let index = components
            .get(marker + offset)
            .and_then(|component| component.as_os_str().to_str())
            .ok_or_else(|| refusal("invalid retained workspace artifact index"))?;
        if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(refusal("invalid retained workspace artifact index"));
        }
    }
    Ok(repository.join(relative))
}

fn create_directory(parent: &Path, name: &str) -> Result<PathBuf, DevError> {
    component_name(name)?;
    let path = parent.join(name);
    match fs::create_dir(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    canonical_directory(&path, Some(parent))?;
    Ok(path)
}

fn component_name(value: &str) -> Result<(), DevError> {
    bounded_string(value)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(refusal("unsafe workspace retention path component"));
    }
    Ok(())
}

fn bounded_string(value: &str) -> Result<(), DevError> {
    if value.is_empty() || value.len() > MAXIMUM_STRING_BYTES || value.chars().any(char::is_control)
    {
        return Err(refusal(
            "missing, oversized or unsafe workspace identity string",
        ));
    }
    Ok(())
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, DevError> {
    let string = value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| refusal(format!("missing workspace {field} identity")))?;
    bounded_string(string)?;
    Ok(string)
}

fn boolean_field(value: &Value, field: &str) -> Result<(), DevError> {
    if value.get(field).and_then(Value::as_bool).is_none() {
        return Err(refusal(format!(
            "missing workspace {field} boolean identity"
        )));
    }
    Ok(())
}

fn string_array(
    value: Option<&Value>,
    maximum: usize,
    empty_allowed: bool,
) -> Result<Vec<String>, DevError> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| refusal("missing workspace string-array identity"))?;
    if values.len() > maximum || (!empty_allowed && values.is_empty()) {
        return Err(refusal("invalid workspace string-array length"));
    }
    let mut strings = Vec::with_capacity(values.len());
    let mut distinct = BTreeSet::new();
    for value in values {
        let string = value
            .as_str()
            .ok_or_else(|| refusal("non-string workspace identity array member"))?;
        bounded_string(string)?;
        if !distinct.insert(string) {
            return Err(refusal("duplicate workspace string-array identity member"));
        }
        strings.push(string.to_owned());
    }
    Ok(strings)
}

fn checked_total(
    current: usize,
    addition: usize,
    maximum: usize,
    label: &str,
) -> Result<usize, DevError> {
    let total = current
        .checked_add(addition)
        .ok_or_else(|| refusal(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(refusal(format!("{label} exceed their limit")));
    }
    Ok(total)
}

fn refusal(message: impl Into<String>) -> DevError {
    DevError::corrupt(message)
}

// Value's ordinary decoder accepts duplicate object keys. Original Cargo evidence
// must have one unambiguous value for each identity field, including nested fields.
struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON without duplicate object keys")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(value)))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                let number = serde_json::Number::from_f64(value)
                    .ok_or_else(|| E::custom("non-finite Cargo number"))?;
                Ok(UniqueValue(Value::Number(number)))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value.to_owned())))
            }

            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value)))
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueValue(value)) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(name) = map.next_key::<String>()? {
                    if values.contains_key(&name) {
                        return Err(de::Error::custom("duplicate Cargo object key"));
                    }
                    let UniqueValue(value) = map.next_value()?;
                    values.insert(name, value);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}
