//! Source-only query evidence cannot certify a transferred candidate process.
use super::*;
use crate::semantic_cli::SemanticBackend;
use std::path::Component;

pub(super) fn eligible(command: &[String]) -> bool {
    command.len() >= 5
        && command[1] == "--project"
        && Path::new(&command[2]).is_absolute()
        && matches!(command[3].as_str(), "query" | "inspect")
}

fn verify_origin(command: &CommandEvidence, verifier: Option<&FileProof>) -> Result<(), DevError> {
    let Some(call) = command.observation.as_semantic() else {
        return Ok(());
    };
    if !eligible(&command.command)
        || verifier.is_none_or(|proof| command.command.first() != Some(&proof.path))
        || call.backend != SemanticBackend::Filesystem
        || call.elapsed_nanoseconds > duration_nanoseconds(COMMAND_TIMEOUT)
    {
        return Err(DevError::corrupt(
            "service source read cannot certify a candidate process, mutable operation or foreign verifier",
        ));
    }
    command
        .observation
        .require_arguments(&command.command[1..])?;
    command.observation.require_outcome(call.returned_success)
}

pub(super) fn verify(
    evidence_root: &Path,
    receipt: &ServiceReceipt,
    verifier: Option<&FileProof>,
) -> Result<(), DevError> {
    if receipt.source_verifier.as_ref() != verifier {
        return Err(DevError::corrupt("service source verifier binding differs"));
    }
    if let Some(verifier) = verifier
        && (evidence::proof(Path::new(&verifier.path), verifier.path.clone())? != *verifier
            || receipt
                .commands
                .iter()
                .all(|command| command.observation.as_semantic().is_none()))
    {
        return Err(DevError::corrupt(
            "service source verifier or read evidence changed",
        ));
    }
    for command in &receipt.commands {
        verify_command(evidence_root, command, verifier)?;
    }
    Ok(())
}

fn verify_command(
    evidence_root: &Path,
    command: &CommandEvidence,
    verifier: Option<&FileProof>,
) -> Result<(), DevError> {
    verify_origin(command, verifier)?;
    if let Some(process) = command.observation.as_process() {
        return evidence::verify_process_files(evidence_root, process);
    }
    let call = &command.observation;
    for proof in [call.stdout(), call.stderr()] {
        let path = Path::new(&proof.path);
        if path.components().count() != 1
            || !matches!(path.components().next(), Some(Component::Normal(_)))
            || proof.kind != evidence::FileKind::File
            || proof
                .bytes
                .is_none_or(|bytes| bytes > MAXIMUM_COMMAND_STDOUT_BYTES)
            || evidence::proof(&evidence_root.join(path), proof.path.clone())? != *proof
        {
            return Err(DevError::corrupt(
                "service source response evidence changed",
            ));
        }
    }
    let bytes = process::read_bounded(
        &evidence_root.join(&call.stdout().path),
        MAXIMUM_COMMAND_STDOUT_BYTES,
    )?;
    let records = parse_records("service source response", &bytes).map_err(|_| {
        DevError::corrupt("service source response is not canonical public records")
    })?;
    call.require_public_response(&records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_cli::SemanticObservation;

    #[test]
    fn source_and_process_logs_bind_the_owned_directory_and_current_authority() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let project = root.join("project");
        lkjscript::platform::execute_new(&[
            project.display().to_string(),
            "--template".into(),
            "minimal".into(),
        ])
        .unwrap();
        let head = fs::read(project.join("HEAD")).unwrap();
        let evidence_root = root.join("evidence");
        fs::create_dir(&evidence_root).unwrap();
        let verifier = std::env::current_exe().unwrap().canonicalize().unwrap();
        let proof = evidence::proof(&verifier, verifier.display().to_string()).unwrap();
        let mut context = ServiceContext::new(root, &evidence_root, "source-read-test".into());
        context.source_verifier = Some(proof.clone());
        let query = vec![
            root.join("absent-candidate").display().to_string(),
            "--project".into(),
            project.display().to_string(),
            "query".into(),
            "owners".into(),
        ];
        let output = context
            .observe_command(CommandRequest::standard("query", query))
            .unwrap();
        output.observation.require_outcome(true).unwrap();
        assert!(output.observation.as_semantic().is_some());
        let command = &context.commands[0];
        verify_command(&evidence_root, command, Some(&proof)).unwrap();
        assert!(verify_command(root, command, Some(&proof)).is_err());
        assert!(verify_command(&evidence_root, command, None).is_err());
        let stdout = evidence_root.join(&output.observation.stdout().path);
        let original = fs::read(&stdout).unwrap();
        fs::write(&stdout, b"substituted response").unwrap();
        assert!(verify_command(&evidence_root, command, Some(&proof)).is_err());
        fs::write(&stdout, original).unwrap();
        verify_command(&evidence_root, command, Some(&proof)).unwrap();
        assert_eq!(fs::read(project.join("HEAD")).unwrap(), head);

        let output = context
            .observe_command(CommandRequest::standard(
                "process",
                vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    "printf actual-process".into(),
                ],
            ))
            .unwrap();
        assert!(output.observation.as_process().is_some());
        assert_eq!(output.stdout, b"actual-process");
        verify_command(&evidence_root, &context.commands[1], None).unwrap();
        assert!(verify_command(root, &context.commands[1], None).is_err());
    }

    #[test]
    fn source_read_mode_is_explicit_and_rejects_mutation_and_duplicate_options() {
        let options = |values: &[&str]| parse(values.iter().map(OsString::from));
        assert!(!options(&[]).unwrap().source_reads);
        assert!(options(&["--source-reads"]).unwrap().source_reads);
        assert!(options(&["--source-reads", "--source-reads"]).is_err());
        for operation in [
            "query", "inspect", "build", "check", "change", "data", "serve", "run",
        ] {
            let command =
                ["verifier", "--project", "/project", operation, "owners"].map(str::to_owned);
            assert_eq!(eligible(&command), matches!(operation, "query" | "inspect"));
        }
        assert!(!eligible(
            &["verifier", "--project", "relative", "query", "owners"].map(str::to_owned)
        ));
    }

    #[test]
    fn semantic_evidence_cannot_be_relabelled_as_candidate_or_different_authority() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("verifier");
        fs::write(&executable, b"fixture verifier identity").unwrap();
        let verifier = evidence::proof(&executable, executable.display().to_string()).unwrap();
        let command = vec![
            verifier.path.clone(),
            "--project".into(),
            "/project".into(),
            "query".into(),
            "owners".into(),
        ];
        let observation = CallObservation::Semantic(SemanticObservation {
            schema: "lkjscript-linked-semantic-call-2".into(),
            backend: SemanticBackend::Filesystem,
            operation: "query".into(),
            arguments: command[1..].to_vec(),
            returned_success: true,
            protocol_code: 0,
            elapsed_nanoseconds: 1,
            response: verifier.clone(),
            diagnostics: verifier.clone(),
        });
        let baseline = CommandEvidence {
            name: "query".into(),
            command,
            observation,
        };
        verify_origin(&baseline, Some(&verifier)).unwrap();
        assert!(verify_origin(&baseline, None).is_err());
        for mutation in 0..4 {
            let mut changed = baseline.clone();
            match mutation {
                0 => changed.command[0].push_str("-foreign"),
                1 => changed.command[2] = "/other-project".into(),
                2 => changed.command[3] = "change".into(),
                _ => {
                    let CallObservation::Semantic(call) = &mut changed.observation else {
                        unreachable!()
                    };
                    call.backend = SemanticBackend::Memory;
                }
            }
            assert!(verify_origin(&changed, Some(&verifier)).is_err());
        }
    }
}
