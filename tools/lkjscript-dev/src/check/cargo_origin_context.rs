use super::model::{InputSnapshot, RuntimeIdentity};
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof, VerificationDigest};
use crate::process;
#[cfg(unix)]
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Component, Path, PathBuf};

const MAXIMUM_ANCESTORS: usize = 64;
const MAXIMUM_CONFIG_PATHS: usize = 130;
const MAXIMUM_CONFIG_BYTES: u64 = 1024 * 1024;
const MAXIMUM_TOTAL_CONFIG_BYTES: u64 = 8 * 1024 * 1024;
const MAXIMUM_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAXIMUM_COMPILER_BYTES: u64 = 512 * 1024 * 1024;
const WORKSPACE_MANIFESTS: [&str; 3] = [
    "Cargo.toml",
    "tools/lkjscript-dev/Cargo.toml",
    "tools/lkjscript-site/Cargo.toml",
];
const WORKSPACE_MEMBERS: &str = "members=[\".\",\"tools/lkjscript-dev\",\"tools/lkjscript-site\"]";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Context {
    pub(super) repository: String,
    pub(super) target: String,
    pub(super) input_digest: VerificationDigest,
    pub(super) cargo_lock_digest: VerificationDigest,
    pub(super) runtime_digest: VerificationDigest,
    pub(super) configs: Vec<FileProof>,
    pub(super) workspace_manifests: Vec<String>,
    pub(super) compiler_files: Vec<FileProof>,
}

impl Context {
    pub(super) fn capture(
        repository: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
    ) -> Result<Self, DevError> {
        let target = Self::target(repository)?;
        Self::capture_at(repository, &target, snapshot, runtime)
    }

    pub(super) fn target(repository: &Path) -> Result<PathBuf, DevError> {
        let repository = canonical_repository(repository)?;
        let environment = process::environment();
        let target = match environment.get("CARGO_TARGET_DIR") {
            Some(configured) if !configured.is_empty() => PathBuf::from(configured),
            Some(_) => {
                return Err(DevError::infrastructure(
                    "Cargo target environment path is empty",
                ));
            }
            None => repository.join("target"),
        };
        physical_target(&repository, &target)
    }

    pub(super) fn capture_at(
        repository: &Path,
        target: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
    ) -> Result<Self, DevError> {
        let repository = canonical_repository(repository)?;
        let target = physical_target(&repository, target)?;
        let environment = process::environment();
        if environment
            .get("CARGO_BUILD_TARGET")
            .is_some_and(|value| !supported_target(value))
        {
            return Err(DevError::infrastructure(
                "Cargo custom target specifications are unsupported",
            ));
        }
        if let Some(flags) = environment.get("RUSTFLAGS")
            && !safe_flags(&flags.split_ascii_whitespace().collect::<Vec<_>>())
        {
            return Err(DevError::infrastructure(
                "Cargo rustflags reference unsupported origins",
            ));
        }
        if let Some(flags) = environment.get("CARGO_ENCODED_RUSTFLAGS") {
            let flags = if flags.is_empty() {
                Vec::new()
            } else {
                flags.split('\u{1f}').collect()
            };
            if !safe_flags(&flags) {
                return Err(DevError::infrastructure(
                    "Cargo encoded rustflags reference unsupported origins",
                ));
            }
        }
        if runtime.digest != super::snapshot::runtime_digest(runtime)? {
            return Err(DevError::corrupt(
                "Cargo origin runtime digest is inconsistent",
            ));
        }
        let cargo_home = match environment.get("CARGO_HOME") {
            Some(home) => normalize_absolute(Path::new(home))?,
            None => {
                let home = environment.get("HOME").ok_or_else(|| {
                    DevError::infrastructure("Cargo origin requires approved HOME or CARGO_HOME")
                })?;
                normalize_absolute(&Path::new(home).join(".cargo"))?
            }
        };
        let configs = capture_configs(&repository, &cargo_home)?;
        let workspace_manifests = capture_workspace(&repository, snapshot)?;
        let compiler_files = capture_compiler(&repository)?;
        Ok(Self {
            repository: path_text(&repository)?,
            target: path_text(&target)?,
            input_digest: snapshot.digest.clone(),
            cargo_lock_digest: snapshot.cargo_lock_digest.clone(),
            runtime_digest: runtime.digest.clone(),
            configs,
            workspace_manifests,
            compiler_files,
        })
    }

    // The caller supplies a current source/runtime snapshot. This separately
    // rereads the effective config candidates and the fixed workspace manifests.
    pub(super) fn recheck(
        &self,
        repository: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
    ) -> Result<(), DevError> {
        if *self != Self::capture_at(repository, Path::new(&self.target), snapshot, runtime)? {
            return Err(DevError::corrupt(
                "Cargo compilation origin context changed",
            ));
        }
        Ok(())
    }

    pub(super) fn ledger_path(&self) -> Result<PathBuf, DevError> {
        let target = Path::new(&self.target);
        let parent = target
            .parent()
            .ok_or_else(|| DevError::corrupt("Cargo compilation origin target has no parent"))?;
        let key = VerificationDigest::of(self.target.as_bytes());
        Ok(parent
            .join(".artifacts/lkjscript-dev/cargo-origin-target-locks")
            .join(format!("{}.json", key.as_str())))
    }
}

#[derive(Debug)]
pub(super) struct TargetLease {
    // Closing the sole descriptor releases the advisory lock on every exit.
    _file: File,
}

impl TargetLease {
    pub(super) fn acquire(context: &Context) -> Result<Self, DevError> {
        #[cfg(unix)]
        {
            let repository = canonical_repository(Path::new(&context.repository))?;
            let target = physical_target(&repository, Path::new(&context.target))?;
            if path_text(&repository)? != context.repository
                || path_text(&target)? != context.target
            {
                return Err(DevError::corrupt(
                    "Cargo target identity changed before lease",
                ));
            }
            let parent = target.parent().ok_or_else(|| {
                DevError::infrastructure("Cargo physical target has no stable parent")
            })?;
            let mut directory = parent.to_path_buf();
            for component in [".artifacts", "lkjscript-dev", "cargo-origin-target-locks"] {
                directory.push(component);
                create_owned_directory(&directory)?;
            }
            let key = VerificationDigest::of(context.target.as_bytes());
            let path = directory.join(format!("{}.lock", key.as_str()));
            match fs::symlink_metadata(&path) {
                Ok(metadata) => validate_lock_metadata(&metadata)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(DevError::infrastructure(format!(
                        "inspect existing Cargo target lease: {error}"
                    )));
                }
            }
            let descriptor = rustix::fs::open(
                &path,
                rustix::fs::OFlags::RDWR
                    | rustix::fs::OFlags::CREATE
                    | rustix::fs::OFlags::CLOEXEC
                    | rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::NONBLOCK,
                rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
            )
            .map_err(|error| {
                DevError::infrastructure(format!("open Cargo target lease: {error}"))
            })?;
            let file = File::from(descriptor);
            let opened = file.metadata().map_err(|error| {
                DevError::infrastructure(format!("inspect Cargo target lease: {error}"))
            })?;
            validate_lock_metadata(&opened)?;
            FileExt::try_lock_exclusive(&file).map_err(|error| {
                DevError::infrastructure(format!("Cargo target lease is unavailable: {error}"))
            })?;
            check_directory_chain(&directory, false)?;
            let selected = fs::symlink_metadata(&path).map_err(|error| {
                DevError::infrastructure(format!("recheck Cargo target lease: {error}"))
            })?;
            validate_lock_metadata(&selected)?;
            if !same_file(&opened, &selected) || physical_target(&repository, &target)? != target {
                return Err(DevError::corrupt("Cargo target lease entry changed"));
            }
            Ok(Self { _file: file })
        }
        #[cfg(not(unix))]
        {
            let _ = context;
            Err(DevError::unavailable(
                "Cargo compilation origin target leases require Unix metadata protections",
            ))
        }
    }
}

fn capture_configs(repository: &Path, cargo_home: &Path) -> Result<Vec<FileProof>, DevError> {
    let mut paths = BTreeSet::new();
    for (index, ancestor) in repository.ancestors().enumerate() {
        if index >= MAXIMUM_ANCESTORS {
            return Err(DevError::infrastructure(
                "Cargo config ancestor count exceeds limit",
            ));
        }
        for name in ["config", "config.toml"] {
            paths.insert(ancestor.join(".cargo").join(name));
        }
    }
    for name in ["config", "config.toml"] {
        paths.insert(cargo_home.join(name));
    }
    if paths.len() > MAXIMUM_CONFIG_PATHS {
        return Err(DevError::infrastructure(
            "Cargo config path count exceeds limit",
        ));
    }
    let mut proofs = Vec::with_capacity(paths.len());
    let mut total = 0_u64;
    for path in paths {
        let proof = bounded_proof(&path, path_text(&path)?, MAXIMUM_CONFIG_BYTES)?;
        if proof.kind == FileKind::File {
            let text = bounded_text(&path, MAXIMUM_CONFIG_BYTES)?;
            validate_config(&text)?;
            if proof != bounded_proof(&path, path_text(&path)?, MAXIMUM_CONFIG_BYTES)? {
                return Err(DevError::corrupt("Cargo config changed during capture"));
            }
        }
        let bytes = proof.bytes.unwrap_or_default();
        total = total
            .checked_add(bytes)
            .ok_or_else(|| DevError::infrastructure("Cargo config byte count overflow"))?;
        if total > MAXIMUM_TOTAL_CONFIG_BYTES {
            return Err(DevError::infrastructure("Cargo config bytes exceed limit"));
        }
        proofs.push(proof);
    }
    Ok(proofs)
}

fn capture_compiler(repository: &Path) -> Result<Vec<FileProof>, DevError> {
    let environment = process::environment();
    for name in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
        if environment.get(name).is_some_and(|value| !value.is_empty()) {
            return Err(DevError::infrastructure(
                "Cargo compiler wrappers are unsupported",
            ));
        }
    }
    let selected = super::snapshot::observe_executable(repository, "rustc")?;
    let selected_path = selected.path.ok_or_else(|| {
        DevError::infrastructure("Cargo compilation origin compiler is unavailable")
    })?;
    let sysroot = super::snapshot::checked_text(repository, &["rustc", "--print", "sysroot"])?;
    let actual_path = normalize_absolute(&Path::new(&sysroot).join("bin/rustc"))?;
    let actual = bounded_proof(
        &actual_path,
        path_text(&actual_path)?,
        MAXIMUM_COMPILER_BYTES,
    )?;
    if actual.kind != FileKind::File {
        return Err(DevError::infrastructure(
            "Cargo sysroot compiler file is unavailable",
        ));
    }
    if let Some(configured) = environment.get("RUSTC")
        && configured != "rustc"
    {
        let path = Path::new(configured);
        let path = if path.is_absolute() {
            normalize_absolute(path)?
        } else {
            normalize_absolute(&repository.join(path))?
        };
        if path != normalize_absolute(&selected_path)? && path != actual_path {
            return Err(DevError::infrastructure(
                "Cargo custom compiler selection is unsupported",
            ));
        }
    }
    if selected.proof != super::snapshot::executable_proof(repository, "rustc")? {
        return Err(DevError::corrupt(
            "Cargo compiler selection changed during capture",
        ));
    }
    let mut proofs = vec![selected.proof.entry];
    if let Some(resolved) = selected.proof.resolved {
        proofs.push(resolved);
    }
    proofs.push(actual);
    Ok(proofs)
}

fn capture_workspace(repository: &Path, snapshot: &InputSnapshot) -> Result<Vec<String>, DevError> {
    let mut manifests = Vec::with_capacity(WORKSPACE_MANIFESTS.len());
    for label in WORKSPACE_MANIFESTS {
        let path = repository.join(label);
        let before = bounded_proof(&path, label.to_owned(), MAXIMUM_MANIFEST_BYTES)?;
        if before.kind != FileKind::File {
            return Err(DevError::infrastructure(
                "Cargo workspace manifest is absent",
            ));
        }
        let mut entries = snapshot
            .entries
            .iter()
            .filter(|entry| entry.proof.path == label);
        let entry = entries.next().ok_or_else(|| {
            DevError::corrupt("Cargo workspace manifest is absent from source snapshot")
        })?;
        if entries.next().is_some() || entry.proof != before {
            return Err(DevError::corrupt(
                "Cargo workspace manifest source binding changed",
            ));
        }
        let text = bounded_text(&path, MAXIMUM_MANIFEST_BYTES)?;
        let (workspace, isolated) = validate_workspace_manifest(label, &text)?;
        if before != bounded_proof(&path, label.to_owned(), MAXIMUM_MANIFEST_BYTES)? {
            return Err(DevError::corrupt(
                "Cargo workspace manifest changed during capture",
            ));
        }
        manifests.push(path_text(&path)?);
        if label == "Cargo.toml" && !workspace {
            if !isolated {
                refuse_ancestor_manifests(repository)?;
            }
            break;
        }
    }
    Ok(manifests)
}

// Closed, line-oriented shapes only. Quoted keys, dotted assignments, multiline
// values and dependency routing beyond the maintained root link are refused.
fn validate_workspace_manifest(label: &str, text: &str) -> Result<(bool, bool), DevError> {
    let root = label == "Cargo.toml";
    let mut table = String::new();
    let mut workspace_seen = false;
    let mut members_seen = false;
    let mut resolver_seen = false;
    let mut multiple_members = false;
    for raw in text.lines() {
        let line = compact_line(raw)?;
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            table = manifest_table(&line)?;
            if table == "workspace" {
                if !root || workspace_seen {
                    return Err(DevError::infrastructure(
                        "unsupported Cargo workspace declaration",
                    ));
                }
                workspace_seen = true;
            }
            continue;
        }
        let (key, value) = assignment(&line)?;
        if table == "workspace" {
            if line == WORKSPACE_MEMBERS && !members_seen {
                members_seen = true;
                multiple_members = true;
            } else if line == "members=[]" && !members_seen {
                members_seen = true;
            } else if line == "resolver=\"3\"" && !resolver_seen {
                resolver_seen = true;
            } else {
                return Err(DevError::infrastructure(
                    "Cargo workspace member layout is unsupported",
                ));
            }
        } else if key == "workspace" || table.is_empty() {
            return Err(DevError::infrastructure(
                "unsupported implicit Cargo workspace declaration",
            ));
        } else if matches!(
            table.as_str(),
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            validate_dependency(label, key, value)?;
        } else if key == "build"
            || !simple_value(value)
            || (key == "path" && !relative_source_path(value))
        {
            return Err(DevError::infrastructure(
                "unsupported Cargo manifest value or source path",
            ));
        }
    }
    if root && multiple_members && !resolver_seen {
        return Err(DevError::infrastructure(
            "Cargo workspace member layout is incomplete",
        ));
    }
    Ok((multiple_members, workspace_seen))
}

fn refuse_ancestor_manifests(repository: &Path) -> Result<(), DevError> {
    for (index, ancestor) in repository.ancestors().skip(1).enumerate() {
        if index >= MAXIMUM_ANCESTORS {
            return Err(DevError::infrastructure(
                "Cargo workspace ancestor count exceeds limit",
            ));
        }
        match fs::symlink_metadata(ancestor.join("Cargo.toml")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => {
                return Err(DevError::infrastructure(
                    "implicit ancestor Cargo workspace routing is unsupported",
                ));
            }
            Err(error) => {
                return Err(DevError::infrastructure(format!(
                    "inspect ancestor Cargo manifest: {error}"
                )));
            }
        }
    }
    Ok(())
}

fn manifest_table(line: &str) -> Result<String, DevError> {
    let table = if let Some(table) = line.strip_prefix("[[").and_then(|s| s.strip_suffix("]]")) {
        if !matches!(table, "bin" | "test" | "bench" | "example") {
            return Err(DevError::infrastructure(
                "unsupported Cargo manifest array table",
            ));
        }
        table
    } else {
        let table = table_name(line)?;
        let supported = matches!(
            table,
            "package"
                | "workspace"
                | "dependencies"
                | "dev-dependencies"
                | "build-dependencies"
                | "lib"
                | "features"
                | "lints.clippy"
                | "lints.rust"
        ) || table.strip_prefix("profile.").is_some_and(bare_key);
        if !supported {
            return Err(DevError::infrastructure("unsupported Cargo manifest table"));
        }
        table
    };
    Ok(table.to_owned())
}

fn validate_dependency(label: &str, key: &str, value: &str) -> Result<(), DevError> {
    if string_value(value) {
        return Ok(());
    }
    if label == "tools/lkjscript-dev/Cargo.toml"
        && key == "lkjscript"
        && value == "{path=\"../..\"}"
    {
        return Ok(());
    }
    let inline = value
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| DevError::infrastructure("unsupported Cargo dependency value"))?;
    let mut keys = BTreeSet::new();
    for field in separated_values(inline)? {
        let (key, value) = assignment(field)?;
        if !keys.insert(key) {
            return Err(DevError::infrastructure(
                "duplicate Cargo dependency option",
            ));
        }
        let supported = match key {
            "version" | "package" => string_value(value),
            "default-features" | "optional" => matches!(value, "true" | "false"),
            "features" => string_array(value),
            _ => false,
        };
        if !supported {
            return Err(DevError::infrastructure(
                "unsupported Cargo dependency routing or option",
            ));
        }
    }
    if !keys.contains("version") {
        return Err(DevError::infrastructure(
            "Cargo registry dependency version is absent",
        ));
    }
    Ok(())
}

fn validate_config(text: &str) -> Result<(), DevError> {
    let mut table = String::new();
    for raw in text.lines() {
        let line = compact_line(raw)?;
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            let name = table_name(&line)?;
            if !matches!(name, "build" | "net" | "term")
                && !name.strip_prefix("target.").is_some_and(supported_target)
            {
                return Err(DevError::infrastructure(
                    "unsupported Cargo config routing table",
                ));
            }
            table = name.to_owned();
            continue;
        }
        let (key, value) = assignment(&line)?;
        let supported = match table.as_str() {
            "build" => match key {
                "jobs" => integer_value(value),
                "target" => target_value(value),
                "rustflags" => flags_value(value),
                "rustdocflags" => value == "[]" || value == "\"\"",
                _ => false,
            },
            "net" => match key {
                "offline" | "git-fetch-with-cli" => matches!(value, "true" | "false"),
                "retry" => integer_value(value),
                _ => false,
            },
            "term" => match key {
                "quiet" | "verbose" => matches!(value, "true" | "false"),
                "color" => string_value(value),
                _ => false,
            },
            _ if table.starts_with("target.") => {
                (key == "rustflags" && flags_value(value))
                    || (key == "rustdocflags" && (value == "[]" || value == "\"\""))
            }
            _ => false,
        };
        if !supported {
            return Err(DevError::infrastructure(
                "unsupported Cargo config include or dynamic routing",
            ));
        }
    }
    Ok(())
}

fn table_name(line: &str) -> Result<&str, DevError> {
    let table = line
        .strip_prefix('[')
        .and_then(|line| line.strip_suffix(']'))
        .ok_or_else(|| DevError::infrastructure("unsupported Cargo table syntax"))?;
    if !table.split('.').all(bare_key) {
        return Err(DevError::infrastructure(
            "quoted or escaped Cargo table keys are unsupported",
        ));
    }
    Ok(table)
}

fn assignment(line: &str) -> Result<(&str, &str), DevError> {
    let (key, value) = line
        .split_once('=')
        .ok_or_else(|| DevError::infrastructure("multiline Cargo assignments are unsupported"))?;
    if !bare_key(key) || value.is_empty() {
        return Err(DevError::infrastructure(
            "quoted, escaped or dotted Cargo keys are unsupported",
        ));
    }
    Ok((key, value))
}

fn bare_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn string_value(value: &str) -> bool {
    let inner = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'));
    match inner {
        Some(inner) => inner.bytes().all(|byte| {
            byte.is_ascii() && !byte.is_ascii_control() && !matches!(byte, b'"' | b'\\')
        }),
        None => false,
    }
}

fn integer_value(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn string_array(value: &str) -> bool {
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return false;
    };
    match separated_values(inner) {
        Ok(values) => values.into_iter().all(string_value),
        Err(_) => false,
    }
}

fn simple_value(value: &str) -> bool {
    string_value(value)
        || string_array(value)
        || integer_value(value)
        || matches!(value, "true" | "false")
}

fn supported_target(value: &str) -> bool {
    matches!(
        value,
        "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl"
    )
}

fn target_value(value: &str) -> bool {
    if string_value(value) {
        return supported_target(&value[1..value.len() - 1]);
    }
    match string_values(value) {
        Some(values) => values.into_iter().all(supported_target),
        None => false,
    }
}

fn flags_value(value: &str) -> bool {
    if string_value(value) {
        return safe_flags(
            &value[1..value.len() - 1]
                .split_ascii_whitespace()
                .collect::<Vec<_>>(),
        );
    }
    match string_values(value) {
        Some(values) => safe_flags(&values),
        None => false,
    }
}

fn string_values(value: &str) -> Option<Vec<&str>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let values = separated_values(inner).ok()?;
    if !values.iter().all(|value| string_value(value)) {
        return None;
    }
    Some(
        values
            .into_iter()
            .map(|value| &value[1..value.len() - 1])
            .collect(),
    )
}

// These flags contain no input-file references. Other flags, including response
// files, search paths, extern files and alternate sysroots, need a later owner.
fn safe_flags(flags: &[&str]) -> bool {
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        let supported = match *flag {
            "-C" => flags.next().is_some_and(|value| safe_codegen(value)),
            "--cfg" => flags.next().is_some_and(|value| bare_key(value)),
            "--cap-lints" => flags
                .next()
                .is_some_and(|value| matches!(*value, "allow" | "warn" | "deny" | "forbid")),
            _ => false,
        };
        if !supported {
            return false;
        }
    }
    true
}

fn safe_codegen(value: &str) -> bool {
    let Some((key, value)) = value.split_once('=') else {
        return false;
    };
    match key {
        "link-arg" => value == "-fuse-ld=mold",
        "opt-level" => matches!(value, "0" | "1" | "2" | "3" | "s" | "z"),
        "debuginfo" => matches!(
            value,
            "0" | "1" | "2" | "none" | "line-tables-only" | "limited" | "full"
        ),
        "debug-assertions" | "overflow-checks" => matches!(value, "on" | "off" | "yes" | "no"),
        "panic" => matches!(value, "abort" | "unwind"),
        "codegen-units" => integer_value(value),
        "lto" => matches!(value, "off" | "thin" | "fat"),
        _ => false,
    }
}

fn relative_source_path(value: &str) -> bool {
    if !string_value(value) {
        return false;
    }
    let path = &value[1..value.len() - 1];
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn separated_values(value: &str) -> Result<Vec<&str>, DevError> {
    let mut result = Vec::new();
    let mut quote = false;
    let mut array = false;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        match character {
            '"' => quote = !quote,
            '\\' | '\'' => {
                return Err(DevError::infrastructure(
                    "escaped or literal Cargo values are unsupported",
                ));
            }
            '[' if !quote && !array => array = true,
            ']' if !quote && array => array = false,
            '[' | ']' | '{' | '}' if !quote => {
                return Err(DevError::infrastructure(
                    "nested Cargo values are unsupported",
                ));
            }
            ',' if !quote && !array => {
                if start == index {
                    return Err(DevError::infrastructure("empty Cargo value is unsupported"));
                }
                result.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if quote || array {
        return Err(DevError::infrastructure(
            "unterminated Cargo value is unsupported",
        ));
    }
    if start < value.len() {
        result.push(&value[start..]);
    }
    Ok(result)
}

fn compact_line(raw: &str) -> Result<String, DevError> {
    let mut result = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in raw.chars() {
        if let Some(delimiter) = quote {
            result.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' && delimiter == '"' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character == '#' {
            break;
        } else if character == '"' || character == '\'' {
            result.push(character);
            quote = Some(character);
        } else if !character.is_ascii_whitespace() {
            result.push(character);
        }
    }
    if quote.is_some() {
        return Err(DevError::infrastructure(
            "multiline Cargo manifest strings are unsupported",
        ));
    }
    Ok(result)
}

fn bounded_text(path: &Path, maximum: u64) -> Result<String, DevError> {
    let mut text = String::new();
    File::open(path)
        .map_err(|error| DevError::infrastructure(format!("open Cargo origin text: {error}")))?
        .take(maximum + 1)
        .read_to_string(&mut text)
        .map_err(|error| DevError::infrastructure(format!("read Cargo origin text: {error}")))?;
    if text.len() as u64 > maximum {
        return Err(DevError::infrastructure("Cargo origin text exceeds limit"));
    }
    Ok(text)
}

fn bounded_proof(path: &Path, label: String, maximum: u64) -> Result<FileProof, DevError> {
    let parent = path
        .parent()
        .ok_or_else(|| DevError::infrastructure("Cargo origin file path has no parent"))?;
    check_directory_chain(parent, true)?;
    let before = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum
            {
                return Err(DevError::infrastructure(
                    "Cargo origin file kind or size is unsupported",
                ));
            }
            Some(metadata)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "inspect Cargo origin file: {error}"
            )));
        }
    };
    let proof = evidence::proof(path, label)?;
    check_directory_chain(parent, true)?;
    match before {
        Some(before) => {
            let after = fs::symlink_metadata(path).map_err(|error| {
                DevError::infrastructure(format!("recheck Cargo origin file: {error}"))
            })?;
            if proof.kind != FileKind::File
                || proof.bytes != Some(before.len())
                || before.len() > maximum
                || !same_file(&before, &after)
                || before.modified().ok() != after.modified().ok()
            {
                return Err(DevError::corrupt(
                    "Cargo origin file changed during capture",
                ));
            }
        }
        None => {
            if proof.kind != FileKind::Missing {
                return Err(DevError::corrupt(
                    "Cargo origin absent file changed during capture",
                ));
            }
            match fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => {
                    return Err(DevError::corrupt(
                        "Cargo origin absent file changed during capture",
                    ));
                }
            }
        }
    }
    Ok(proof)
}

fn canonical_repository(repository: &Path) -> Result<PathBuf, DevError> {
    let repository = fs::canonicalize(repository).map_err(|error| {
        DevError::infrastructure(format!("canonicalize Cargo repository: {error}"))
    })?;
    check_directory_chain(&repository, false)?;
    Ok(repository)
}

fn physical_target(repository: &Path, target: &Path) -> Result<PathBuf, DevError> {
    let target = if target.is_absolute() {
        normalize_absolute(target)?
    } else {
        normalize_absolute(&repository.join(target))?
    };
    let parent = target
        .parent()
        .ok_or_else(|| DevError::infrastructure("Cargo physical target has no stable parent"))?;
    check_directory_chain(parent, false)?;
    check_directory_chain(&target, true)?;
    match fs::symlink_metadata(&target) {
        Ok(_) => fs::canonicalize(&target).map_err(|error| {
            DevError::infrastructure(format!("canonicalize Cargo physical target: {error}"))
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(target),
        Err(error) => Err(DevError::infrastructure(format!(
            "inspect Cargo target: {error}"
        ))),
    }
}

fn normalize_absolute(path: &Path) -> Result<PathBuf, DevError> {
    if !path.is_absolute() {
        return Err(DevError::infrastructure(
            "Cargo origin environment path must be absolute",
        ));
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                check_directory_chain(&result, false)?;
                result.pop();
            }
            _ => result.push(component.as_os_str()),
        }
    }
    Ok(result)
}

fn check_directory_chain(path: &Path, allow_missing: bool) -> Result<(), DevError> {
    let path = normalize_absolute(path)?;
    let mut current = PathBuf::new();
    let mut missing = false;
    for (index, component) in path.components().enumerate() {
        if index >= MAXIMUM_ANCESTORS {
            return Err(DevError::infrastructure(
                "Cargo origin path depth exceeds limit",
            ));
        }
        current.push(component.as_os_str());
        if missing {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(DevError::infrastructure(
                        "Cargo origin directory chain is unsupported",
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && allow_missing => {
                missing = true;
            }
            Err(error) => {
                return Err(DevError::infrastructure(format!(
                    "inspect Cargo origin directory: {error}"
                )));
            }
        }
    }
    Ok(())
}

fn path_text(path: &Path) -> Result<String, DevError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| DevError::infrastructure("Cargo origin path is not UTF-8"))
}

fn same_file(left: &Metadata, right: &Metadata) -> bool {
    #[cfg(unix)]
    {
        left.dev() == right.dev()
            && left.ino() == right.ino()
            && left.mode() == right.mode()
            && left.len() == right.len()
            && left.file_type() == right.file_type()
    }
    #[cfg(not(unix))]
    {
        left.len() == right.len() && left.file_type() == right.file_type()
    }
}

#[cfg(unix)]
fn create_owned_directory(path: &Path) -> Result<(), DevError> {
    let parent = path
        .parent()
        .ok_or_else(|| DevError::infrastructure("Cargo target lease directory has no parent"))?;
    check_directory_chain(parent, false)?;
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "create Cargo lease directory: {error}"
            )));
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect Cargo lease directory: {error}"))
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o022 != 0
    {
        return Err(DevError::infrastructure(
            "Cargo lease directory is not privately owned",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn validate_lock_metadata(metadata: &Metadata) -> Result<(), DevError> {
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o022 != 0
        || metadata.nlink() != 1
        || metadata.len() != 0
    {
        return Err(DevError::infrastructure(
            "Cargo target lease entry is not a private regular file",
        ));
    }
    Ok(())
}
