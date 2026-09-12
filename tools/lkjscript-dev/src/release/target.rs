use super::archive;
use super::model::{ElfIdentity, SchemaIdentity};
use crate::error::DevError;
use crate::evidence;
use crate::process::{self, ProcessSpec, ProcessStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) use lkjscript::release_container::ARCHIVE_NAME;
pub(super) use lkjscript::release_container::TARGET_TRIPLE;
pub(super) const TAR_NAME: &str = "lkjscript-x86_64-unknown-linux-musl.tar";
pub(super) use lkjscript::release_container::ELF_INSPECTOR;
pub(super) use lkjscript::release_container::LINKAGE_MODEL;
pub(super) const POLICY_SCHEMA: &str = "lkjscript-release-target-policy";
pub(super) const POLICY_SCHEMA_VERSION: u32 = 2;
pub(super) const BUILD_SCHEMA: &str = "lkjscript-target-build-receipt";
pub(super) const BUILD_SCHEMA_VERSION: u32 = 1;

const MAXIMUM_CANDIDATE_BYTES: u64 = 256 * 1024 * 1024;
const BUILD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const MUSL_PACKAGE_VERSION: &str = "1.2.4-2";
const MUSL_PACKAGE_BASE: &str = "https://archive.ubuntu.com/ubuntu/pool/universe/m/musl";

pub(super) const MUSL_USERLAND_IMAGE: &str =
    "alpine@sha256:7c8cb692ae09657cbc4a3f3cbd0e8d5a2690ba38386aaaf252dbb060bf5eb2e6";
pub(super) const OLDER_GLIBC_USERLAND_IMAGE: &str =
    "debian@sha256:70509c95d1857a3704c0a5d92ee2e0adac95f612a9386889d70760bfd7c1ebba";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MuslPackage {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) architecture: String,
    pub(super) file_name: String,
    pub(super) url: String,
    pub(super) sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UserlandPolicy {
    pub(super) role: String,
    pub(super) image: String,
    pub(super) operating_system: String,
    pub(super) architecture: String,
    pub(super) expected_libc: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TargetPolicy {
    pub(super) schema: SchemaIdentity,
    pub(super) target_triple: String,
    pub(super) archive_name: String,
    pub(super) runtime_linkage: String,
    pub(super) elf_inspector: String,
    pub(super) rust_toolchain: String,
    pub(super) linker: String,
    pub(super) c_compiler: String,
    pub(super) musl_packages: Vec<MuslPackage>,
    pub(super) userlands: Vec<UserlandPolicy>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BuiltCandidate {
    pub(super) path: String,
    pub(super) byte_length: u64,
    pub(super) mode: u32,
    pub(super) sha256: String,
    pub(super) elf: ElfIdentity,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TargetBuildReceipt {
    pub(super) schema: SchemaIdentity,
    pub(super) source_commit: String,
    pub(super) target_policy_sha256: String,
    pub(super) started_unix_nanoseconds: u128,
    pub(super) completed_unix_nanoseconds: u128,
    pub(super) elapsed_nanoseconds: u64,
    pub(super) command: Vec<String>,
    pub(super) build_process: crate::process::ProcessObservation,
    pub(super) rustc: String,
    pub(super) cargo: String,
    pub(super) musl_gcc: String,
    pub(super) musl_gcc_dumpmachine: String,
    pub(super) installed_musl_packages: Vec<String>,
    pub(super) candidate: BuiltCandidate,
}

#[derive(Debug)]
struct BuildOptions {
    output: PathBuf,
    receipt: PathBuf,
}

pub(super) fn policy() -> TargetPolicy {
    TargetPolicy {
        schema: SchemaIdentity {
            identity: POLICY_SCHEMA.to_owned(),
            version: POLICY_SCHEMA_VERSION,
        },
        target_triple: TARGET_TRIPLE.to_owned(),
        archive_name: ARCHIVE_NAME.to_owned(),
        runtime_linkage: LINKAGE_MODEL.to_owned(),
        elf_inspector: ELF_INSPECTOR.to_owned(),
        rust_toolchain: super::TOOLCHAIN_CHANNEL.to_owned(),
        linker: "musl-gcc".to_owned(),
        c_compiler: "musl-gcc".to_owned(),
        musl_packages: musl_packages(),
        userlands: vec![
            UserlandPolicy {
                role: "musl".to_owned(),
                image: MUSL_USERLAND_IMAGE.to_owned(),
                operating_system: "linux".to_owned(),
                architecture: "amd64".to_owned(),
                expected_libc: "musl-1.2".to_owned(),
            },
            UserlandPolicy {
                role: "older-glibc".to_owned(),
                image: OLDER_GLIBC_USERLAND_IMAGE.to_owned(),
                operating_system: "linux".to_owned(),
                architecture: "amd64".to_owned(),
                expected_libc: "glibc-2.31".to_owned(),
            },
        ],
    }
}

fn musl_packages() -> Vec<MuslPackage> {
    [
        (
            "musl",
            "musl_1.2.4-2_amd64.deb",
            "9f0883c20b4b746e05e947bafd99cb933f5494ffaaa6fcd360cbe1fbcf264883",
        ),
        (
            "musl-dev",
            "musl-dev_1.2.4-2_amd64.deb",
            "4b451ecb6a0f8469883058cf22a807f3bd9cc16d115cc08b7efc35fe8eb44db2",
        ),
        (
            "musl-tools",
            "musl-tools_1.2.4-2_amd64.deb",
            "46c01d212d3eb3a1322693089037f0a5c92383a089d39c392db3c86c19ffb229",
        ),
    ]
    .into_iter()
    .map(|(name, file_name, sha256)| MuslPackage {
        name: name.to_owned(),
        version: MUSL_PACKAGE_VERSION.to_owned(),
        architecture: "amd64".to_owned(),
        file_name: file_name.to_owned(),
        url: format!("{MUSL_PACKAGE_BASE}/{file_name}"),
        sha256: sha256.to_owned(),
    })
    .collect()
}

pub(super) fn policy_sha256() -> Result<String, DevError> {
    let bytes = archive::canonical_json(&policy())?;
    Ok(archive::sha256_bytes(&bytes)?.as_str().to_owned())
}

pub(super) fn print_policy(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let mut arguments = arguments;
    if crate::next_utf8(&mut arguments, "release target option")?.is_some() {
        return Err(DevError::usage("release target accepts no options"));
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "status": "passed",
            "policy_sha256": policy_sha256()?,
            "policy": policy(),
        }))
        .map_err(|error| {
            DevError::infrastructure(format!("encode release target policy: {error}"))
        })?
    );
    Ok(0)
}

pub(super) fn build(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse_build_options(arguments)?;
    require_create_new_absolute(&options.output, "target build output")?;
    require_create_new_absolute(&options.receipt, "target build receipt")?;
    let repository = super::repository_root()?;
    super::ensure_clean_checkout(&repository)?;
    let source_commit = super::command_text("git", &["rev-parse", "HEAD"], &repository, 1024)?;
    super::validate_git_sha(&source_commit, "target build source commit")?;
    let started = Instant::now();
    let started_unix_nanoseconds = unix_nanoseconds()?;
    let parent = options
        .output
        .parent()
        .ok_or_else(|| DevError::usage("target build output has no parent"))?;
    archive::ensure_directory(parent, "target build output parent")?;
    let receipt_parent = options
        .receipt
        .parent()
        .ok_or_else(|| DevError::usage("target build receipt has no parent"))?;
    archive::ensure_directory(receipt_parent, "target build receipt parent")?;
    let stdout = options.receipt.with_extension("cargo.stdout.log");
    let stderr = options.receipt.with_extension("cargo.stderr.log");
    archive::reject_existing(&stdout, "target build stdout log")?;
    archive::reject_existing(&stderr, "target build stderr log")?;
    let command = vec![
        "cargo".to_owned(),
        "build".to_owned(),
        "--release".to_owned(),
        "--locked".to_owned(),
        "--bin".to_owned(),
        "lkjscript".to_owned(),
        "--target".to_owned(),
        TARGET_TRIPLE.to_owned(),
    ];
    let mut environment = process::environment();
    environment.insert(
        "CC_x86_64_unknown_linux_musl".to_owned(),
        "musl-gcc".to_owned(),
    );
    environment.insert(
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER".to_owned(),
        "musl-gcc".to_owned(),
    );
    let observation = process::run(
        &ProcessSpec {
            command: command.clone(),
            cwd: repository.clone(),
            environment,
            timeout: BUILD_TIMEOUT,
            maximum_stdout_bytes: 8 * 1024 * 1024,
            maximum_stderr_bytes: 32 * 1024 * 1024,
            stdout_path: stdout,
            stderr_path: stderr,
            unavailable_exit_code: None,
        },
        receipt_parent,
    );
    if observation.status != ProcessStatus::Passed {
        return Err(DevError::infrastructure(format!(
            "static target build ended as {:?}: {}",
            observation.status,
            observation.reason.as_deref().unwrap_or("no reason")
        )));
    }
    let built = repository
        .join("target")
        .join(TARGET_TRIPLE)
        .join("release/lkjscript");
    let built_candidate = observe_candidate(&built)?;
    archive::copy_new(&built, &options.output, 0o755)?;
    let output_candidate = observe_candidate(&options.output)?;
    if built_candidate.byte_length != output_candidate.byte_length
        || built_candidate.sha256 != output_candidate.sha256
        || built_candidate.elf != output_candidate.elf
    {
        return Err(DevError::infrastructure(
            "copied static target output disagrees with the built candidate",
        ));
    }
    let rustc = super::command_text("rustc", &["-Vv"], &repository, 16 * 1024)?;
    let cargo = super::command_text("cargo", &["-V"], &repository, 4096)?;
    let musl_gcc = super::command_text("musl-gcc", &["--version"], &repository, 64 * 1024)?;
    let musl_gcc_dumpmachine =
        super::command_text("musl-gcc", &["-dumpmachine"], &repository, 4096)?;
    let installed_musl_packages = installed_musl_packages(&repository)?;
    let receipt = TargetBuildReceipt {
        schema: SchemaIdentity {
            identity: BUILD_SCHEMA.to_owned(),
            version: BUILD_SCHEMA_VERSION,
        },
        source_commit,
        target_policy_sha256: policy_sha256()?,
        started_unix_nanoseconds,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        command,
        build_process: observation,
        rustc,
        cargo,
        musl_gcc,
        musl_gcc_dumpmachine,
        installed_musl_packages,
        candidate: output_candidate,
    };
    validate_build_receipt(&receipt, &options.output, &options.receipt)?;
    archive::write_new(&options.receipt, &archive::canonical_json(&receipt)?, 0o644)?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "status": "passed",
            "target": TARGET_TRIPLE,
            "runtime_linkage": LINKAGE_MODEL,
            "candidate": options.output,
            "candidate_bytes": receipt.candidate.byte_length,
            "candidate_sha256": receipt.candidate.sha256,
            "receipt": options.receipt,
            "target_policy_sha256": receipt.target_policy_sha256,
        }))
        .map_err(|error| DevError::infrastructure(format!(
            "encode target build summary: {error}"
        )))?
    );
    Ok(0)
}

pub(super) fn inspect_static_elf(path: &Path) -> Result<ElfIdentity, DevError> {
    let metadata = archive::ensure_regular(path, "static ELF candidate")?;
    if metadata.len() == 0 || metadata.len() > MAXIMUM_CANDIDATE_BYTES {
        return Err(DevError::corrupt(format!(
            "static ELF candidate has an invalid byte length {}",
            metadata.len()
        )));
    }
    let bytes = fs::read(path)
        .map_err(|error| DevError::infrastructure(format!("read static ELF candidate: {error}")))?;
    Ok(lkjscript::release_container::inspect_static_elf_bytes(
        &bytes,
    )?)
}

#[cfg(test)]
use lkjscript::release_container::inspect_static_elf_bytes;

pub(super) fn observe_candidate(path: &Path) -> Result<BuiltCandidate, DevError> {
    let metadata = archive::ensure_regular(path, "static target candidate")?;
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage("static target candidate is not executable"));
    }
    let (sha256, byte_length) = archive::sha256_file(path)?;
    Ok(BuiltCandidate {
        path: path.display().to_string(),
        byte_length,
        mode: metadata.permissions().mode() & 0o7777,
        sha256: sha256.as_str().to_owned(),
        elf: inspect_static_elf(path)?,
    })
}

pub(super) fn read_build_receipt(
    path: &Path,
    candidate: &Path,
) -> Result<TargetBuildReceipt, DevError> {
    let bytes = fs::read(path)
        .map_err(|error| DevError::infrastructure(format!("read target build receipt: {error}")))?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(DevError::corrupt("target build receipt exceeds 4 MiB"));
    }
    let receipt: TargetBuildReceipt = serde_json::from_slice(&bytes)
        .map_err(|error| DevError::corrupt(format!("decode target build receipt: {error}")))?;
    if archive::canonical_json(&receipt)? != bytes {
        return Err(DevError::corrupt(
            "target build receipt is not in canonical first-party encoding",
        ));
    }
    validate_build_receipt(&receipt, candidate, path)?;
    Ok(receipt)
}

fn validate_build_receipt(
    receipt: &TargetBuildReceipt,
    candidate: &Path,
    receipt_path: &Path,
) -> Result<(), DevError> {
    let observed = observe_candidate(candidate)?;
    if receipt.schema.identity != BUILD_SCHEMA
        || receipt.schema.version != BUILD_SCHEMA_VERSION
        || receipt.target_policy_sha256 != policy_sha256()?
        || receipt.command
            != [
                "cargo",
                "build",
                "--release",
                "--locked",
                "--bin",
                "lkjscript",
                "--target",
                TARGET_TRIPLE,
            ]
        || receipt.build_process.status != ProcessStatus::Passed
        || receipt.build_process.exit_code != Some(0)
        || receipt.build_process.stdout_limit_exhausted
        || receipt.build_process.stderr_limit_exhausted
        || receipt.musl_gcc_dumpmachine != "x86_64-linux-gnu"
        || receipt.completed_unix_nanoseconds < receipt.started_unix_nanoseconds
        || receipt.candidate.byte_length != observed.byte_length
        || receipt.candidate.mode != 0o755
        || receipt.candidate.sha256 != observed.sha256
        || receipt.candidate.elf != observed.elf
    {
        return Err(DevError::corrupt("target build receipt binding mismatch"));
    }
    validate_process_log(
        receipt_path,
        &receipt.build_process.stdout,
        "target build stdout",
    )?;
    validate_process_log(
        receipt_path,
        &receipt.build_process.stderr,
        "target build stderr",
    )?;
    super::validate_git_sha(&receipt.source_commit, "target build receipt source commit")?;
    Ok(())
}

fn validate_process_log(
    receipt_path: &Path,
    expected: &crate::evidence::FileProof,
    label: &str,
) -> Result<(), DevError> {
    let parent = receipt_path
        .parent()
        .ok_or_else(|| DevError::corrupt("target build receipt has no parent"))?;
    let relative = Path::new(&expected.path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|item| matches!(item, Component::CurDir | Component::ParentDir))
    {
        return Err(DevError::corrupt(format!(
            "{label} proof path is not canonical and relative"
        )));
    }
    let observed = evidence::proof(&parent.join(relative), expected.path.clone())?;
    if &observed != expected {
        return Err(DevError::corrupt(format!(
            "{label} proof changed after the target build"
        )));
    }
    Ok(())
}

fn installed_musl_packages(repository: &Path) -> Result<Vec<String>, DevError> {
    let packages = super::command_text(
        "dpkg-query",
        &[
            "-W",
            "-f=${Package}=${Version}:${Architecture}\\n",
            "musl",
            "musl-dev",
            "musl-tools",
        ],
        repository,
        16 * 1024,
    )?;
    let observed = packages.lines().map(str::to_owned).collect::<Vec<_>>();
    let expected = vec![
        format!("musl={MUSL_PACKAGE_VERSION}:amd64"),
        format!("musl-dev={MUSL_PACKAGE_VERSION}:amd64"),
        format!("musl-tools={MUSL_PACKAGE_VERSION}:amd64"),
    ];
    if observed != expected {
        return Err(DevError::corrupt(format!(
            "installed musl packages disagree with target policy: observed {observed:?}"
        )));
    }
    Ok(observed)
}

fn parse_build_options(
    mut arguments: impl Iterator<Item = OsString>,
) -> Result<BuildOptions, DevError> {
    let mut values = BTreeMap::new();
    while let Some(argument) = crate::next_utf8(&mut arguments, "release build option")? {
        let name = match argument.as_str() {
            "--output" | "--receipt" => argument,
            value => {
                return Err(DevError::usage(format!(
                    "unknown release build option '{value}'"
                )));
            }
        };
        let value = crate::next_utf8(&mut arguments, &format!("value for {name}"))?
            .ok_or_else(|| DevError::usage(format!("{name} requires a value")))?;
        if values.insert(name.clone(), value).is_some() {
            return Err(DevError::usage(format!("duplicate option '{name}'")));
        }
    }
    Ok(BuildOptions {
        output: PathBuf::from(
            values
                .remove("--output")
                .ok_or_else(|| DevError::usage("required option '--output' is missing"))?,
        ),
        receipt: PathBuf::from(
            values
                .remove("--receipt")
                .ok_or_else(|| DevError::usage("required option '--receipt' is missing"))?,
        ),
    })
}

fn require_create_new_absolute(path: &Path, label: &str) -> Result<(), DevError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(DevError::usage(format!(
            "{label} '{}' must be absolute and lexically canonical",
            path.display()
        )));
    }
    archive::reject_existing(path, label)?;
    let parent = path
        .parent()
        .ok_or_else(|| DevError::usage(format!("{label} has no parent")))?;
    archive::ensure_directory(parent, &format!("{label} parent"))?;
    let canonical = parent.canonicalize().map_err(|error| {
        DevError::infrastructure(format!(
            "resolve {label} parent '{}': {error}",
            parent.display()
        ))
    })?;
    if canonical != parent {
        return Err(DevError::usage(format!(
            "{label} parent '{}' contains a symlink or noncanonical component",
            parent.display()
        )));
    }
    Ok(())
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock before epoch: {error}")))
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::release) fn elf_fixture(dynamic_tags: &[(i64, u64)]) -> Vec<u8> {
        let program_offset = 64_usize;
        let dynamic_offset = program_offset + 2 * 56;
        let dynamic_bytes = dynamic_tags.len() * 16;
        let mut bytes = vec![0_u8; dynamic_offset + dynamic_bytes];
        bytes[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&3_u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&62_u16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[32..40].copy_from_slice(&(program_offset as u64).to_le_bytes());
        bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
        bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&2_u16.to_le_bytes());
        bytes[program_offset..program_offset + 4].copy_from_slice(&1_u32.to_le_bytes());
        let file_length = bytes.len() as u64;
        bytes[program_offset + 32..program_offset + 40].copy_from_slice(&file_length.to_le_bytes());
        let dynamic = program_offset + 56;
        bytes[dynamic..dynamic + 4].copy_from_slice(&2_u32.to_le_bytes());
        bytes[dynamic + 8..dynamic + 16].copy_from_slice(&(dynamic_offset as u64).to_le_bytes());
        bytes[dynamic + 32..dynamic + 40].copy_from_slice(&(dynamic_bytes as u64).to_le_bytes());
        for (index, (tag, value)) in dynamic_tags.iter().enumerate() {
            let offset = dynamic_offset + index * 16;
            bytes[offset..offset + 8].copy_from_slice(&tag.to_le_bytes());
            bytes[offset + 8..offset + 16].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn target_policy_has_one_static_target_and_pinned_inputs() {
        let policy = policy();
        assert_eq!(policy.target_triple, TARGET_TRIPLE);
        assert_eq!(policy.archive_name, ARCHIVE_NAME);
        assert_eq!(policy.runtime_linkage, LINKAGE_MODEL);
        assert_eq!(policy.musl_packages.len(), 3);
        assert_eq!(policy.userlands.len(), 2);
        assert!(policy.musl_packages.iter().all(|package| {
            package.version == MUSL_PACKAGE_VERSION && package.sha256.len() == 64
        }));
        assert!(!ARCHIVE_NAME.contains("unknown-linux-gnu"));
    }

    #[test]
    fn target_policy_summary_binds_the_canonical_policy_digest() {
        let policy = policy();
        let bytes = archive::canonical_json(&policy).expect("canonical target policy");
        let expected = archive::sha256_bytes(&bytes).expect("target policy digest");
        assert_eq!(policy_sha256().expect("policy SHA-256"), expected.as_str());
    }

    #[test]
    fn static_elf_inspection_accepts_static_pie_and_rejects_dynamic_or_malformed_inputs() {
        let valid = elf_fixture(&[(21, 0), (0, 0)]);
        let identity = inspect_static_elf_bytes(&valid).expect("static PIE fixture");
        assert_eq!(identity.runtime_linkage, LINKAGE_MODEL);
        assert_eq!(identity.interpreter_headers, 0);
        assert_eq!(identity.needed_libraries, 0);
        assert_eq!(identity.glibc_version_requirements, 0);

        assert!(inspect_static_elf_bytes(&elf_fixture(&[(1, 1), (0, 0)])).is_err());
        assert!(inspect_static_elf_bytes(&elf_fixture(&[(0x6fff_fffe, 1), (0, 0)])).is_err());
        assert!(inspect_static_elf_bytes(&elf_fixture(&[(0, 0), (21, 1)])).is_err());
        assert!(inspect_static_elf_bytes(&valid[..63]).is_err());
        let mut foreign = valid.clone();
        foreign[18..20].copy_from_slice(&183_u16.to_le_bytes());
        assert!(inspect_static_elf_bytes(&foreign).is_err());
        let mut non_elf = valid;
        non_elf[0] = 0;
        assert!(inspect_static_elf_bytes(&non_elf).is_err());
    }

    #[test]
    fn static_elf_inspection_rejects_an_interpreter_header() {
        let mut dynamic = elf_fixture(&[(21, 0), (0, 0)]);
        let second_program = 64 + 56;
        dynamic[second_program..second_program + 4].copy_from_slice(&3_u32.to_le_bytes());
        assert!(inspect_static_elf_bytes(&dynamic).is_err());
    }

    #[test]
    fn build_options_are_closed_and_require_create_new_outputs() {
        let parsed = parse_build_options(
            ["--output", "/tmp/candidate", "--receipt", "/tmp/receipt"]
                .into_iter()
                .map(OsString::from),
        )
        .expect("target build options");
        assert_eq!(parsed.output, PathBuf::from("/tmp/candidate"));
        assert!(
            parse_build_options(
                ["--output", "/tmp/candidate", "--unknown", "value"]
                    .into_iter()
                    .map(OsString::from)
            )
            .is_err()
        );
    }
}
