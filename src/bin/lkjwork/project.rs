use crate::bindings::Bindings;
use lkjscript::application::{
    ApplicationDigest, ApplicationValue, HostInterface, HostOperation, HostOutcomeClass,
};
use lkjscript::instance::{
    HostAdapterInput, HostAdapterKind, HostExecutionReceipt, HostGrant, HostGrantDescriptor,
    INSTANCE_CONTRACT_VERSION, InstanceCreateReceipt, InstanceCreateRequest, InstanceEventRequest,
    InstanceFakeHostRequest, InstanceHostRequest, InstanceId, InstanceInspection, InstanceMode,
    InstancePolicy, InstanceQueryReceipt, InstanceQueryRequest, InstanceResumeRequest,
    InstanceStore, InstanceTransitionReceipt, InstanceTransitionStatus,
    MAXIMUM_BLOB_NAMESPACE_BYTES, MAXIMUM_BLOB_OBJECTS, immutable_blob_digest,
};
use lkjscript::schema::ByteString;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

const LOCATOR_VERSION: u16 = 1;
const LOCATOR_FILE: &str = "locator";
const PRODUCT_DIRECTORY: &str = ".lkjwork";
const INSTANCE_STORE_DIRECTORY: &str = "instance-store";
const BLOBS_DIRECTORY: &str = "blobs";
const MAXIMUM_LOCATOR_BYTES: u64 = 4096;
const MAXIMUM_BACKUP_BYTES: u64 = 1024 * 1024 * 1024;
const MAXIMUM_BACKUP_FILES: u64 = 1_000_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Locator {
    version: u16,
    instance: InstanceId,
    application: ApplicationDigest,
    adapter: HostAdapterKind,
    checksum: String,
}

#[derive(Clone)]
pub struct Project {
    pub root: PathBuf,
    pub instance: InstanceId,
    pub application: ApplicationDigest,
    pub adapter: HostAdapterKind,
    product_directory: PathBuf,
    store: Arc<InstanceStore>,
}

#[derive(Clone, Debug)]
pub struct BackupReceipt {
    pub files: u64,
    pub bytes: u64,
    pub revision: u64,
    pub state_digest: String,
}

#[derive(Clone, Copy, Debug)]
pub struct FakeAttachmentOutcomes {
    pub put: HostOutcomeClass,
    pub inspect: HostOutcomeClass,
}

impl Project {
    pub fn session_cache_key(path: &Path) -> Result<PathBuf, String> {
        canonical_directory(path, "project path")
    }

    pub fn initialize(
        destination: &Path,
        name: &str,
        bindings: &Bindings,
        adapter: HostAdapterKind,
    ) -> Result<(Self, InstanceCreateReceipt), String> {
        if name.is_empty() {
            return Err("project name must not be empty".to_owned());
        }
        let root = prepare_destination(destination)?;
        let product_directory = root.join(PRODUCT_DIRECTORY);
        if fs::symlink_metadata(&product_directory).is_ok() {
            return Err("project destination already contains lkjwork authority".to_owned());
        }
        create_private_directory(&product_directory)?;
        let instance = random_instance_id()?;
        let result = (|| -> Result<InstanceCreateReceipt, String> {
            create_private_directory(&product_directory.join(BLOBS_DIRECTORY))?;
            let instance_store = product_directory.join(INSTANCE_STORE_DIRECTORY);
            create_private_directory(&instance_store)?;
            let initial_state = initial_state(bindings, name)?;
            let grant = HostGrant {
                version: INSTANCE_CONTRACT_VERSION,
                name: "attachments".to_owned(),
                instance,
                slot: "attachments".to_owned(),
                interface: HostInterface::ImmutableBlob,
                adapter,
                descriptor: HostGrantDescriptor::ImmutableBlob {
                    namespace: product_directory
                        .join(BLOBS_DIRECTORY)
                        .to_string_lossy()
                        .into_owned(),
                    maximum_objects: MAXIMUM_BLOB_OBJECTS as u64,
                    maximum_bytes: MAXIMUM_BLOB_NAMESPACE_BYTES,
                },
            };
            let store = InstanceStore::open(&instance_store).map_err(|error| error.to_string())?;
            let receipt = store
                .create(
                    &InstanceCreateRequest {
                        version: INSTANCE_CONTRACT_VERSION,
                        mode: InstanceMode::Commit,
                        instance,
                        initial_state,
                        grants: vec![grant],
                        policy: InstancePolicy::default(),
                    },
                    bindings.application_bytes(),
                )
                .map_err(|error| error.to_string())?;
            drop(store);
            let locator = Locator {
                version: LOCATOR_VERSION,
                instance,
                application: bindings.application_digest(),
                adapter,
                checksum: locator_checksum(
                    LOCATOR_VERSION,
                    instance,
                    bindings.application_digest(),
                    adapter,
                ),
            };
            publish_locator(&product_directory, &locator)?;
            sync_directory(&product_directory)?;
            sync_directory(&root).map_err(|error| {
                format!(
                    "project may be visible at {} but parent synchronization failed: {error}",
                    root.display()
                )
            })?;
            Ok(receipt)
        })();
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(error) => {
                if fs::symlink_metadata(product_directory.join(LOCATOR_FILE)).is_err() {
                    let _ = fs::remove_dir_all(&product_directory);
                }
                return Err(error);
            }
        };
        let store = Arc::new(
            InstanceStore::open(&product_directory.join(INSTANCE_STORE_DIRECTORY))
                .map_err(|error| error.to_string())?,
        );
        Ok((
            Self {
                root,
                instance,
                application: bindings.application_digest(),
                adapter,
                product_directory,
                store,
            },
            receipt,
        ))
    }

    pub fn discover(explicit: Option<&Path>, bindings: &Bindings) -> Result<Self, String> {
        Self::discover_with_cache(explicit, bindings, false)
    }

    pub fn discover_session(explicit: Option<&Path>, bindings: &Bindings) -> Result<Self, String> {
        Self::discover_with_cache(explicit, bindings, true)
    }

    fn discover_with_cache(
        explicit: Option<&Path>,
        bindings: &Bindings,
        session_cache: bool,
    ) -> Result<Self, String> {
        if let Some(path) = explicit {
            let root = canonical_directory(path, "project path")?;
            return Self::open_root_with_cache(root, bindings, session_cache);
        }
        let mut current = canonical_directory(
            &std::env::current_dir().map_err(|error| error.to_string())?,
            "current directory",
        )?;
        loop {
            if fs::symlink_metadata(current.join(PRODUCT_DIRECTORY).join(LOCATOR_FILE)).is_ok() {
                return Self::open_root_with_cache(current, bindings, session_cache);
            }
            if !current.pop() {
                break;
            }
        }
        Err("no lkjwork project was found; run `lkjwork init` or pass --project".to_owned())
    }

    pub fn inspect(&self) -> Result<InstanceInspection, String> {
        self.store
            .inspect(self.instance)
            .map_err(|error| error.to_string())
    }

    pub fn revalidate_locator(&self, bindings: &Bindings) -> Result<(), String> {
        let locator_path = self.product_directory.join(LOCATOR_FILE);
        reject_symlink(&locator_path, "project locator")?;
        let metadata = fs::metadata(&locator_path).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.len() > MAXIMUM_LOCATOR_BYTES {
            return Err("lkjwork locator is not a bounded regular file".to_owned());
        }
        let bytes = fs::read(&locator_path).map_err(|error| error.to_string())?;
        let locator: Locator = lkjscript::instance::strict_json(&bytes, "lkjwork locator")
            .map_err(|error| error.to_string())?;
        if locator.version != LOCATOR_VERSION
            || locator.checksum
                != locator_checksum(
                    locator.version,
                    locator.instance,
                    locator.application,
                    locator.adapter,
                )
            || locator.instance != self.instance
            || locator.application != self.application
            || locator.adapter != self.adapter
            || locator.application != bindings.application_digest()
        {
            return Err("lkjwork locator no longer names this exact project authority".to_owned());
        }
        Ok(())
    }

    pub fn inspect_deep(&self) -> Result<InstanceInspection, String> {
        self.store
            .inspect_deep(self.instance)
            .map_err(|error| error.to_string())
    }

    pub fn mutate(
        &self,
        event: ApplicationValue,
        event_key: Option<String>,
        base_revision: Option<u64>,
    ) -> Result<(InstanceTransitionReceipt, String), String> {
        let current = self
            .store
            .inspect(self.instance)
            .map_err(|error| error.to_string())?;
        let selected_revision = base_revision.unwrap_or(current.revision);
        let event_key =
            event_key.unwrap_or(event_key_for(self.instance, selected_revision, &event)?);
        let receipt = self
            .store
            .apply_event(&InstanceEventRequest {
                version: INSTANCE_CONTRACT_VERSION,
                mode: InstanceMode::Commit,
                instance: self.instance,
                base_revision: selected_revision,
                event_key: Some(event_key.clone()),
                event,
            })
            .map_err(|error| error.to_string())?;
        Ok((receipt, event_key))
    }

    pub fn query(&self, query: ApplicationValue) -> Result<InstanceQueryReceipt, String> {
        self.store
            .query(&InstanceQueryRequest {
                version: INSTANCE_CONTRACT_VERSION,
                instance: self.instance,
                revision: None,
                query,
            })
            .map_err(|error| error.to_string())
    }

    pub fn attach(
        &self,
        event: ApplicationValue,
        event_key: Option<String>,
        base_revision: Option<u64>,
        fake_outcomes: Option<FakeAttachmentOutcomes>,
    ) -> Result<(InstanceTransitionReceipt, Vec<HostExecutionReceipt>, String), String> {
        let (mut transition, event_key) = self.mutate(event, event_key, base_revision)?;
        let mut host_receipts = Vec::new();
        for resume_index in 1..=2 {
            if transition.status != InstanceTransitionStatus::Suspended {
                return Ok((transition, host_receipts, event_key));
            }
            let command = transition
                .command
                .as_ref()
                .ok_or_else(|| "suspended attachment omitted its exact host command".to_owned())?;
            if !transition.replayed {
                let host = match (self.adapter, fake_outcomes) {
                    (HostAdapterKind::Production, None) => self
                        .store
                        .execute_host(&InstanceHostRequest {
                            version: INSTANCE_CONTRACT_VERSION,
                            instance: self.instance,
                            command: command.id,
                            grant: self.attachment_grant(),
                            input: HostAdapterInput::None,
                        })
                        .map_err(|error| error.to_string())?,
                    (HostAdapterKind::DeterministicFake, Some(outcomes)) => {
                        let class = match command.operation {
                            HostOperation::PutBlob => outcomes.put,
                            HostOperation::InspectBlob => outcomes.inspect,
                        };
                        let evidence = fake_blob_evidence(command.operation, &command.request)?;
                        self.store
                            .record_fake_outcome(&InstanceFakeHostRequest {
                                version: INSTANCE_CONTRACT_VERSION,
                                instance: self.instance,
                                command: command.id,
                                grant: self.attachment_grant(),
                                class,
                                evidence,
                            })
                            .map_err(|error| error.to_string())?
                    }
                    (HostAdapterKind::Production, Some(_)) => {
                        return Err(
                            "fake outcomes cannot be supplied to a production project".to_owned()
                        );
                    }
                    (HostAdapterKind::DeterministicFake, None) => {
                        return Err(
                            "a deterministic-fake project requires explicit host outcomes"
                                .to_owned(),
                        );
                    }
                };
                host_receipts.push(host);
            }
            transition = self
                .store
                .resume(&InstanceResumeRequest {
                    version: INSTANCE_CONTRACT_VERSION,
                    mode: InstanceMode::Commit,
                    instance: self.instance,
                    base_revision: transition.next_revision,
                    event_key: Some(format!("{event_key}.r{resume_index}")),
                })
                .map_err(|error| error.to_string())?;
        }
        if transition.status == InstanceTransitionStatus::Suspended {
            Err("attachment reconciliation exceeded the closed two-command workflow".to_owned())
        } else {
            Ok((transition, host_receipts, event_key))
        }
    }

    pub fn backup(
        &self,
        destination: &Path,
        bindings: &Bindings,
    ) -> Result<(Self, BackupReceipt), String> {
        let absolute = if destination.is_absolute() {
            destination.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|error| error.to_string())?
                .join(destination)
        };
        if absolute
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err("backup destination must not contain parent traversal".to_owned());
        }
        if fs::symlink_metadata(&absolute).is_ok() {
            return Err("backup destination already exists; backups never replace".to_owned());
        }
        let parent = absolute
            .parent()
            .ok_or_else(|| "backup destination has no parent".to_owned())?;
        let parent = canonical_directory(parent, "backup destination parent")?;
        let name = absolute
            .file_name()
            .ok_or_else(|| "backup destination has no final component".to_owned())?;
        let destination = parent.join(name);
        let stage = parent.join(format!(
            ".lkjwork-backup-{}-{}",
            std::process::id(),
            random_stage_suffix()?
        ));
        create_private_directory(&stage)?;
        let source_inspection = self
            .store
            .inspect_deep(self.instance)
            .map_err(|error| error.to_string())?;
        let copy_result = copy_tree(
            &self.product_directory,
            &stage.join(PRODUCT_DIRECTORY),
            &mut CopyBudget::default(),
        );
        let budget = match copy_result {
            Ok(budget) => budget,
            Err(error) => {
                let _ = fs::remove_dir_all(&stage);
                return Err(error);
            }
        };
        sync_directory(&stage)?;
        let staged = Self::open_root(stage.clone(), bindings)?;
        let staged_inspection = staged.inspect_deep()?;
        if staged_inspection.instance != source_inspection.instance
            || staged_inspection.application != source_inspection.application
            || staged_inspection.revision != source_inspection.revision
            || staged_inspection.state_digest != source_inspection.state_digest
        {
            let _ = fs::remove_dir_all(&stage);
            return Err("backup staging authority differs from its exact source".to_owned());
        }
        drop(staged);
        fs::rename(&stage, &destination).map_err(|error| {
            let _ = fs::remove_dir_all(&stage);
            error.to_string()
        })?;
        sync_directory(&parent).map_err(|error| {
            format!(
                "backup may be visible at {} but parent synchronization failed: {error}",
                destination.display()
            )
        })?;
        let copied = Self::open_root(destination, bindings)?;
        let inspection = copied.inspect()?;
        Ok((
            copied,
            BackupReceipt {
                files: budget.files,
                bytes: budget.bytes,
                revision: inspection.revision,
                state_digest: inspection.state_digest.to_string(),
            },
        ))
    }

    pub fn restore_from(
        source: &Self,
        destination: &Path,
        bindings: &Bindings,
    ) -> Result<(Self, InstanceCreateReceipt), String> {
        let source_inspection = source
            .store
            .inspect(source.instance)
            .map_err(|error| error.to_string())?;
        if source_inspection.application != bindings.application_digest() {
            return Err("backup is bound to a foreign lkjwork application".to_owned());
        }
        if source_inspection.pending_command.is_some() {
            return Err("cannot restore semantic state while a host command is pending".to_owned());
        }
        let root = prepare_destination(destination)?;
        let product_directory = root.join(PRODUCT_DIRECTORY);
        if fs::symlink_metadata(&product_directory).is_ok() {
            return Err("restore destination already contains lkjwork authority".to_owned());
        }
        create_private_directory(&product_directory)?;
        let instance = random_instance_id()?;
        let restored_grant =
            attachment_grant_for(&product_directory, instance, HostAdapterKind::Production);
        let result = (|| -> Result<InstanceCreateReceipt, String> {
            copy_tree(
                &source.blobs_directory(),
                &product_directory.join(BLOBS_DIRECTORY),
                &mut CopyBudget::default(),
            )?;
            let instance_store = product_directory.join(INSTANCE_STORE_DIRECTORY);
            create_private_directory(&instance_store)?;
            let store = InstanceStore::open(&instance_store).map_err(|error| error.to_string())?;
            let receipt = store
                .create(
                    &InstanceCreateRequest {
                        version: INSTANCE_CONTRACT_VERSION,
                        mode: InstanceMode::Commit,
                        instance,
                        initial_state: source_inspection.state,
                        grants: vec![restored_grant.clone()],
                        policy: source_inspection.policy,
                    },
                    bindings.application_bytes(),
                )
                .map_err(|error| error.to_string())?;
            drop(store);
            let locator = Locator {
                version: LOCATOR_VERSION,
                instance,
                application: bindings.application_digest(),
                adapter: HostAdapterKind::Production,
                checksum: locator_checksum(
                    LOCATOR_VERSION,
                    instance,
                    bindings.application_digest(),
                    HostAdapterKind::Production,
                ),
            };
            publish_locator(&product_directory, &locator)?;
            sync_directory(&product_directory)?;
            sync_directory(&root).map_err(|error| {
                format!(
                    "restored project may be visible at {} but parent synchronization failed: {error}",
                    root.display()
                )
            })?;
            Ok(receipt)
        })();
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(error) => {
                if fs::symlink_metadata(product_directory.join(LOCATOR_FILE)).is_err() {
                    let _ = fs::remove_dir_all(&product_directory);
                }
                return Err(error);
            }
        };
        let restored = Self::open_root(root, bindings)?;
        Ok((restored, receipt))
    }

    pub fn blobs_directory(&self) -> PathBuf {
        self.product_directory.join(BLOBS_DIRECTORY)
    }

    fn attachment_grant(&self) -> HostGrant {
        attachment_grant_for(&self.product_directory, self.instance, self.adapter)
    }

    fn open_root(root: PathBuf, bindings: &Bindings) -> Result<Self, String> {
        Self::open_root_with_cache(root, bindings, false)
    }

    fn open_root_with_cache(
        root: PathBuf,
        bindings: &Bindings,
        session_cache: bool,
    ) -> Result<Self, String> {
        let product_directory = root.join(PRODUCT_DIRECTORY);
        reject_symlink(&product_directory, "product directory")?;
        let metadata = fs::metadata(&product_directory).map_err(|error| error.to_string())?;
        if !metadata.is_dir() {
            return Err("lkjwork product path is not a directory".to_owned());
        }
        ensure_private(&product_directory)?;
        let locator_path = product_directory.join(LOCATOR_FILE);
        reject_symlink(&locator_path, "project locator")?;
        let metadata = fs::metadata(&locator_path).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.len() > MAXIMUM_LOCATOR_BYTES {
            return Err("lkjwork locator is not a bounded regular file".to_owned());
        }
        let bytes = fs::read(&locator_path).map_err(|error| error.to_string())?;
        let locator: Locator = lkjscript::instance::strict_json(&bytes, "lkjwork locator")
            .map_err(|error| error.to_string())?;
        if locator.version != LOCATOR_VERSION
            || locator.checksum
                != locator_checksum(
                    locator.version,
                    locator.instance,
                    locator.application,
                    locator.adapter,
                )
        {
            return Err("lkjwork locator version or checksum is invalid".to_owned());
        }
        if locator.application != bindings.application_digest() {
            return Err("project is bound to a foreign lkjwork application".to_owned());
        }
        let store_path = product_directory.join(INSTANCE_STORE_DIRECTORY);
        reject_symlink(&store_path, "instance store")?;
        let store = if session_cache {
            InstanceStore::open_session(&store_path)
        } else {
            InstanceStore::open(&store_path)
        }
        .map_err(|error| error.to_string())?;
        let project = Self {
            root,
            instance: locator.instance,
            application: locator.application,
            adapter: locator.adapter,
            product_directory,
            store: Arc::new(store),
        };
        Ok(project)
    }
}

fn attachment_grant_for(
    product_directory: &Path,
    instance: InstanceId,
    adapter: HostAdapterKind,
) -> HostGrant {
    HostGrant {
        version: INSTANCE_CONTRACT_VERSION,
        name: "attachments".to_owned(),
        instance,
        slot: "attachments".to_owned(),
        interface: HostInterface::ImmutableBlob,
        adapter,
        descriptor: HostGrantDescriptor::ImmutableBlob {
            namespace: product_directory
                .join(BLOBS_DIRECTORY)
                .to_string_lossy()
                .into_owned(),
            maximum_objects: MAXIMUM_BLOB_OBJECTS as u64,
            maximum_bytes: MAXIMUM_BLOB_NAMESPACE_BYTES,
        },
    }
}

fn initial_state(bindings: &Bindings, name: &str) -> Result<ApplicationValue, String> {
    bindings.product(
        "project",
        vec![
            ("name", bindings.text(name)?),
            ("next_task_id", bindings.integer(1)),
            ("next_note_id", bindings.integer(1)),
            ("tasks", bindings.sequence("task_sequence", Vec::new())?),
            (
                "activity",
                bindings.sequence("activity_sequence", Vec::new())?,
            ),
            (
                "pending_attachment",
                bindings.sum("pending_attachment_option", "none", None)?,
            ),
        ],
    )
}

fn random_instance_id() -> Result<InstanceId, String> {
    loop {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes)
            .map_err(|error| format!("cannot obtain instance entropy: {error}"))?;
        if bytes != [0; 16] {
            return Ok(InstanceId::from_bytes(bytes));
        }
    }
}

fn random_stage_suffix() -> Result<String, String> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("cannot obtain staging entropy: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[derive(Clone, Copy, Debug, Default)]
struct CopyBudget {
    files: u64,
    bytes: u64,
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    budget: &mut CopyBudget,
) -> Result<CopyBudget, String> {
    reject_symlink(source, "backup source")?;
    let metadata = fs::metadata(source).map_err(|error| error.to_string())?;
    if !metadata.is_dir() {
        return Err("backup source is not a directory".to_owned());
    }
    create_private_directory(destination)?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "backup source contains symlink {}",
                source_path.display()
            ));
        }
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path, budget)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(format!(
                "backup source contains nonregular file {}",
                source_path.display()
            ));
        }
        budget.files = budget
            .files
            .checked_add(1)
            .ok_or_else(|| "backup file count overflows".to_owned())?;
        budget.bytes = budget
            .bytes
            .checked_add(metadata.len())
            .ok_or_else(|| "backup byte count overflows".to_owned())?;
        if budget.files > MAXIMUM_BACKUP_FILES || budget.bytes > MAXIMUM_BACKUP_BYTES {
            return Err("backup exceeds exact file or byte policy".to_owned());
        }
        let mut input = File::open(&source_path).map_err(|error| error.to_string())?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&destination_path)
            .map_err(|error| error.to_string())?;
        let copied = std::io::copy(&mut (&mut input).take(metadata.len() + 1), &mut output)
            .map_err(|error| error.to_string())?;
        if copied != metadata.len() {
            return Err(format!(
                "backup source changed while copying {}",
                source_path.display()
            ));
        }
        output.sync_all().map_err(|error| error.to_string())?;
    }
    sync_directory(destination)?;
    Ok(*budget)
}

fn prepare_destination(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    if absolute
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("project destination must not contain parent traversal".to_owned());
    }
    match fs::symlink_metadata(&absolute) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err("project destination must be a non-symlink directory".to_owned());
            }
            if fs::read_dir(&absolute)
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
            {
                return Err("project destination must be empty".to_owned());
            }
            canonical_directory(&absolute, "project destination")
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = absolute
                .parent()
                .ok_or_else(|| "project destination has no parent".to_owned())?;
            let parent = canonical_directory(parent, "project destination parent")?;
            let name = absolute
                .file_name()
                .ok_or_else(|| "project destination has no final component".to_owned())?;
            let destination = parent.join(name);
            create_private_directory(&destination)?;
            Ok(destination)
        }
        Err(error) => Err(error.to_string()),
    }
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf, String> {
    reject_symlink(path, label)?;
    let canonical =
        fs::canonicalize(path).map_err(|error| format!("cannot resolve {label}: {error}"))?;
    let metadata = fs::metadata(&canonical).map_err(|error| error.to_string())?;
    if !metadata.is_dir() {
        return Err(format!("{label} is not a directory"));
    }
    Ok(canonical)
}

fn create_private_directory(path: &Path) -> Result<(), String> {
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    builder.create(path).map_err(|error| error.to_string())?;
    ensure_private(path)
}

fn ensure_private(path: &Path) -> Result<(), String> {
    let mode = fs::metadata(path)
        .map_err(|error| error.to_string())?
        .permissions()
        .mode()
        & 0o777;
    if mode & 0o077 != 0 {
        return Err(format!(
            "{} must not grant group or other access",
            path.display()
        ));
    }
    Ok(())
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && metadata.file_type().is_symlink()
    {
        return Err(format!("{label} must not be a symlink"));
    }
    Ok(())
}

fn locator_checksum(
    version: u16,
    instance: InstanceId,
    application: ApplicationDigest,
    adapter: HostAdapterKind,
) -> String {
    let mut hasher = blake3::Hasher::new_derive_key("lkjwork.locator.v1");
    hasher.update(&version.to_le_bytes());
    hasher.update(&instance.as_bytes());
    hasher.update(&application.as_bytes());
    hasher.update(&[match adapter {
        HostAdapterKind::Production => 1,
        HostAdapterKind::DeterministicFake => 2,
    }]);
    hasher.finalize().to_hex().to_string()
}

fn fake_blob_evidence(
    operation: HostOperation,
    request: &ApplicationValue,
) -> Result<ByteString, String> {
    let ApplicationValue::Sum {
        payload: Some(payload),
        ..
    } = request
    else {
        return Err("fake blob request must contain one exact variant payload".to_owned());
    };
    let ApplicationValue::Bytes(request) = payload.as_ref() else {
        return Err("fake blob request payload must contain exact bytes".to_owned());
    };
    let digest = match operation {
        HostOperation::PutBlob => immutable_blob_digest(request.as_slice()).as_bytes(),
        HostOperation::InspectBlob if request.len() == 32 => {
            let mut digest = [0_u8; 32];
            digest.copy_from_slice(request.as_slice());
            digest
        }
        HostOperation::InspectBlob => {
            return Err("fake blob inspection request must contain one exact digest".to_owned());
        }
    };
    ByteString::from_slice(&digest).map_err(|error| error.to_string())
}

fn event_key_for(
    instance: InstanceId,
    base_revision: u64,
    event: &ApplicationValue,
) -> Result<String, String> {
    let encoded = serde_json::to_vec(event).map_err(|error| error.to_string())?;
    let mut hasher = blake3::Hasher::new_derive_key("lkjwork.event-key.v1");
    hasher.update(&instance.as_bytes());
    hasher.update(&base_revision.to_le_bytes());
    hasher.update(&(encoded.len() as u64).to_le_bytes());
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}

fn publish_locator(directory: &Path, locator: &Locator) -> Result<(), String> {
    let path = directory.join(LOCATOR_FILE);
    reject_symlink(&path, "project locator")?;
    let bytes = serde_json::to_vec(locator).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAXIMUM_LOCATOR_BYTES {
        return Err("lkjwork locator exceeds byte policy".to_owned());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())
}
