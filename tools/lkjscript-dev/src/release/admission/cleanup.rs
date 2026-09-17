//! Exact owned Docker resources survive an interrupted CLI long enough for the parent to join
//! cleanup. An ownership intent is durable before create; only its exact CID/name is inspected.
use super::*;
use crate::release::model::Sha256Digest;

const OWNER_LABEL: &str = "io.lkjscript.release-admission.owner";
const ROLES: [&str; 2] = ["musl", "older-glibc"];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::release) struct OwnedContainer {
    role: String,
    owner: String,
    pub(super) name: String,
    cid_file: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    id: String,
    name: String,
    owner: Option<String>,
}

fn expected(root: &Path, role: &str) -> Result<OwnedContainer, DevError> {
    if !ROLES.contains(&role) || root.canonicalize()? != root {
        return Err(DevError::corrupt(
            "owned target resource has a foreign role or root",
        ));
    }
    let root_text = root
        .to_str()
        .ok_or_else(|| DevError::corrupt("owned resource root is not UTF-8"))?;
    let owner = archive::sha256_bytes(
        format!("lkjscript-target-owned-container\n{root_text}\n{role}\n").as_bytes(),
    )?
    .as_str()
    .to_owned();
    Ok(OwnedContainer {
        role: role.to_owned(),
        name: format!("lkjscript-admission-{role}-{}", &owner[..24]),
        owner,
        cid_file: format!("{role}-container.cid"),
    })
}

fn marker(root: &Path, role: &str) -> PathBuf {
    root.join(format!("{role}-container-owner.json"))
}

pub(in crate::release) fn prepare(root: &Path, role: &str) -> Result<OwnedContainer, DevError> {
    let owner = expected(root, role)?;
    archive::reject_existing(&root.join(&owner.cid_file), "owned container CID")?;
    archive::write_new(&marker(root, role), &evidence::encode_json(&owner)?, 0o600)?;
    archive::synchronize_directory(root)?;
    Ok(owner)
}

pub(in crate::release) fn read(
    root: &Path,
    role: &str,
) -> Result<Option<OwnedContainer>, DevError> {
    let wanted = expected(root, role)?;
    let path = marker(root, role);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if fs::symlink_metadata(root.join(&wanted.cid_file)).is_ok() {
                return Err(DevError::corrupt(
                    "container CID has no durable ownership intent",
                ));
            }
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
        Ok(metadata)
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.permissions().mode() & 0o7777 != 0o600 =>
        {
            return Err(DevError::corrupt(
                "owned container marker is not a private regular file",
            ));
        }
        Ok(_) => {}
    }
    let bytes = process::read_bounded(&path, 4096)?;
    let observed: OwnedContainer = serde_json::from_slice(&bytes)?;
    if observed != wanted || evidence::encode_json(&observed)? != bytes {
        return Err(DevError::corrupt(
            "container ownership intent differs from its exact root and role",
        ));
    }
    Ok(Some(observed))
}

pub(super) fn create_command(root: &Path, owner: &OwnedContainer, image: &str) -> Vec<String> {
    ["docker".to_owned(), "create".to_owned()]
        .into_iter()
        .chain(options(root, owner))
        .chain([
            "--network".to_owned(),
            "host".to_owned(),
            image.to_owned(),
            "/bin/true".to_owned(),
        ])
        .collect()
}

pub(in crate::release) fn options(root: &Path, owner: &OwnedContainer) -> Vec<String> {
    [
        "--name".to_owned(),
        owner.name.clone(),
        "--cidfile".to_owned(),
        root.join(&owner.cid_file).display().to_string(),
        "--label".to_owned(),
        format!("{OWNER_LABEL}={}", owner.owner),
    ]
    .into()
}

pub(in crate::release) fn cid(
    root: &Path,
    owner: &OwnedContainer,
) -> Result<Option<String>, DevError> {
    let path = root.join(&owner.cid_file);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
        Ok(_) => {
            archive::ensure_regular(&path, "owned container CID")?;
        }
    }
    let bytes = process::read_bounded(&path, 65)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let text =
        std::str::from_utf8(&bytes).map_err(|_| DevError::corrupt("container CID is not UTF-8"))?;
    let id = text.strip_suffix('\n').unwrap_or(text);
    validate_id(id)?;
    Ok(Some(id.to_owned()))
}

fn validate_id(id: &str) -> Result<(), DevError> {
    Sha256Digest::new(id.to_owned())
        .map(|_| ())
        .map_err(|_| DevError::corrupt("container CID is not a complete lowercase identity"))
}

fn lookup_command(selector: String) -> Vec<String> {
    [
        "docker",
        "container",
        "ls",
        "--all",
        "--no-trunc",
        "--filter",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([selector, "--format".to_owned(), "{{.ID}}".to_owned()])
    .collect()
}

fn found(bytes: &[u8]) -> Result<Option<String>, DevError> {
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > 65 {
        return Err(DevError::corrupt(
            "owned container lookup was not a single complete identity",
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| DevError::corrupt("container lookup is not UTF-8"))?;
    let id = text.strip_suffix('\n').unwrap_or(text);
    validate_id(id)?;
    Ok(Some(id.to_owned()))
}

pub(in crate::release) fn remove_with(
    root: &Path,
    owner: &OwnedContainer,
    mut invoke: impl FnMut(&str, Vec<String>) -> Result<Vec<u8>, DevError>,
) -> Result<(), DevError> {
    if read(root, &owner.role)?.as_ref() != Some(owner) {
        return Err(DevError::corrupt(
            "container cleanup lacks matching persisted ownership",
        ));
    }
    let recorded = cid(root, owner)?;
    let selector = recorded.as_ref().map_or_else(
        || format!("name=^/{}$", owner.name),
        |id| format!("id={id}"),
    );
    let Some(id) = found(&invoke(
        &format!("{}-container-lookup", owner.role),
        lookup_command(selector),
    )?)?
    else {
        return Ok(());
    };
    if recorded.as_ref().is_some_and(|recorded| recorded != &id) {
        return Err(DevError::corrupt(
            "owned container lookup substituted a different full CID",
        ));
    }
    let format = format!(
        "{{\"id\":{{{{json .Id}}}},\"name\":{{{{json .Name}}}},\"owner\":{{{{json (index .Config.Labels \"{OWNER_LABEL}\")}}}}}}"
    );
    let bytes = invoke(
        &format!("{}-container-ownership", owner.role),
        vec![
            "docker".into(),
            "container".into(),
            "inspect".into(),
            "--format".into(),
            format,
            id.clone(),
        ],
    )?;
    if bytes.len() > 4096 {
        return Err(DevError::corrupt(
            "owned container identity exceeds its bound",
        ));
    }
    let identity: Identity = serde_json::from_slice(&bytes)?;
    if identity.id != id
        || identity.name != format!("/{}", owner.name)
        || identity.owner.as_deref() != Some(owner.owner.as_str())
    {
        return Err(DevError::corrupt(
            "container identity/ownership mismatch; no resource removed",
        ));
    }
    invoke(
        &format!("{}-container-remove", owner.role),
        vec!["docker".into(), "rm".into(), "--force".into(), id.clone()],
    )?;
    if found(&invoke(
        &format!("{}-container-removed", owner.role),
        lookup_command(format!("id={id}")),
    )?)?
    .is_some()
    {
        return Err(DevError::infrastructure(
            "owned container remains after removal",
        ));
    }
    Ok(())
}

pub(in crate::release) fn cleanup_owned_resources(root: &Path) -> Result<(), DevError> {
    match fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
        Ok(_) => {
            archive::ensure_directory(root, "target owned resource root")?;
        }
    }
    let owners = ROLES
        .into_iter()
        .map(|role| read(root, role))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if owners.is_empty() {
        return Ok(());
    }
    let logs = tempfile::Builder::new()
        .prefix("owned-container-cleanup-")
        .tempdir_in(root)?
        .keep();
    let mut context = Context {
        evidence_root: logs.clone(),
        ordinal: 0,
        commands: Vec::new(),
    };
    let mut failures = Vec::new();
    for owner in owners {
        if let Err(error) = remove_with(root, &owner, |name, command| {
            context.external_success(name, command, COMMAND_TIMEOUT)
        }) {
            failures.push(error.to_string());
        }
    }
    evidence::publish_json(
        &logs.join("receipt.json"),
        &serde_json::json!({
            "cleanup_complete": failures.is_empty(), "failures": failures, "commands": context.commands,
        }),
    )?;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(DevError::infrastructure(format!(
            "owned target container cleanup failed; originals retained in '{}': {}",
            logs.display(),
            failures.join("; ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_uses_exact_authenticated_identity_and_recovers_missing_cid() {
        for recorded in [false, true] {
            let temporary = tempfile::tempdir().expect("owned cleanup fixture");
            let root = temporary.path().canonicalize().expect("canonical root");
            let owner = prepare(&root, "musl").expect("durable intent");
            let id = "1".repeat(64);
            if recorded {
                fs::write(root.join(&owner.cid_file), &id).expect("Docker CID fixture");
            }
            let mut commands = Vec::new();
            remove_with(&root, &owner, |name, command| {
                commands.push(command);
                Ok(if name.ends_with("-lookup") {
                    format!("{id}\n").into_bytes()
                } else if name.ends_with("-ownership") {
                    serde_json::to_vec(&serde_json::json!({
                        "id": id, "name": format!("/{}", owner.name), "owner": owner.owner,
                    }))
                    .expect("identity")
                } else {
                    Vec::new()
                })
            })
            .expect("owned cleanup");
            assert_eq!(commands.len(), 4);
            assert_eq!(commands[2], ["docker", "rm", "--force", &id]);
            assert!(commands[0].contains(&if recorded {
                format!("id={id}")
            } else {
                format!("name=^/{}$", owner.name)
            }));
            remove_with(&root, &owner, |_, command| {
                assert!(!command.contains(&"rm".to_owned()));
                Ok(Vec::new())
            })
            .expect("already absent is idempotent");
        }
    }

    #[test]
    fn cleanup_rejects_foreign_missing_or_mismatched_ownership_without_removal() {
        let temporary = tempfile::tempdir().expect("cleanup fixture");
        let root = temporary.path().canonicalize().expect("canonical root");
        let owner = prepare(&root, "older-glibc").expect("owner intent");
        let id = "1".repeat(64);
        fs::write(root.join(&owner.cid_file), &id).expect("CID");
        for fault in [
            "foreign-label",
            "foreign-name",
            "foreign-id",
            "missing-label",
        ] {
            let mut removed = false;
            assert!(remove_with(&root, &owner, |name, command| {
                removed |= command.get(1).is_some_and(|part| part == "rm");
                Ok(if name.ends_with("-lookup") { format!("{id}\n").into_bytes() } else {
                    serde_json::to_vec(&serde_json::json!({
                        "id": if fault == "foreign-id" { "2".repeat(64) } else { id.clone() },
                        "name": if fault == "foreign-name" { "/foreign".to_owned() } else { format!("/{}", owner.name) },
                        "owner": if fault == "missing-label" { None } else if fault == "foreign-label" { Some("foreign") } else { Some(owner.owner.as_str()) },
                    })).expect("foreign fixture")
                })
            }).is_err());
            assert!(!removed, "removed {fault}");
        }
        fs::remove_file(marker(&root, "older-glibc")).expect("remove owned marker fixture");
        assert!(remove_with(&root, &owner, |_, _| panic!("unowned resource was queried")).is_err());
    }
}
