use super::archive;
use super::model::SchemaIdentity;
use crate::error::DevError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

const IDENTITY_SCHEMA: &str = "lkjscript-application-verifier-handoff";
const IDENTITY_SCHEMA_VERSION: u32 = 3;
pub(super) const EXECUTABLE_NAME: &str = "lkjscript-dev";
pub(super) const IDENTITY_NAME: &str = "verifier-identity.json";
const ROLES: [&str; 4] = [
    "release-verify",
    "distributed-http",
    "outbound-http",
    "stateful-http",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FileIdentity {
    name: String,
    byte_length: u64,
    sha256: String,
    mode: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct VerifierIdentity {
    schema: SchemaIdentity,
    tag: String,
    commit_sha: String,
    roles: Vec<String>,
    file: FileIdentity,
}

struct PrepareOptions {
    executable: PathBuf,
    output: PathBuf,
    tag: String,
    commit: String,
}

struct VerifyOptions {
    executable: PathBuf,
    identity: PathBuf,
    tag: String,
    commit: String,
    expected_sha256: String,
    expected_bytes: u64,
}

pub(super) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let subcommand = crate::next_utf8(&mut arguments, "release verifier subcommand")?
        .ok_or_else(|| DevError::usage("release verifier subcommand is required"))?;
    match subcommand.as_str() {
        "prepare" => prepare(parse_prepare(arguments)?),
        "verify" => verify(parse_verify(arguments)?),
        other => Err(DevError::usage(format!(
            "unknown release verifier subcommand '{other}'"
        ))),
    }
}

fn prepare(options: PrepareOptions) -> Result<u8, DevError> {
    validate_tag(&options.tag)?;
    super::validate_git_sha(&options.commit, "verifier handoff commit")?;
    super::require_absolute_regular_executable(&options.executable, "host verifier executable")?;
    require_absent_output(&options.output)?;
    let parent = options
        .output
        .parent()
        .ok_or_else(|| DevError::usage("verifier handoff output has no parent"))?;
    let work = tempfile::Builder::new()
        .prefix(".lkjscript-verifier-handoff-")
        .tempdir_in(parent)
        .map_err(|error| DevError::infrastructure(format!("create verifier handoff: {error}")))?;
    fs::set_permissions(work.path(), fs::Permissions::from_mode(0o700))?;
    let executable = work.path().join(EXECUTABLE_NAME);
    archive::copy_new(&options.executable, &executable, 0o755)?;
    let file = observe_executable(&executable)?;
    let identity = VerifierIdentity {
        schema: SchemaIdentity {
            identity: IDENTITY_SCHEMA.to_owned(),
            version: IDENTITY_SCHEMA_VERSION,
        },
        tag: options.tag,
        commit_sha: options.commit,
        roles: ROLES.iter().map(|role| (*role).to_owned()).collect(),
        file,
    };
    archive::write_new(
        &work.path().join(IDENTITY_NAME),
        &archive::canonical_json(&identity)?,
        0o644,
    )?;
    validate_inventory(work.path())?;
    super::publish_directory_no_replace(work.path(), &options.output)?;
    archive::synchronize_directory(parent)?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "status": "passed",
            "directory": options.output,
            "executable": EXECUTABLE_NAME,
            "identity": IDENTITY_NAME,
            "bytes": identity.file.byte_length,
            "sha256": identity.file.sha256,
            "schema": identity.schema,
        }))?
    );
    Ok(0)
}

fn verify(options: VerifyOptions) -> Result<u8, DevError> {
    validate_tag(&options.tag)?;
    super::validate_git_sha(&options.commit, "verifier handoff commit")?;
    super::require_absolute_regular_executable(
        &options.executable,
        "transferred verifier executable",
    )?;
    super::require_absolute_regular(&options.identity, "transferred verifier identity")?;
    let bytes = fs::read(&options.identity)?;
    if bytes.len() > 1024 * 1024 {
        return Err(DevError::corrupt(
            "transferred verifier identity exceeds 1 MiB",
        ));
    }
    let identity: VerifierIdentity = serde_json::from_slice(&bytes)
        .map_err(|error| DevError::corrupt(format!("decode verifier identity: {error}")))?;
    if archive::canonical_json(&identity)? != bytes {
        return Err(DevError::corrupt(
            "transferred verifier identity is not canonical",
        ));
    }
    let observed = observe_executable(&options.executable)?;
    if identity.schema.identity != IDENTITY_SCHEMA
        || identity.schema.version != IDENTITY_SCHEMA_VERSION
        || identity.tag != options.tag
        || identity.commit_sha != options.commit
        || identity.roles != ROLES
        || identity.file.name != EXECUTABLE_NAME
        || identity.file != observed
        || identity.file.sha256 != options.expected_sha256
        || identity.file.byte_length != options.expected_bytes
        || options
            .executable
            .file_name()
            .and_then(|name| name.to_str())
            != Some(EXECUTABLE_NAME)
        || options.identity.file_name().and_then(|name| name.to_str()) != Some(IDENTITY_NAME)
        || options.executable.parent() != options.identity.parent()
    {
        return Err(DevError::corrupt(
            "transferred verifier identity or executable binding mismatch",
        ));
    }
    let parent = options
        .executable
        .parent()
        .ok_or_else(|| DevError::corrupt("transferred verifier has no parent"))?;
    validate_inventory(parent)?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "status": "passed",
            "tag": identity.tag,
            "commit_sha": identity.commit_sha,
            "executable": options.executable,
            "identity": options.identity,
            "bytes": identity.file.byte_length,
            "sha256": identity.file.sha256,
            "schema": identity.schema,
            "roles": identity.roles,
        }))?
    );
    Ok(0)
}

fn observe_executable(path: &Path) -> Result<FileIdentity, DevError> {
    let metadata = archive::ensure_regular(path, "verifier executable")?;
    let mode = metadata.permissions().mode() & 0o7777;
    if mode != 0o755 {
        return Err(DevError::corrupt(format!(
            "verifier executable mode is {mode:o}, expected 755"
        )));
    }
    let (sha256, byte_length) = archive::sha256_file(path)?;
    Ok(FileIdentity {
        name: EXECUTABLE_NAME.to_owned(),
        byte_length,
        sha256: sha256.as_str().to_owned(),
        mode,
    })
}

fn validate_inventory(directory: &Path) -> Result<(), DevError> {
    let metadata = fs::symlink_metadata(directory)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::corrupt(
            "verifier handoff is not a regular directory",
        ));
    }
    let mut names = fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(std::io::Error::other(
                    "verifier handoff contains a non-regular entry",
                ));
            }
            entry
                .file_name()
                .into_string()
                .map_err(|_| std::io::Error::other("non-UTF-8 verifier handoff entry"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    if names != [EXECUTABLE_NAME, IDENTITY_NAME] {
        return Err(DevError::corrupt("verifier handoff inventory is not exact"));
    }
    Ok(())
}

fn parse_prepare(arguments: impl Iterator<Item = OsString>) -> Result<PrepareOptions, DevError> {
    let mut values = parse_values(
        arguments,
        &["--executable", "--output", "--tag", "--commit"],
    )?;
    Ok(PrepareOptions {
        executable: PathBuf::from(required(&mut values, "--executable")?),
        output: PathBuf::from(required(&mut values, "--output")?),
        tag: required(&mut values, "--tag")?,
        commit: required(&mut values, "--commit")?,
    })
}

fn parse_verify(arguments: impl Iterator<Item = OsString>) -> Result<VerifyOptions, DevError> {
    let mut values = parse_values(
        arguments,
        &[
            "--executable",
            "--identity",
            "--tag",
            "--commit",
            "--expected-sha256",
            "--expected-bytes",
        ],
    )?;
    let expected_sha256 = required(&mut values, "--expected-sha256")?;
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DevError::usage(
            "--expected-sha256 must be 64 lowercase hexadecimal characters",
        ));
    }
    let expected_bytes = required(&mut values, "--expected-bytes")?
        .parse::<u64>()
        .ok()
        .filter(|bytes| *bytes > 0)
        .ok_or_else(|| DevError::usage("--expected-bytes must be a positive integer"))?;
    Ok(VerifyOptions {
        executable: PathBuf::from(required(&mut values, "--executable")?),
        identity: PathBuf::from(required(&mut values, "--identity")?),
        tag: required(&mut values, "--tag")?,
        commit: required(&mut values, "--commit")?,
        expected_sha256,
        expected_bytes,
    })
}

fn parse_values(
    mut arguments: impl Iterator<Item = OsString>,
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, DevError> {
    let mut values = BTreeMap::new();
    while let Some(name) = crate::next_utf8(&mut arguments, "release verifier option")? {
        if !allowed.contains(&name.as_str()) {
            return Err(DevError::usage(format!(
                "unknown release verifier option '{name}'"
            )));
        }
        let value = crate::next_utf8(&mut arguments, "release verifier option value")?
            .ok_or_else(|| DevError::usage(format!("{name} requires a value")))?;
        if values.insert(name.clone(), value).is_some() {
            return Err(DevError::usage(format!(
                "duplicate release verifier option '{name}'"
            )));
        }
    }
    Ok(values)
}

fn required(values: &mut BTreeMap<String, String>, name: &str) -> Result<String, DevError> {
    values
        .remove(name)
        .ok_or_else(|| DevError::usage(format!("required option '{name}' is missing")))
}

fn require_absent_output(path: &Path) -> Result<(), DevError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(DevError::usage(
            "verifier handoff output must be absolute and lexically canonical",
        ));
    }
    archive::reject_existing(path, "verifier handoff output")?;
    let parent = path
        .parent()
        .ok_or_else(|| DevError::usage("verifier handoff output has no parent"))?;
    archive::ensure_directory(parent, "verifier handoff parent")?;
    if parent.canonicalize()? != parent {
        return Err(DevError::usage(
            "verifier handoff parent contains a symlink or noncanonical component",
        ));
    }
    Ok(())
}

fn validate_tag(tag: &str) -> Result<(), DevError> {
    let Some(version) = tag.strip_prefix('v') else {
        return Err(DevError::usage("verifier handoff tag must start with 'v'"));
    };
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return Err(DevError::usage(
            "verifier handoff tag must be exact vMAJOR.MINOR.PATCH",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_identity_schema_roles_and_tag_are_closed() {
        assert_eq!(IDENTITY_SCHEMA_VERSION, 3);
        assert_eq!(
            ROLES,
            [
                "release-verify",
                "distributed-http",
                "outbound-http",
                "stateful-http"
            ]
        );
        assert!(validate_tag("v0.1.8").is_ok());
        for rejected in ["0.1.8", "v01.1.8", "v0.1", "v0.1.8-rc"] {
            assert!(validate_tag(rejected).is_err());
        }
    }

    #[test]
    fn verifier_options_reject_unknown_duplicate_and_incomplete_inputs() {
        assert!(parse_prepare([].into_iter()).is_err());
        assert!(parse_prepare(["--unknown", "x"].into_iter().map(OsString::from)).is_err());
        assert!(
            parse_prepare(
                [
                    "--executable",
                    "/a",
                    "--executable",
                    "/b",
                    "--output",
                    "/c",
                    "--tag",
                    "v0.1.8",
                    "--commit",
                    "0",
                ]
                .into_iter()
                .map(OsString::from)
            )
            .is_err()
        );
    }

    #[test]
    fn verifier_handoff_round_trips_and_rejects_identity_or_inventory_mutation() {
        let temporary = tempfile::tempdir().expect("temporary verifier handoff parent");
        let output = temporary.path().join("handoff");
        let source = std::env::current_exe().expect("current test executable");
        prepare(PrepareOptions {
            executable: source,
            output: output.clone(),
            tag: "v0.1.8".to_owned(),
            commit: "0".repeat(40),
        })
        .expect("prepare verifier handoff");
        let executable = output.join(EXECUTABLE_NAME);
        let identity = output.join(IDENTITY_NAME);
        let observed = observe_executable(&executable).expect("observe verifier");
        let options = || VerifyOptions {
            executable: executable.clone(),
            identity: identity.clone(),
            tag: "v0.1.8".to_owned(),
            commit: "0".repeat(40),
            expected_sha256: observed.sha256.clone(),
            expected_bytes: observed.byte_length,
        };
        verify(options()).expect("verify exact handoff");

        let identity_bytes = fs::read(&identity).expect("read identity");
        let mut noncanonical = identity_bytes.clone();
        noncanonical.push(b'\n');
        fs::write(&identity, noncanonical).expect("write noncanonical identity");
        assert!(verify(options()).is_err());
        fs::write(&identity, identity_bytes).expect("restore identity");

        fs::write(output.join("extra"), b"unexpected").expect("write extra inventory entry");
        assert!(verify(options()).is_err());
    }

    #[test]
    fn verifier_handoff_requires_absolute_create_new_output() {
        assert!(require_absent_output(Path::new("relative")).is_err());
        let temporary = tempfile::tempdir().expect("temporary output parent");
        let existing = temporary.path().join("existing");
        fs::create_dir(&existing).expect("create existing output");
        assert!(require_absent_output(&existing).is_err());
    }
}
