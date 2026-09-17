//! Owning-context original admission. Successful summaries do not replace ordered commands,
//! their retained outputs, independent expected results, or completed resource cleanup.
use super::*;

struct Originals<'a> {
    root: &'a Path,
    commands: &'a [CommandEvidence],
    next: usize,
}

fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}

impl Originals<'_> {
    fn take(
        &mut self,
        name: &str,
        command: Vec<String>,
        exit: i32,
        timeout: Duration,
    ) -> Result<Vec<u8>, DevError> {
        let ordinal = self.next;
        let observed = self.commands.get(ordinal).ok_or_else(|| {
            DevError::corrupt(format!("target original omitted required command '{name}'"))
        })?;
        self.next += 1;
        let process = &observed.process;
        require(
            observed.name == name
                && observed.command == command
                && observed.expected == format!("exit-{exit}")
                && process.status
                    == if exit == 0 {
                        ProcessStatus::Passed
                    } else {
                        ProcessStatus::Failed
                    }
                && process.exit_code == Some(exit)
                && process.signal.is_none()
                && process.reason.as_deref()
                    == if exit == 0 {
                        None
                    } else {
                        Some("nonzero_exit")
                    }
                && !process.stdout_limit_exhausted
                && !process.stderr_limit_exhausted
                && process.stdout_limit_bytes == MAXIMUM_COMMAND_OUTPUT_BYTES
                && process.stderr_limit_bytes == MAXIMUM_COMMAND_OUTPUT_BYTES
                && u128::from(process.elapsed_nanoseconds) <= timeout.as_nanos(),
            "target original command, outcome, deadline or output bound differs",
        )?;
        for (stream, proof) in [("stdout", &process.stdout), ("stderr", &process.stderr)] {
            let name = format!("{ordinal:04}-{}.{stream}.log", safe_name(name)?);
            let path = self.root.join(&name);
            archive::ensure_regular(&path, "target original command log")?;
            require(
                proof.path == name && evidence::proof(&path, name)? == *proof,
                "target original log bytes, mode or identity changed",
            )?;
        }
        let bytes = process::read_bounded(
            &self.root.join(&process.stdout.path),
            MAXIMUM_COMMAND_OUTPUT_BYTES,
        )?;
        require(
            process.stdout.digest.as_ref() == Some(&evidence::VerificationDigest::of(&bytes)),
            "target original stdout changed while admitted",
        )?;
        Ok(bytes)
    }

    fn candidate(
        &mut self,
        policy: &UserlandPolicy,
        rootfs: &Path,
        name: &str,
        arguments: &[&str],
        exit: i32,
    ) -> Result<Vec<CompactRecord>, DevError> {
        let name = format!("{}-candidate-{name}", policy.role);
        let bytes = self.take(
            &name,
            candidate_invocation(rootfs, arguments),
            exit,
            COMMAND_TIMEOUT,
        )?;
        let records = compact(&name, &bytes)?;
        require(
            !records.is_empty(),
            "target candidate command omitted its public result",
        )?;
        Ok(records)
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| (*s).to_owned()).collect()
}

pub(super) fn verify(
    path: &Path,
    receipt: &TargetAdmissionReceipt,
    candidate: &Path,
    verifier: &Path,
) -> Result<(), DevError> {
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("target receipt parent absent"))?;
    require(
        root.canonicalize()? == root,
        "target originals are outside their canonical producing root",
    )?;
    let mut originals = Originals {
        root,
        commands: &receipt.commands,
        next: 0,
    };
    for (policy, observed) in target::policy().userlands.iter().zip(&receipt.userlands) {
        userland(&mut originals, policy, observed, &receipt.candidate)?;
    }
    for role in ORACLES {
        let root = root.join(role.name());
        originals.take(
            role.name(),
            vec![
                verifier.display().to_string(),
                role.name().to_owned(),
                "--binary".to_owned(),
                candidate.display().to_string(),
                "--evidence-root".to_owned(),
                root.display().to_string(),
                "--machine".to_owned(),
            ],
            0,
            role.timeout(),
        )?;
    }
    let summary = originals.take(
        "service-oracle",
        vec![
            verifier.display().to_string(),
            "service".to_owned(),
            "--binary".to_owned(),
            candidate.display().to_string(),
            "--machine".to_owned(),
        ],
        0,
        ORACLE_TIMEOUT,
    )?;
    let value = require_machine_passed("service", &summary)?;
    let recorded = receipt
        .oracles
        .last()
        .ok_or_else(|| DevError::corrupt("service original absent"))?;
    let repository = super::super::repository_root()?;
    let expected = evidence::relative(&repository, Path::new(&recorded.receipt.path));
    require(
        value.get("receipt").and_then(Value::as_str) == Some(expected.as_str()),
        "target service summary substituted its owning receipt",
    )?;
    require(
        originals.next == originals.commands.len(),
        "target originals contain extra or reordered commands",
    )
}

fn userland(
    originals: &mut Originals<'_>,
    policy: &UserlandPolicy,
    observed: &UserlandObservation,
    candidate: &BuiltCandidate,
) -> Result<(), DevError> {
    require(
        observed.role == policy.role
            && observed.expected_libc == policy.expected_libc
            && observed.candidate_sha256 == candidate.sha256
            && observed.candidate_mode == 0o755
            && observed.network_policy == "sudo-unshare-network-namespace-no-host-library-mounts"
            && observed.candidate_commands == 12
            && observed.product_version == lkjscript::PRODUCT_VERSION
            && observed.execution_value == "\"hello\""
            && observed.differential_equal
            && observed.container_cleanup_complete
            && observed.temporary_root_removed
            && observed.rootfs_archive_bytes > 0
            && observed.rootfs_archive_bytes <= MAXIMUM_ROOTFS_ARCHIVE_BYTES,
        "target userland proof omitted a required environment, workload, result or cleanup property",
    )?;
    super::super::model::Sha256Digest::new(observed.rootfs_archive_sha256.clone())
        .map_err(DevError::corrupt)?;
    super::super::validate_capabilities_digest(&observed.capabilities_digest)?;
    let temporary = Path::new(&observed.temporary_root);
    require(
        temporary.is_absolute()
            && temporary.parent() == Some(originals.root)
            && temporary
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|name| name.starts_with(&format!(".{}-userland-", policy.role)))
            && matches!(fs::symlink_metadata(temporary), Err(error) if error.kind() == std::io::ErrorKind::NotFound),
        "target temporary root ownership or completed cleanup differs",
    )?;
    let rootfs = temporary.join("rootfs");
    let exported = temporary.join("rootfs.tar");
    originals.take(
        &format!("{}-image-pull", policy.role),
        strings(&["docker", "pull", "--platform", "linux/amd64", &policy.image]),
        0,
        IMAGE_TIMEOUT,
    )?;
    let inspect = originals.take(
        &format!("{}-image-inspect", policy.role),
        strings(&["docker", "image", "inspect", &policy.image]),
        0,
        COMMAND_TIMEOUT,
    )?;
    require(
        inspect_image(&inspect, policy)? == observed.image,
        "target image original differs from its pinned userland observation",
    )?;
    let owner = cleanup::read(originals.root, &policy.role)?
        .ok_or_else(|| DevError::corrupt("successful target omitted container ownership intent"))?;
    let id = cleanup::cid(originals.root, &owner)?
        .ok_or_else(|| DevError::corrupt("successful target omitted its exact container CID"))?;
    let create = originals.take(
        &format!("{}-container-create", policy.role),
        cleanup::create_command(originals.root, &owner, &policy.image),
        0,
        COMMAND_TIMEOUT,
    )?;
    require(
        std::str::from_utf8(&create).is_ok_and(|s| s.trim_end_matches('\n') == id),
        "Docker create original does not bind the recorded CID",
    )?;
    originals.take(
        &format!("{}-container-export", policy.role),
        vec![
            "docker".into(),
            "export".into(),
            "--output".into(),
            exported.display().to_string(),
            id,
        ],
        0,
        IMAGE_TIMEOUT,
    )?;
    let resource_root = originals.root;
    cleanup::remove_with(resource_root, &owner, |name, command| {
        originals.take(name, command, 0, COMMAND_TIMEOUT)
    })?;
    originals.take(
        &format!("{}-rootfs-extract", policy.role),
        vec![
            "tar".into(),
            "--extract".into(),
            "--file".into(),
            exported.display().to_string(),
            "--directory".into(),
            rootfs.display().to_string(),
            "--no-same-owner".into(),
            "--no-same-permissions".into(),
        ],
        0,
        IMAGE_TIMEOUT,
    )?;

    let os_path = originals.root.join(format!("{}-os-release", policy.role));
    archive::ensure_regular(&os_path, "userland os-release original")?;
    let os_bytes = process::read_bounded(&os_path, 64 * 1024)?;
    let os_lines = std::str::from_utf8(&os_bytes)
        .map_err(|_| DevError::corrupt("userland os-release original is not UTF-8"))?
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let expected_id = if policy.role == "musl" {
        "ID=alpine"
    } else {
        "ID=debian"
    };
    require(
        archive::sha256_bytes(&os_bytes)?.as_str() == observed.os_release_sha256
            && os_lines == observed.os_release
            && os_lines.iter().any(|line| line == expected_id),
        "target os-release original, digest or expected environment differs",
    )?;

    let capabilities =
        originals.candidate(policy, &rootfs, "capabilities", &["capabilities"], 0)?;
    require_field(&capabilities, "product", "name", "lkjscript")?;
    require_field(
        &capabilities,
        "product",
        "version",
        lkjscript::PRODUCT_VERSION,
    )?;
    require_field(
        &capabilities,
        "capabilities",
        "digest",
        &observed.capabilities_digest,
    )?;
    let project = "/work/application";
    let created = originals.candidate(
        policy,
        &rootfs,
        "new-command-project",
        &[
            "new",
            project,
            "--template",
            "command",
            "--name",
            "admission",
        ],
        0,
    )?;
    require_field(&created, "project", "template", "command")?;
    let initial = required_field(required_record(&created, "revision")?, "id")?;
    require(
        initial == observed.initial_revision && !initial.is_empty(),
        "initial userland revision original differs",
    )?;
    let initial_status = originals.candidate(
        policy,
        &rootfs,
        "status-initial",
        &["--project", project, "status"],
        0,
    )?;
    require_field(&initial_status, "revision", "id", initial)?;
    let owners = originals.candidate(
        policy,
        &rootfs,
        "query-pure-function",
        &[
            "--project",
            project,
            "query",
            "owners",
            "--kind",
            "pure_function",
            "--limit",
            "20",
        ],
        0,
    )?;
    let owner = required_record(&owners, "owner")?;
    require_exact(
        required_field(owner, "name")?,
        "greet",
        "independent command recipe function",
    )?;
    let owner_id = required_field(owner, "id")?;
    let plan = originals.candidate(
        policy,
        &rootfs,
        "change-plan",
        &[
            "--project",
            project,
            "change",
            "plan",
            "rename.owner",
            "--base",
            initial,
            "--owner",
            owner_id,
            "--name",
            "greet-admitted",
            "--output",
            "/work/rename.logical-plan",
        ],
        0,
    )?;
    let token = required_field(required_record(&plan, "plan")?, "token")?;
    originals.candidate(
        policy,
        &rootfs,
        "change-apply",
        &[
            "--project",
            project,
            "change",
            "apply",
            "rename.owner",
            "--base",
            initial,
            "--owner",
            owner_id,
            "--name",
            "greet-admitted",
            "--plan",
            token,
        ],
        0,
    )?;
    let accepted = originals.candidate(
        policy,
        &rootfs,
        "status-accepted",
        &["--project", project, "status"],
        0,
    )?;
    let revision = required_field(required_record(&accepted, "revision")?, "id")?;
    require(
        revision == observed.accepted_revision && revision != initial,
        "reviewed userland edit did not advance its exact original revision",
    )?;
    let checked = originals.candidate(
        policy,
        &rootfs,
        "check",
        &["--project", project, "check"],
        0,
    )?;
    require_field(&checked, "tests", "failed", "0")?;
    require_field(&checked, "tests", "differential", "equal")?;
    require(
        required_field(required_record(&checked, "tests")?, "passed")?
            .parse::<u64>()
            .is_ok_and(|count| count > 0),
        "userland source check omitted its independent recipe assertion",
    )?;
    originals.candidate(
        policy,
        &rootfs,
        "build-incremental",
        &[
            "--project",
            project,
            "build",
            "--output",
            "/work/application.lkja",
        ],
        0,
    )?;
    originals.candidate(
        policy,
        &rootfs,
        "build-clean",
        &[
            "--project",
            project,
            "build",
            "--output",
            "/work/application-clean.lkja",
        ],
        0,
    )?;
    for kind in ["incremental", "clean"] {
        let path = originals.root.join(format!("{}-{kind}.lkja", policy.role));
        let metadata = archive::ensure_regular(&path, "userland artifact original")?;
        require(
            metadata.len() > 0
                && metadata.len() <= MAXIMUM_ROOTFS_ARCHIVE_BYTES
                && metadata.permissions().mode() & 0o7777 == 0o644,
            "userland artifact original is empty, oversized or has the wrong mode",
        )?;
        let (digest, length) = archive::sha256_file(&path)?;
        require(
            digest.as_str() == observed.artifact_sha256
                && digest.as_str() == observed.clean_artifact_sha256
                && length == observed.artifact_bytes,
            "retained clean/incremental artifacts differ from their compared original bytes",
        )?;
    }
    let run = originals.candidate(
        policy,
        &rootfs,
        "run-main",
        &["--project", project, "run", "main"],
        0,
    )?;
    require_field(&run, "execution", "value", "\"hello\"")?;
    require_field(&run, "execution", "differential", "equal")?;
    let rejected = originals.candidate(
        policy,
        &rootfs,
        "unknown-operation-rejected",
        &["unknown-operation"],
        2,
    )?;
    require_field(&rejected, "result", "status", "failure")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn retain(
        root: &Path,
        commands: &mut Vec<CommandEvidence>,
        name: &str,
        command: Vec<String>,
        output: &[u8],
        exit: i32,
    ) {
        let mut streams = Vec::new();
        for (stream, bytes) in [("stdout", output), ("stderr", &b""[..])] {
            let relative = format!("{:04}-{name}.{stream}.log", commands.len());
            let path = root.join(&relative);
            fs::write(&path, bytes).expect("independent retained original");
            streams.push(evidence::proof(&path, relative).expect("original proof"));
        }
        commands.push(CommandEvidence {
            name: name.to_owned(),
            command,
            expected: format!("exit-{exit}"),
            process: ProcessObservation {
                status: if exit == 0 {
                    ProcessStatus::Passed
                } else {
                    ProcessStatus::Failed
                },
                exit_code: Some(exit),
                signal: None,
                reason: if exit == 0 {
                    None
                } else {
                    Some("nonzero_exit".to_owned())
                },
                elapsed_nanoseconds: 1,
                cpu_nanoseconds: None,
                peak_rss_kib: None,
                stdout_limit_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
                stderr_limit_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
                stdout_limit_exhausted: false,
                stderr_limit_exhausted: false,
                stdout: streams.remove(0),
                stderr: streams.remove(0),
            },
        });
    }

    fn record(operation: &str, fields: &[(&str, &str)]) -> Vec<u8> {
        lkjscript::platform::control::render_record(operation, fields)
            .expect("literal public output")
            .into_bytes()
    }

    fn userland_fixture(
        root: &Path,
        policy: &UserlandPolicy,
    ) -> (Vec<CommandEvidence>, UserlandObservation, BuiltCandidate) {
        let mut commands = Vec::new();
        let owner = cleanup::prepare(root, &policy.role).expect("fixture owner intent");
        let id = "1".repeat(64);
        let options = cleanup::options(root, &owner);
        fs::write(&options[3], &id).expect("fixture Docker CID");
        let temporary = root.join(format!(".{}-userland-fixture", policy.role));
        let rootfs = temporary.join("rootfs");
        let exported = temporary.join("rootfs.tar");
        let name = |suffix: &str| format!("{}-{suffix}", policy.role);
        retain(
            root,
            &mut commands,
            &name("image-pull"),
            strings(&["docker", "pull", "--platform", "linux/amd64", &policy.image]),
            b"pulled\n",
            0,
        );
        let image_id = format!("sha256:{}", "5".repeat(64));
        let image = serde_json::to_vec(&serde_json::json!([{
            "Id": image_id, "Os": "linux", "Architecture": "amd64", "Size": 1024,
            "RepoDigests": [policy.image],
        }]))
        .expect("independent image metadata");
        retain(
            root,
            &mut commands,
            &name("image-inspect"),
            strings(&["docker", "image", "inspect", &policy.image]),
            &image,
            0,
        );
        retain(
            root,
            &mut commands,
            &name("container-create"),
            cleanup::create_command(root, &owner, &policy.image),
            format!("{id}\n").as_bytes(),
            0,
        );
        retain(
            root,
            &mut commands,
            &name("container-export"),
            vec![
                "docker".into(),
                "export".into(),
                "--output".into(),
                exported.display().to_string(),
                id.clone(),
            ],
            b"",
            0,
        );
        cleanup::remove_with(root, &owner, |name, command| {
            let output = if name.ends_with("-lookup") {
                format!("{id}\n").into_bytes()
            } else if name.ends_with("-ownership") {
                serde_json::to_vec(&serde_json::json!({
                    "id": id, "name": format!("/{}", options[1]),
                    "owner": options[5].split_once('=').expect("label assignment").1,
                }))
                .expect("ownership output")
            } else {
                Vec::new()
            };
            retain(root, &mut commands, name, command, &output, 0);
            Ok(output)
        })
        .expect("recorded successful cleanup");
        retain(
            root,
            &mut commands,
            &name("rootfs-extract"),
            vec![
                "tar".into(),
                "--extract".into(),
                "--file".into(),
                exported.display().to_string(),
                "--directory".into(),
                rootfs.display().to_string(),
                "--no-same-owner".into(),
                "--no-same-permissions".into(),
            ],
            b"",
            0,
        );
        let capability_digest = "6".repeat(64);
        let capabilities = [
            record(
                "product",
                &[
                    ("name", "lkjscript"),
                    ("version", lkjscript::PRODUCT_VERSION),
                ],
            ),
            record("capabilities", &[("digest", &capability_digest)]),
        ]
        .concat();
        let project = "/work/application";
        let success = record("result", &[("status", "success")]);
        let steps: Vec<(&str, Vec<&str>, Vec<u8>, i32)> = vec![
            ("capabilities", vec!["capabilities"], capabilities, 0),
            (
                "new-command-project",
                vec![
                    "new",
                    project,
                    "--template",
                    "command",
                    "--name",
                    "admission",
                ],
                [
                    record("project", &[("template", "command")]),
                    record("revision", &[("id", "initial")]),
                ]
                .concat(),
                0,
            ),
            (
                "status-initial",
                vec!["--project", project, "status"],
                record("revision", &[("id", "initial")]),
                0,
            ),
            (
                "query-pure-function",
                vec![
                    "--project",
                    project,
                    "query",
                    "owners",
                    "--kind",
                    "pure_function",
                    "--limit",
                    "20",
                ],
                record("owner", &[("name", "greet"), ("id", "greet-owner")]),
                0,
            ),
            (
                "change-plan",
                vec![
                    "--project",
                    project,
                    "change",
                    "plan",
                    "rename.owner",
                    "--base",
                    "initial",
                    "--owner",
                    "greet-owner",
                    "--name",
                    "greet-admitted",
                    "--output",
                    "/work/rename.logical-plan",
                ],
                record("plan", &[("token", "reviewed-token")]),
                0,
            ),
            (
                "change-apply",
                vec![
                    "--project",
                    project,
                    "change",
                    "apply",
                    "rename.owner",
                    "--base",
                    "initial",
                    "--owner",
                    "greet-owner",
                    "--name",
                    "greet-admitted",
                    "--plan",
                    "reviewed-token",
                ],
                success.clone(),
                0,
            ),
            (
                "status-accepted",
                vec!["--project", project, "status"],
                record("revision", &[("id", "accepted")]),
                0,
            ),
            (
                "check",
                vec!["--project", project, "check"],
                record(
                    "tests",
                    &[("passed", "1"), ("failed", "0"), ("differential", "equal")],
                ),
                0,
            ),
            (
                "build-incremental",
                vec![
                    "--project",
                    project,
                    "build",
                    "--output",
                    "/work/application.lkja",
                ],
                success.clone(),
                0,
            ),
            (
                "build-clean",
                vec![
                    "--project",
                    project,
                    "build",
                    "--output",
                    "/work/application-clean.lkja",
                ],
                success,
                0,
            ),
            (
                "run-main",
                vec!["--project", project, "run", "main"],
                record(
                    "execution",
                    &[("value", "\"hello\""), ("differential", "equal")],
                ),
                0,
            ),
            (
                "unknown-operation-rejected",
                vec!["unknown-operation"],
                record("result", &[("status", "failure")]),
                2,
            ),
        ];
        for (step, arguments, output, exit) in steps {
            retain(
                root,
                &mut commands,
                &name(&format!("candidate-{step}")),
                candidate_invocation(&rootfs, &arguments),
                &output,
                exit,
            );
        }
        let os = if policy.role == "musl" {
            b"ID=alpine\n".as_slice()
        } else {
            b"ID=debian\n".as_slice()
        };
        fs::write(root.join(name("os-release")), os).expect("os-release original");
        let artifact = b"independently compared artifact bytes";
        for kind in ["incremental", "clean"] {
            let path = root.join(format!("{}-{kind}.lkja", policy.role));
            fs::write(&path, artifact).expect("artifact original");
            fs::set_permissions(path, fs::Permissions::from_mode(0o644))
                .expect("independent artifact original mode");
        }
        let artifact_sha = archive::sha256_bytes(artifact)
            .expect("artifact digest")
            .as_str()
            .to_owned();
        let candidate = BuiltCandidate {
            path: root.join("candidate").display().to_string(),
            byte_length: 128,
            mode: 0o755,
            sha256: "7".repeat(64),
            elf: lkjscript::release_container::inspect_static_elf_bytes(
                &target::tests::elf_fixture(&[(0, 0)]),
            )
            .expect("independent static ELF fixture"),
        };
        let observed = UserlandObservation {
            role: policy.role.clone(),
            status: AdmissionStatus::Passed,
            image: ImageIdentity {
                requested: policy.image.clone(),
                image_id,
                repository_digests: vec![policy.image.clone()],
                operating_system: "linux".into(),
                architecture: "amd64".into(),
                virtual_size_bytes: 1024,
            },
            expected_libc: policy.expected_libc.clone(),
            os_release_sha256: archive::sha256_bytes(os)
                .expect("os digest")
                .as_str()
                .to_owned(),
            os_release: vec![
                std::str::from_utf8(os)
                    .expect("os text")
                    .trim_end()
                    .to_owned(),
            ],
            rootfs_archive_bytes: 1024,
            rootfs_archive_sha256: "8".repeat(64),
            candidate_sha256: candidate.sha256.clone(),
            candidate_mode: 0o755,
            network_policy: "sudo-unshare-network-namespace-no-host-library-mounts".into(),
            candidate_commands: 12,
            product_version: lkjscript::PRODUCT_VERSION.to_owned(),
            capabilities_digest: capability_digest,
            initial_revision: "initial".into(),
            accepted_revision: "accepted".into(),
            artifact_bytes: artifact.len() as u64,
            artifact_sha256: artifact_sha.clone(),
            clean_artifact_sha256: artifact_sha,
            execution_value: "\"hello\"".into(),
            differential_equal: true,
            container_cleanup_complete: true,
            temporary_root_removed: true,
            temporary_root: temporary.display().to_string(),
        };
        (commands, observed, candidate)
    }

    #[test]
    fn userland_original_reader_requires_the_complete_lifecycle_and_independent_result() {
        for policy in target::policy().userlands {
            let temporary = tempfile::tempdir().expect("userland original fixture");
            let root = temporary.path();
            let (commands, observed, candidate) = userland_fixture(root, &policy);
            let read = |records: &[CommandEvidence], observation: &UserlandObservation| {
                let mut originals = Originals {
                    root,
                    commands: records,
                    next: 0,
                };
                userland(&mut originals, &policy, observation, &candidate)?;
                require(originals.next == records.len(), "leftover command fixture")
            };
            read(&commands, &observed)
                .expect("complete independently constructed userland originals");
            let mut weakened = observed.clone();
            weakened.candidate_commands = 11;
            assert!(read(&commands, &weakened).is_err());
            let mut omitted = commands.clone();
            omitted.remove(
                omitted
                    .iter()
                    .position(|command| command.name.ends_with("-candidate-check"))
                    .expect("check original"),
            );
            assert!(read(&omitted, &observed).is_err());
            let mut rehashed = commands.clone();
            let run = rehashed
                .iter_mut()
                .find(|command| command.name.ends_with("-candidate-run-main"))
                .expect("run original");
            let output = root.join(&run.process.stdout.path);
            let original = fs::read(&output).expect("original run output");
            fs::write(
                &output,
                record(
                    "execution",
                    &[("value", "\"wrong\""), ("differential", "equal")],
                ),
            )
            .expect("plausible false result");
            run.process.stdout = evidence::proof(&output, run.process.stdout.path.clone())
                .expect("rehash false result");
            assert!(read(&rehashed, &observed).is_err());
            fs::write(&output, original).expect("restore genuine output");
            assert!(read(&commands, &observed).is_ok());
        }
    }

    #[test]
    fn original_command_reader_rejects_changed_commands_outcomes_and_retained_logs() {
        let temporary = tempfile::tempdir().expect("original fixture");
        let root = temporary.path();
        let stdout = root.join("0000-proof.stdout.log");
        let stderr = root.join("0000-proof.stderr.log");
        fs::write(&stdout, b"independent expected output\n").expect("stdout original");
        fs::write(&stderr, b"").expect("stderr original");
        let command = vec!["owned-tool".to_owned(), "expected-workload".to_owned()];
        let healthy = CommandEvidence {
            name: "proof".to_owned(),
            command: command.clone(),
            expected: "exit-0".to_owned(),
            process: ProcessObservation {
                status: ProcessStatus::Passed,
                exit_code: Some(0),
                signal: None,
                reason: None,
                elapsed_nanoseconds: 1,
                cpu_nanoseconds: None,
                peak_rss_kib: None,
                stdout_limit_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
                stderr_limit_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
                stdout_limit_exhausted: false,
                stderr_limit_exhausted: false,
                stdout: evidence::proof(&stdout, "0000-proof.stdout.log".to_owned())
                    .expect("stdout proof"),
                stderr: evidence::proof(&stderr, "0000-proof.stderr.log".to_owned())
                    .expect("stderr proof"),
            },
        };
        let read = |records: &[CommandEvidence]| {
            Originals {
                root,
                commands: records,
                next: 0,
            }
            .take("proof", command.clone(), 0, COMMAND_TIMEOUT)
        };
        assert_eq!(
            read(std::slice::from_ref(&healthy)).expect("genuine originals"),
            b"independent expected output\n"
        );
        assert!(read(&[]).is_err());
        for fault in 0..6 {
            let mut changed = healthy.clone();
            match fault {
                0 => changed.command[1] = "weakened-workload".to_owned(),
                1 => changed.process.status = ProcessStatus::Timeout,
                2 => changed.process.reason = Some("failed cleanup".to_owned()),
                3 => changed.process.stdout_limit_exhausted = true,
                4 => changed.process.stdout.mode = Some(0o777),
                _ => changed.process.stdout.path = "foreign.log".to_owned(),
            }
            assert!(read(&[changed]).is_err(), "accepted original fault {fault}");
        }
        fs::write(&stdout, b"different rehashed producer claim\n").expect("tampered original");
        assert!(read(std::slice::from_ref(&healthy)).is_err());
        fs::remove_file(&stdout).expect("remove fixture stdout");
        symlink(&stderr, &stdout).expect("substituted log link");
        assert!(read(std::slice::from_ref(&healthy)).is_err());
        fs::remove_file(&stdout).expect("remove owned link");
        fs::write(&stdout, b"independent expected output\n").expect("restore original");
        assert!(read(&[healthy]).is_ok());
    }
}
