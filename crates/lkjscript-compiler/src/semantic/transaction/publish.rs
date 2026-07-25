use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};
use crate::semantic::transaction::{StagedSource, StagedTransaction};

pub(crate) fn publish(transaction: &StagedTransaction, root: &Path) -> Result<(), ProtocolError> {
    let id = super::journal::transaction_id(transaction);
    let staging_root = root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("target/lkjscript/staging");
    fs::create_dir_all(&staging_root)
        .map_err(|failure| publication("create staging journal directory", failure))?;
    let journal = staging_root.join(format!("{id}.journal"));
    super::journal::write(&journal, transaction, "prepared")?;
    let mut files = prepare_files(&transaction.sources, &id)?;
    let mut published = 0_usize;
    let result = (|| {
        for file in &mut files {
            fs::rename(&file.source.host_path, &file.backup)
                .map_err(|failure| publication("rename original to recovery backup", failure))?;
            file.backed_up = true;
            fs::rename(&file.temporary, &file.source.host_path)
                .map_err(|failure| publication("atomically install staged source", failure))?;
            published += 1;
            sync_parent(&file.source.host_path)?;
        }
        Ok(())
    })();
    if let Err(failure) = result {
        let rollback = rollback(&mut files);
        let _ = super::journal::write(&journal, transaction, "rolled_back");
        return match rollback {
            Ok(()) => Err(failure),
            Err(rollback_failure) => Err(error(
                ProtocolErrorCode::PublicationFailed,
                format!("{failure:?}; rollback failed: {}", rollback_failure.message),
            )),
        };
    }
    if let Err(failure) =
        super::journal::write(&journal, transaction, &format!("committed:{published}"))
    {
        let rollback = rollback(&mut files);
        return rollback.map_or_else(Err, |()| Err(failure));
    }
    for file in &files {
        let _ = fs::remove_file(&file.backup);
    }
    Ok(())
}

struct PublicationFile<'a> {
    source: &'a StagedSource,
    temporary: PathBuf,
    backup: PathBuf,
    backed_up: bool,
}

fn prepare_files<'a>(
    sources: &'a [StagedSource],
    id: &str,
) -> Result<Vec<PublicationFile<'a>>, ProtocolError> {
    let mut output = Vec::with_capacity(sources.len());
    for (index, source) in sources.iter().enumerate() {
        let current = fs::read(&source.host_path)
            .map_err(|failure| publication("reread source precondition", failure))?;
        if current != source.old_bytes {
            cleanup_temporaries(&output);
            return Err(error(
                ProtocolErrorCode::PreconditionFailed,
                format!("source {} changed before publication", source.logical_path),
            ));
        }
        let directory = source.host_path.parent().ok_or_else(|| {
            error(
                ProtocolErrorCode::PublicationFailed,
                "source has no containing directory",
            )
        })?;
        let leaf = source
            .host_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                error(
                    ProtocolErrorCode::PublicationFailed,
                    "source filename is not UTF-8",
                )
            })?;
        let temporary = directory.join(format!(".{leaf}.lkjscript-stage-{id}-{index}"));
        let backup = directory.join(format!(".{leaf}.lkjscript-backup-{id}-{index}"));
        if temporary.exists() || backup.exists() {
            cleanup_temporaries(&output);
            return Err(error(
                ProtocolErrorCode::PublicationFailed,
                "deterministic publication artifact already exists; recovery is required",
            ));
        }
        let mut file = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(failure) => {
                cleanup_temporaries(&output);
                return Err(publication("create same-directory staged source", failure));
            }
        };
        if let Err(failure) = file
            .write_all(&source.new_bytes)
            .and_then(|()| file.sync_all())
        {
            let _ = fs::remove_file(&temporary);
            cleanup_temporaries(&output);
            return Err(publication("flush same-directory staged source", failure));
        }
        output.push(PublicationFile {
            source,
            temporary,
            backup,
            backed_up: false,
        });
    }
    Ok(output)
}

fn rollback(files: &mut [PublicationFile<'_>]) -> Result<(), ProtocolError> {
    let mut first_error = None;
    for file in files.iter_mut().rev() {
        if file.backed_up {
            if let Err(failure) = fs::rename(&file.backup, &file.source.host_path) {
                first_error.get_or_insert_with(|| publication("restore recovery backup", failure));
            }
        }
        let _ = fs::remove_file(&file.temporary);
        let _ = sync_parent(&file.source.host_path);
    }
    first_error.map_or(Ok(()), Err)
}

fn cleanup_temporaries(files: &[PublicationFile<'_>]) {
    for file in files {
        let _ = fs::remove_file(&file.temporary);
    }
}

fn sync_parent(path: &Path) -> Result<(), ProtocolError> {
    let parent = path.parent().ok_or_else(|| {
        error(
            ProtocolErrorCode::PublicationFailed,
            "source has no parent directory",
        )
    })?;
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|failure| publication("flush source directory", failure))
}

fn publication(action: &str, failure: std::io::Error) -> ProtocolError {
    error(
        ProtocolErrorCode::PublicationFailed,
        format!("{action}: {failure}"),
    )
}
