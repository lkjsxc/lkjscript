use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    let action = args.get(1).map(String::as_str).unwrap_or("");
    let root = args.get(2).map(String::as_str).unwrap_or(".");
    if args.len() > 3 || !matches!(action, "lock" | "check") {
        return Err("usage: lkjscript package <lock|check> [package-path]".into());
    }
    match action {
        "lock" => {
            let (path, bytes) = lkjscript_compiler::package::create_lock(Path::new(root))
                .map_err(|error| error.to_string())?;
            atomic_write(&path, &bytes)?;
            println!("locked {}", path.display());
        }
        "check" => {
            lkjscript_compiler::package::verify(Path::new(root))
                .map_err(|error| error.to_string())?;
        }
        _ => return Err("package action must be lock or check".into()),
    }
    Ok(ExitCode::SUCCESS)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("package lock has no parent directory")?;
    let temporary = parent.join(format!(".lkjscript-lock-{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("create {}: {error}", temporary.display()))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("write {}: {error}", temporary.display()))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("install {}: {error}", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
