//! Immutable local runtime slots and a single atomic selection pointer.
mod fs;
use super::diagnostic::{Diagnostic, DiagnosticClass};
use crate::release_container::{self as container, VerifiedArchive, model::Sha256Digest};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const MAXIMUM_VERSIONS: usize = 128;
const MAXIMUM_METADATA_BYTES: u64 = 2 * 1024 * 1024;
const ROOT_METADATA: &str = "INSTALLATION.json";
const LOCK: &str = "installation.lock";
const RECEIPT: &str = "INSTALL-RECEIPT.json";
const STAGE_MARKER: &str = "STAGE.json";
const PAYLOADS: [&str; 4] = [
    "lkjscript",
    "LICENSE",
    "THIRD-PARTY-LICENSES.html",
    "RELEASE-MANIFEST.json",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Ownership {
    format: String,
    version: u32,
    installation: String,
    owner_uid: u32,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    format: String,
    version: u32,
    installation: String,
    archive: VerifiedArchive,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct StageMarker {
    format: String,
    installation: String,
}

#[derive(Clone, Debug)]
pub struct InstalledVersion {
    pub tag: String,
    pub path: PathBuf,
    pub archive_sha256: String,
    pub archive_bytes: u64,
    pub manifest_sha256: String,
    pub executable_sha256: String,
    pub publication: &'static str,
    pub integrity: &'static str,
}
#[derive(Debug)]
pub struct Inventory {
    pub prefix: PathBuf,
    pub selected: Option<String>,
    pub versions: Vec<InstalledVersion>,
}
#[derive(Debug)]
pub struct Mutation {
    pub version: InstalledVersion,
    pub prefix: PathBuf,
    pub selected: Option<String>,
    pub previous: Option<String>,
    pub outcome: &'static str,
    pub manager: PathBuf,
    pub retained_manager: bool,
}

// Fault injection is an internal Rust test boundary, never CLI/environment admission policy.
trait Checkpoints {
    fn at(&self, _point: &'static str) -> Result<(), Diagnostic> {
        Ok(())
    }
}
struct Ordinary;
impl Checkpoints for Ordinary {}

pub fn resolve_prefix(explicit: Option<&str>) -> Result<PathBuf, Diagnostic> {
    let prefix = match explicit {
        Some(value) => PathBuf::from(value),
        None => {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .ok_or_else(|| {
                    source(
                        "runtime_prefix",
                        "an absolute HOME is unavailable; supply an explicit absolute --prefix",
                    )
                })?;
            home.join(".local")
        }
    };
    fs::absolute(&prefix)?;
    if prefix.to_str().is_none() {
        return Err(source(
            "runtime_prefix",
            "prefix must be UTF-8 for exact public path reporting",
        ));
    }
    Ok(prefix)
}

pub fn validate_tag(tag: &str) -> Result<(), Diagnostic> {
    container::validate_strict_tag(tag, tag.strip_prefix('v').unwrap_or(""))
        .map_err(|error| source("runtime_tag", error.message))
}
fn host() -> Result<(), Diagnostic> {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return Err(source(
            "runtime_host",
            "installation supports only Linux x86-64 and x86_64-unknown-linux-musl archives",
        ));
    }
    Ok(())
}

pub fn list(prefix: &Path) -> Result<Inventory, Diagnostic> {
    host()?;
    let Some(root) = Root::open(prefix, false, &Ordinary)? else {
        return Ok(Inventory {
            prefix: prefix.to_owned(),
            selected: None,
            versions: Vec::new(),
        });
    };
    let _lock = root.lock(false)?;
    root.validate_inventory()?;
    let selected = root.pointer()?;
    let versions = root.inventory(false)?;
    check_selection(&selected, &versions)?;
    Ok(Inventory {
        prefix: prefix.to_owned(),
        selected,
        versions,
    })
}

pub fn install(
    prefix: &Path,
    archive: &Path,
    digest: &str,
    activate: bool,
) -> Result<Mutation, Diagnostic> {
    install_with(prefix, archive, digest, activate, &Ordinary)
}
fn install_with(
    prefix: &Path,
    archive: &Path,
    digest: &str,
    activate: bool,
    points: &dyn Checkpoints,
) -> Result<Mutation, Diagnostic> {
    host()?;
    let digest =
        Sha256Digest::new(digest.to_owned()).map_err(|error| source("runtime_sha256", error))?;
    // Open input once with no-follow/nonblocking admission before any installation mutation.
    let archive = if archive.is_absolute() {
        archive.to_owned()
    } else {
        std::env::current_dir().map_err(io_error)?.join(archive)
    };
    let parent = archive
        .parent()
        .ok_or_else(|| source("runtime_archive", "archive requires a parent"))?;
    let archive_parent = fs::open_absolute(parent, false)?
        .ok_or_else(|| source("runtime_archive", "archive parent is absent"))?;
    let mut input = fs::file_at(
        &archive_parent,
        archive
            .file_name()
            .ok_or_else(|| source("runtime_archive", "archive requires a regular filename"))?,
    )?;
    let initial = input.metadata().map_err(io_error)?;
    if initial.len() == 0 || initial.len() > container::MAXIMUM_COMPRESSED_BYTES {
        return Err(capacity("archive exceeds compressed byte admission"));
    }
    let root = Root::open(prefix, true, points)?
        .ok_or_else(|| corrupt("new installation root is absent"))?;
    let _lock = root.lock(true)?;
    root.recheck()?;
    root.clean_stages()?;
    let previous = root.pointer()?;
    let inventory = root.inventory(false)?;
    check_selection(&previous, &inventory)?;
    let stage_name = format!(".stage-{}", random_id()?);
    let stage = fs::private_directory(&root.root, &stage_name)?;
    let operation = (|| {
        write_json(
            &stage,
            STAGE_MARKER,
            &StageMarker {
                format: "lkjscript-install-stage-1".to_owned(),
                installation: root.owner.installation.clone(),
            },
        )?;
        let mut snapshot = fs::create_file(&stage, "archive.gz", 0o600)?;
        fs::reserve(&snapshot, initial.len())?;
        let mut hasher = sha2::Sha256::new();
        let mut buffer = [0_u8; 65536];
        let mut copied = 0_u64;
        loop {
            let count = input.read(&mut buffer).map_err(io_error)?;
            if count == 0 {
                break;
            }
            copied = copied
                .checked_add(count as u64)
                .filter(|value| *value <= initial.len())
                .ok_or_else(|| corrupt("archive changed length while snapshotting"))?;
            snapshot.write_all(&buffer[..count]).map_err(io_error)?;
            hasher.update(&buffer[..count]);
        }
        let after = input.metadata().map_err(io_error)?;
        if copied != initial.len()
            || after.len() != initial.len()
            || after.mtime() != initial.mtime()
            || after.mtime_nsec() != initial.mtime_nsec()
            || after.ctime() != initial.ctime()
            || after.ctime_nsec() != initial.ctime_nsec()
            || hasher
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
                != digest.as_str()
        {
            return Err(corrupt(
                "archive changed during snapshot or does not match the requested SHA-256",
            ));
        }
        fs::sync(&snapshot)?;
        points.at("snapshot-complete")?;
        snapshot.rewind().map_err(io_error)?;
        let bytes = container::read_bounded(&mut snapshot, container::MAXIMUM_COMPRESSED_BYTES)
            .map_err(format_error)?;
        let admitted = container::admit_gzip(&bytes).map_err(format_error)?;
        if admitted.verified.archive_sha256 != digest {
            return Err(corrupt("snapshot identity changed"));
        }
        let tag = &admitted.verified.manifest.source.expected_release_tag;
        let existing = inventory.iter().any(|version| version.tag == *tag);
        if existing {
            let (_, receipt) = root.retained(tag, true)?;
            if receipt.archive != admitted.verified {
                return Err(source(
                    "runtime_version_conflict",
                    format!(
                        "{tag} already retains a different archive identity; immutable versions cannot be overwritten; choose a new owned prefix"
                    ),
                ));
            }
        } else {
            if inventory.len() >= MAXIMUM_VERSIONS {
                return Err(capacity(
                    "installation has reached its 128-version inventory limit",
                ));
            }
            let version = fs::private_directory(&stage, "version")?;
            let target = fs::private_directory(&version, container::TARGET_TRIPLE)?;
            for (member, bytes) in admitted.payloads() {
                let name = member
                    .name
                    .strip_prefix("lkjscript/")
                    .ok_or_else(|| corrupt("admitted member is outside product directory"))?;
                fs::write_new(&target, name, bytes, member.mode)?;
            }
            write_json(
                &target,
                RECEIPT,
                &Receipt {
                    format: "lkjscript-install-receipt".to_owned(),
                    version: 1,
                    installation: root.owner.installation.clone(),
                    archive: admitted.verified.clone(),
                },
            )?;
            rustix::fs::fchmod(&target, rustix::fs::Mode::from_raw_mode(0o755))
                .map_err(io_error)?;
            rustix::fs::fchmod(&version, rustix::fs::Mode::from_raw_mode(0o755))
                .map_err(io_error)?;
            points.at("before-version-sync")?;
            fs::sync(&target)?;
            fs::sync(&version)?;
            fs::sync(&stage)?;
            root.recheck()?;
            points.at("before-version-publish")?;
            fs::publish(&stage, "version", &root.versions, tag).map_err(io_error)?;
            let published = (|| {
                points.at("after-version-publish")?;
                fs::sync(&root.versions)?;
                points.at("after-version-sync")
            })();
            if let Err(mut error) = published {
                error.notes.push(format!("complete version published at {}; selection unchanged; version durability uncertain", root.pinned(tag).display()));
                return Err(error);
            }
        }
        let result = root.finish(
            tag,
            previous.clone(),
            if existing {
                "already-installed"
            } else {
                "installed"
            },
            activate,
            points,
        );
        result.map_err(|mut error| {
            error.notes.push(format!(
                "complete version retained at {}; inspect runtime list for observed selection",
                root.pinned(tag).display()
            ));
            error
        })
    })();
    combine_cleanup(operation, root.clean_stage(&stage_name, &stage))
}
use sha2::Digest;

pub fn select(prefix: &Path, tag: &str) -> Result<Mutation, Diagnostic> {
    select_with(prefix, tag, &Ordinary)
}
fn select_with(prefix: &Path, tag: &str, points: &dyn Checkpoints) -> Result<Mutation, Diagnostic> {
    host()?;
    validate_tag(tag)?;
    let root = Root::open(prefix, false, points)?.ok_or_else(|| {
        source(
            "runtime_not_installed",
            "installation is absent; install the exact archive first",
        )
    })?;
    let _lock = root.lock(true)?;
    root.recheck()?;
    root.clean_stages()?;
    let previous = root.pointer()?;
    let inventory = root.inventory(false)?;
    check_selection(&previous, &inventory)?;
    root.finish(tag, previous, "selected", true, points)
}

struct Root {
    prefix: PathBuf,
    prefix_fd: File,
    bin: File,
    root: File,
    versions: File,
    owner: Ownership,
}
impl Root {
    fn open(
        prefix: &Path,
        create: bool,
        points: &dyn Checkpoints,
    ) -> Result<Option<Self>, Diagnostic> {
        fs::absolute(prefix)?;
        if let Some(existing) = fs::open_absolute(prefix, false)? {
            fs::owned(&existing)?;
            let bin = fs::directory_at(&existing, "bin")?;
            let lib = fs::directory_at(&existing, "lib")?;
            if let Some(bin) = &bin {
                fs::owned(bin)?;
                pointer_at(bin)?;
            }
            if let Some(lib) = &lib {
                fs::owned(lib)?;
                if let Some(root) = fs::directory_at(lib, "lkjscript")? {
                    read_ownership(&root)?;
                } else if bin
                    .as_ref()
                    .map(pointer_at)
                    .transpose()?
                    .flatten()
                    .is_some()
                {
                    return Err(source(
                        "runtime_pointer_conflict",
                        "default pointer has no owned installation; choose a different prefix",
                    ));
                }
            } else if bin
                .as_ref()
                .map(pointer_at)
                .transpose()?
                .flatten()
                .is_some()
            {
                return Err(source(
                    "runtime_pointer_conflict",
                    "default pointer has no owned installation; choose a different prefix",
                ));
            }
            if !create
                && lib
                    .as_ref()
                    .map(|lib| fs::directory_at(lib, "lkjscript"))
                    .transpose()?
                    .flatten()
                    .is_none()
            {
                return Ok(None);
            }
        } else if !create {
            return Ok(None);
        }
        let prefix_fd =
            fs::open_absolute(prefix, create)?.ok_or_else(|| corrupt("prefix disappeared"))?;
        fs::owned(&prefix_fd)?;
        if create {
            fs::mkdir(&prefix_fd, "lib", 0o755)?;
            fs::mkdir(&prefix_fd, "bin", 0o755)?;
        }
        let lib = fs::required_directory(&prefix_fd, "lib")?;
        let bin = fs::required_directory(&prefix_fd, "bin")?;
        fs::owned(&lib)?;
        fs::owned(&bin)?;
        let root = match fs::directory_at(&lib, "lkjscript")? {
            Some(root) => root,
            None if create => initialize(&lib, &prefix.join("lib"), points)?,
            None => return Ok(None),
        };
        let owner = read_ownership(&root)?;
        let versions = fs::required_directory(&root, "versions")?;
        fs::owned(&versions)?;
        let this = Self {
            prefix: prefix.to_owned(),
            prefix_fd,
            bin,
            root,
            versions,
            owner,
        };
        this.pointer()?;
        Ok(Some(this))
    }
    fn lock(&self, exclusive: bool) -> Result<File, Diagnostic> {
        let lock = fs::file_at(&self.root, LOCK)?;
        fs::owned(&lock)?;
        let operation = if exclusive {
            rustix::fs::FlockOperation::NonBlockingLockExclusive
        } else {
            rustix::fs::FlockOperation::NonBlockingLockShared
        };
        rustix::fs::flock(&lock, operation).map_err(|error| {
            if error == rustix::io::Errno::WOULDBLOCK {
                source(
                    "runtime_busy",
                    "another installation operation holds the OS lock; retry after it completes",
                )
            } else {
                io_error(error)
            }
        })?;
        Ok(lock)
    }
    fn recheck(&self) -> Result<(), Diagnostic> {
        let prefix =
            fs::open_absolute(&self.prefix, false)?.ok_or_else(|| corrupt("prefix disappeared"))?;
        let lib = fs::required_directory(&prefix, "lib")?;
        let root = fs::required_directory(&lib, "lkjscript")?;
        let bin = fs::required_directory(&prefix, "bin")?;
        let versions = fs::required_directory(&root, "versions")?;
        if !fs::same(&prefix, &self.prefix_fd)?
            || !fs::same(&root, &self.root)?
            || !fs::same(&bin, &self.bin)?
            || !fs::same(&versions, &self.versions)?
            || read_ownership(&root)? != self.owner
        {
            return Err(corrupt("installation namespace changed while open"));
        }
        self.pointer()?;
        Ok(())
    }
    fn pinned(&self, tag: &str) -> PathBuf {
        self.prefix
            .join("lib/lkjscript/versions")
            .join(tag)
            .join(container::TARGET_TRIPLE)
            .join("lkjscript")
    }
    fn pointer(&self) -> Result<Option<String>, Diagnostic> {
        pointer_at(&self.bin)
    }
    fn inventory(&self, full: bool) -> Result<Vec<InstalledVersion>, Diagnostic> {
        let mut inventory = Vec::new();
        for tag in fs::entries(&self.versions, MAXIMUM_VERSIONS)? {
            validate_tag(&tag)
                .map_err(|_| corrupt("installed version directory has an invalid canonical tag"))?;
            inventory.push(self.retained(&tag, full)?.0);
        }
        Ok(inventory)
    }
    fn retained(&self, tag: &str, full: bool) -> Result<(InstalledVersion, Receipt), Diagnostic> {
        validate_tag(tag)?;
        let version = fs::directory_at(&self.versions, tag)?.ok_or_else(|| {
            source(
                "runtime_not_installed",
                format!("exact version {tag} is not installed"),
            )
        })?;
        fs::owned(&version)?;
        if fs::entries(&version, 2)? != [container::TARGET_TRIPLE] {
            return Err(corrupt("installed version target inventory is invalid"));
        }
        let target = fs::required_directory(&version, container::TARGET_TRIPLE)?;
        fs::owned(&target)?;
        let mut expected = PAYLOADS
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>();
        expected.push(RECEIPT.to_owned());
        expected.sort();
        if fs::entries(&target, 6)? != expected {
            return Err(corrupt(
                "installed payload inventory is incomplete or foreign",
            ));
        }
        let receipt: Receipt = read_json(&target, RECEIPT)?;
        let manifest_bytes = read_file(
            &target,
            "RELEASE-MANIFEST.json",
            container::MAXIMUM_MANIFEST_BYTES,
            Some(0o644),
        )?;
        let manifest = container::decode_manifest(&manifest_bytes).map_err(format_error)?;
        if receipt.format != "lkjscript-install-receipt"
            || receipt.version != 1
            || receipt.installation != self.owner.installation
            || receipt.archive.manifest != manifest
            || manifest.source.expected_release_tag != tag
            || receipt.archive.manifest_sha256
                != container::sha256_bytes(&manifest_bytes).map_err(format_error)?
            || receipt.archive.archive_byte_length == 0
            || receipt.archive.archive_byte_length > container::MAXIMUM_COMPRESSED_BYTES
            || receipt.archive.source_timestamp_unix_seconds
                != manifest.source.commit_timestamp_unix_seconds
            || receipt.archive.members.len() != 5
        {
            return Err(corrupt(
                "installed receipt/manifest identity mismatch; preserve this slot and install into a new owned prefix",
            ));
        }
        for (index, member) in receipt.archive.members.iter().enumerate() {
            let identity = &manifest.packaging.members[index];
            if member.name != identity.name || member.mode != identity.mode {
                return Err(corrupt("installed receipt member order or mode mismatch"));
            }
            if index == 0 {
                if member.byte_length != 0 || member.sha256.is_some() {
                    return Err(corrupt("installed receipt directory identity mismatch"));
                }
                continue;
            }
            let name = PAYLOADS[index - 1];
            let (length, digest) = match index {
                1 => (manifest.executable.byte_length, &manifest.executable.sha256),
                2 => (
                    manifest.root_license.byte_length,
                    &manifest.root_license.sha256,
                ),
                3 => (
                    manifest.third_party_notices.byte_length,
                    &manifest.third_party_notices.sha256,
                ),
                _ => (
                    manifest_bytes.len() as u64,
                    &receipt.archive.manifest_sha256,
                ),
            };
            if member.byte_length != length
                || member.sha256.as_ref() != Some(digest)
                || length > container::MAXIMUM_UNCOMPRESSED_BYTES
            {
                return Err(corrupt("installed receipt payload identity mismatch"));
            }
            let mut file = fs::file_at(&target, name)?;
            fs::owned(&file)?;
            let metadata = file.metadata().map_err(io_error)?;
            if metadata.len() != length || metadata.mode() & 0o7777 != member.mode {
                return Err(corrupt(
                    "installed payload length/mode corruption; use a new owned prefix for recovery",
                ));
            }
            if full {
                let bytes = container::read_bounded(&mut file, length).map_err(format_error)?;
                if container::sha256_bytes(&bytes).map_err(format_error)? != *digest {
                    return Err(corrupt(
                        "installed payload digest corruption; immutable slots are preserved; install into a new owned prefix",
                    ));
                }
                if index == 1
                    && container::inspect_static_elf_bytes(&bytes).map_err(format_error)?
                        != manifest.executable.elf
                {
                    return Err(corrupt(
                        "installed executable static-target identity mismatch",
                    ));
                }
            }
        }
        Ok((
            InstalledVersion {
                tag: tag.to_owned(),
                path: self.pinned(tag),
                archive_sha256: receipt.archive.archive_sha256.as_str().to_owned(),
                archive_bytes: receipt.archive.archive_byte_length,
                manifest_sha256: receipt.archive.manifest_sha256.as_str().to_owned(),
                executable_sha256: manifest.executable.sha256.as_str().to_owned(),
                publication: match manifest.publication_mode {
                    container::model::PublicationMode::DryRun => {
                        "declared-dry-run/unverified-publication"
                    }
                    container::model::PublicationMode::Release => {
                        "declared-release/unverified-publication"
                    }
                },
                integrity: if full { "verified" } else { "unchecked" },
            },
            receipt,
        ))
    }
    fn finish(
        &self,
        tag: &str,
        previous: Option<String>,
        outcome: &'static str,
        activate: bool,
        points: &dyn Checkpoints,
    ) -> Result<Mutation, Diagnostic> {
        let version = self.retained(tag, true)?.0;
        let (manager, retained_manager) = self.manager()?;
        if activate && previous.as_deref() != Some(tag) {
            self.activate(tag, points)?;
        }
        Ok(Mutation {
            version,
            prefix: self.prefix.clone(),
            selected: self.pointer()?,
            previous,
            outcome,
            manager,
            retained_manager,
        })
    }
    fn manager(&self) -> Result<(PathBuf, bool), Diagnostic> {
        static INVOKING: std::sync::OnceLock<Result<(PathBuf, String), Diagnostic>> =
            std::sync::OnceLock::new();
        let (path, digest) = INVOKING
            .get_or_init(|| {
                let path = std::env::current_exe().map_err(io_error)?;
                // Hash the trusted kernel-owned executable reference without retaining the binary in memory.
                let mut file = File::open("/proc/self/exe").map_err(io_error)?;
                let mut hasher = sha2::Sha256::new();
                let mut buffer = [0_u8; 65536];
                loop {
                    let count = file.read(&mut buffer).map_err(io_error)?;
                    if count == 0 {
                        break;
                    }
                    hasher.update(&buffer[..count]);
                }
                Ok((
                    path,
                    hasher
                        .finalize()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>(),
                ))
            })
            .clone()?;
        for version in self.inventory(false)? {
            if version.executable_sha256 == digest.as_str() {
                return Ok((self.retained(&version.tag, true)?.0.path, true));
            }
        }
        Ok((path, false))
    }
    fn activate(&self, tag: &str, points: &dyn Checkpoints) -> Result<(), Diagnostic> {
        self.recheck()?;
        let temporary = format!(
            ".lkjscript-select-{}-{}",
            self.owner.installation,
            random_id()?
        );
        rustix::fs::symlinkat(pointer_target(tag), &self.bin, &temporary).map_err(io_error)?;
        let result = (|| {
            points.at("before-pointer-replace")?;
            self.recheck()?;
            rustix::fs::renameat(&self.bin, &temporary, &self.bin, "lkjscript")
                .map_err(io_error)?;
            let sync = (|| {
                points.at("after-pointer-replace")?;
                fs::sync(&self.bin)?;
                points.at("after-pointer-sync")
            })();
            sync.map_err(|mut error| { error.notes.push(format!("selection observed as {:?}; pointer replacement committed; durability uncertain", self.pointer())); error })
        })();
        let cleanup = (|| {
            if fs::stat(&self.bin, &temporary)?.is_some() {
                fs::remove_file(&self.bin, &temporary)?;
            }
            Ok(())
        })();
        let cleanup = cleanup_location(cleanup, &self.prefix.join("bin").join(&temporary));
        combine_cleanup(result, cleanup)
    }
    fn validate_inventory(&self) -> Result<(), Diagnostic> {
        for name in fs::entries(&self.root, MAXIMUM_VERSIONS + 4)? {
            if [ROOT_METADATA, LOCK, "versions"].contains(&name.as_str()) {
                continue;
            }
            if !stage_name(&name) {
                return Err(corrupt("installation root contains foreign entries"));
            }
            let stage = fs::required_directory(&self.root, &name)?;
            let marker: StageMarker = read_json(&stage, STAGE_MARKER)?;
            if marker.format != "lkjscript-install-stage-1"
                || marker.installation != self.owner.installation
            {
                return Err(corrupt("installation stage has foreign ownership"));
            }
        }
        Ok(())
    }
    fn clean_stages(&self) -> Result<(), Diagnostic> {
        for name in fs::entries(&self.root, MAXIMUM_VERSIONS + 4)? {
            if [ROOT_METADATA, LOCK, "versions"].contains(&name.as_str()) {
                continue;
            }
            if !stage_name(&name) {
                return Err(corrupt("installation root contains foreign entries"));
            }
            let stage = fs::required_directory(&self.root, &name)?;
            let marker: StageMarker = read_json(&stage, STAGE_MARKER).map_err(|mut error| {
                error.notes.push(format!(
                    "incomplete owned stage {}; inspect and remove only this stage before retry",
                    self.prefix.join("lib/lkjscript").join(&name).display()
                ));
                error
            })?;
            if marker
                != (StageMarker {
                    format: "lkjscript-install-stage-1".to_owned(),
                    installation: self.owner.installation.clone(),
                })
            {
                return Err(corrupt(
                    "orphan stage ownership is invalid; no cleanup performed",
                ));
            }
            self.clean_stage(&name, &stage)?;
        }
        for name in fs::entries(&self.bin, 4096)? {
            if name.starts_with(&format!(".lkjscript-select-{}-", self.owner.installation)) {
                let metadata = fs::stat(&self.bin, &name)?
                    .ok_or_else(|| corrupt("selection stage disappeared"))?;
                if rustix::fs::FileType::from_raw_mode(metadata.st_mode)
                    != rustix::fs::FileType::Symlink
                {
                    return Err(corrupt("selection stage is not an owned symlink"));
                }
                read_pointer(&self.bin, &name)?;
                fs::remove_file(&self.bin, &name)?;
            }
        }
        fs::sync(&self.root)?;
        fs::sync(&self.bin)
    }
    fn clean_stage(&self, name: &str, stage: &File) -> Result<(), Diagnostic> {
        cleanup_location(
            self.clean_stage_contents(name, stage),
            &self.prefix.join("lib/lkjscript").join(name),
        )
    }
    fn clean_stage_contents(&self, name: &str, stage: &File) -> Result<(), Diagnostic> {
        let mut entries = fs::entries(stage, 4)?;
        entries.sort_by_key(|name| name == STAGE_MARKER);
        for entry in entries {
            match entry.as_str() {
                STAGE_MARKER | "archive.gz" => {
                    fs::file_at(stage, &entry)?;
                    fs::remove_file(stage, &entry)?;
                }
                "version" => {
                    let version = fs::required_directory(stage, "version")?;
                    for target in fs::entries(&version, 2)? {
                        if target != container::TARGET_TRIPLE {
                            return Err(corrupt(
                                "stage contains a foreign target; cleanup stopped",
                            ));
                        }
                        let payload = fs::required_directory(&version, &target)?;
                        for file in fs::entries(&payload, 6)? {
                            if !PAYLOADS.contains(&file.as_str()) && file != RECEIPT {
                                return Err(corrupt(
                                    "stage contains a foreign payload; cleanup stopped",
                                ));
                            }
                            fs::file_at(&payload, &file)?;
                            fs::remove_file(&payload, &file)?;
                        }
                        fs::remove_directory(&version, &target)?;
                    }
                    fs::remove_directory(stage, "version")?;
                }
                _ => return Err(corrupt("stage contains foreign entries; cleanup stopped")),
            }
        }
        fs::remove_directory(&self.root, name)?;
        fs::sync(&self.root)
    }
}

fn initialize(lib: &File, lib_path: &Path, points: &dyn Checkpoints) -> Result<File, Diagnostic> {
    let id = random_id()?;
    let stage_name = format!(".lkjscript-init-{id}");
    let stage = fs::private_directory(lib, &stage_name)?;
    let result = (|| {
        let owner = Ownership {
            format: "lkjscript-installation".to_owned(),
            version: 1,
            installation: id,
            owner_uid: rustix::process::geteuid().as_raw(),
        };
        write_json(&stage, ROOT_METADATA, &owner)?;
        fs::write_new(&stage, LOCK, b"", 0o600)?;
        fs::mkdir(&stage, "versions", 0o755)?;
        fs::sync(&stage)?;
        points.at("before-root-publish")?;
        match fs::publish(lib, &stage_name, lib, "lkjscript") {
            Ok(()) => {
                points.at("after-root-publish")?;
                fs::sync(lib)?;
                Ok(true)
            }
            Err(rustix::io::Errno::EXIST) => Ok(false),
            Err(error) => Err(io_error(error)),
        }
    })();
    let cleanup = (|| {
        if fs::stat(lib, &stage_name)?.is_some() {
            if fs::stat(&stage, "versions")?.is_some() {
                fs::remove_directory(&stage, "versions")?;
            }
            for name in [ROOT_METADATA, LOCK] {
                if fs::stat(&stage, name)?.is_some() {
                    fs::remove_file(&stage, name)?;
                }
            }
            fs::remove_directory(lib, &stage_name)?;
            fs::sync(lib)?;
        }
        Ok(())
    })();
    let cleanup = cleanup_location(cleanup, &lib_path.join(&stage_name));
    combine_cleanup(result, cleanup)?;
    let root = fs::required_directory(lib, "lkjscript")?;
    read_ownership(&root)?;
    Ok(root)
}
fn read_ownership(root: &File) -> Result<Ownership, Diagnostic> {
    fs::owned(root)?;
    let owner: Ownership = read_json(root, ROOT_METADATA).map_err(|_| source("runtime_root_conflict", "reserved product root lacks valid installation ownership metadata; choose a different prefix"))?;
    if owner.format != "lkjscript-installation"
        || owner.version != 1
        || !id_valid(&owner.installation)
        || owner.owner_uid != rustix::process::geteuid().as_raw()
    {
        return Err(source(
            "runtime_root_conflict",
            "reserved product root has foreign ownership; choose a different prefix",
        ));
    }
    let lock = fs::file_at(root, LOCK)?;
    fs::owned(&lock)?;
    if lock.metadata().map_err(io_error)?.len() != 0 {
        return Err(corrupt("installation lock file has foreign content"));
    }
    Ok(owner)
}
fn pointer_target(tag: &str) -> String {
    format!(
        "../lib/lkjscript/versions/{tag}/{}/lkjscript",
        container::TARGET_TRIPLE
    )
}
fn pointer_at(bin: &File) -> Result<Option<String>, Diagnostic> {
    if fs::stat(bin, "lkjscript")?.is_none() {
        return Ok(None);
    }
    read_pointer(bin, "lkjscript").map(Some)
}
fn read_pointer(bin: &File, name: &str) -> Result<String, Diagnostic> {
    let metadata = fs::stat(bin, name)?.ok_or_else(|| corrupt("selection pointer disappeared"))?;
    if rustix::fs::FileType::from_raw_mode(metadata.st_mode) != rustix::fs::FileType::Symlink
        || metadata.st_uid != rustix::process::geteuid().as_raw()
        || metadata.st_size > 256
    {
        return Err(source(
            "runtime_pointer_conflict",
            "bin/lkjscript is not an installation-owned relative symlink; preserve it and choose a different prefix",
        ));
    }
    let target = rustix::fs::readlinkat(bin, name, Vec::new()).map_err(io_error)?;
    let target = target
        .to_str()
        .map_err(|_| corrupt("selection pointer is not UTF-8"))?;
    let tag = target
        .strip_prefix("../lib/lkjscript/versions/")
        .and_then(|value| value.split('/').next())
        .ok_or_else(|| {
            source(
                "runtime_pointer_conflict",
                "default pointer has a foreign target; choose a different prefix",
            )
        })?;
    validate_tag(tag)?;
    if target != pointer_target(tag) {
        return Err(source(
            "runtime_pointer_conflict",
            "default pointer is not the sole canonical direct relative selection",
        ));
    }
    Ok(tag.to_owned())
}
fn check_selection(
    selected: &Option<String>,
    inventory: &[InstalledVersion],
) -> Result<(), Diagnostic> {
    if selected
        .as_ref()
        .is_some_and(|tag| !inventory.iter().any(|version| &version.tag == tag))
    {
        return Err(corrupt("selection pointer names an absent version"));
    }
    Ok(())
}
fn read_file(
    directory: &File,
    name: &str,
    limit: u64,
    mode: Option<u32>,
) -> Result<Vec<u8>, Diagnostic> {
    let mut file = fs::file_at(directory, name)?;
    fs::owned(&file)?;
    let metadata = file.metadata().map_err(io_error)?;
    if metadata.len() > limit || mode.is_some_and(|mode| metadata.mode() & 0o7777 != mode) {
        return Err(corrupt(
            "installation file exceeds its bound or has invalid mode",
        ));
    }
    container::read_bounded(&mut file, limit).map_err(format_error)
}
fn read_json<T: serde::de::DeserializeOwned + Serialize>(
    directory: &File,
    name: &str,
) -> Result<T, Diagnostic> {
    let bytes = read_file(directory, name, MAXIMUM_METADATA_BYTES, Some(0o644))?;
    let value: T = serde_json::from_slice(&bytes)
        .map_err(|error| corrupt(format!("invalid installation metadata: {error}")))?;
    if container::canonical_json(&value).map_err(format_error)? != bytes {
        return Err(corrupt("installation metadata is not canonical"));
    }
    Ok(value)
}
fn write_json(directory: &File, name: &str, value: &impl Serialize) -> Result<(), Diagnostic> {
    let bytes = container::canonical_json(value).map_err(format_error)?;
    if bytes.len() as u64 > MAXIMUM_METADATA_BYTES {
        return Err(capacity("installation metadata exceeds 2 MiB"));
    }
    fs::write_new(directory, name, &bytes, 0o644)
}
fn random_id() -> Result<String, Diagnostic> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(io_error)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
fn id_valid(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn stage_name(name: &str) -> bool {
    name.strip_prefix(".stage-").is_some_and(id_valid)
}
fn cleanup_location(result: Result<(), Diagnostic>, path: &Path) -> Result<(), Diagnostic> {
    result.map_err(|mut error| {
        error.notes.push(format!(
            "cleanup incomplete at {}; preserve unrelated contents and inspect only this owned stage before retry",
            path.display()
        ));
        error
    })
}
fn combine_cleanup<T>(
    result: Result<T, Diagnostic>,
    cleanup: Result<(), Diagnostic>,
) -> Result<T, Diagnostic> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(mut cleanup)) => {
            cleanup
                .notes
                .push("operation may have committed; inspect runtime list before retry".to_owned());
            Err(cleanup)
        }
        (Err(mut error), Err(cleanup)) => {
            error
                .notes
                .push(format!("required cleanup also failed: {cleanup}"));
            error.notes.extend(cleanup.notes);
            Err(error)
        }
    }
}
fn source(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}
fn corrupt(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, "runtime_corrupt", message)
}
fn capacity(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, "runtime_capacity", message)
}
fn io_error(error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        "runtime_io",
        error.to_string(),
    )
}
fn format_error(error: container::ContainerError) -> Diagnostic {
    if error.infrastructure {
        io_error(error)
    } else {
        Diagnostic::new(DiagnosticClass::Corrupt, "runtime_archive", error.message)
    }
}

#[cfg(test)]
mod tests;
