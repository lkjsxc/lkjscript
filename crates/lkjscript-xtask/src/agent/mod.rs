mod bounds;
mod checkpoint_lock;
mod compact;
mod context;
mod git;
mod history;
mod json;
mod model;
mod publication_rollback;
mod quarantine;
mod references;
mod storage;
mod validate;

#[cfg(test)]
mod tests;

use std::path::Path;

pub fn run(root: &Path, args: &[String]) -> i32 {
    let result = match args.first().map(String::as_str) {
        Some("checkpoint") if args.len() == 2 => checkpoint(root, Path::new(&args[1])),
        Some("resume-context") if valid_resume_args(&args[1..]) => resume(root, &args[1..]),
        Some("validate-state") if args.len() == 2 => validate_state(root, &args[1]),
        Some("compact-state") if args.len() == 2 => compact_state(root, &args[1]),
        _ => {
            usage();
            return 2;
        }
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn valid_resume_args(args: &[String]) -> bool {
    matches!(args, [_])
        || matches!(
            args,
            [_, flag, profile]
                if flag == "--profile" && matches!(profile.as_str(), "weak" | "strong")
        )
}

fn checkpoint(root: &Path, request_path: &Path) -> Result<(), String> {
    let request = json::read_request(request_path)?;
    validate::task_id(&request.state.task_id)?;
    let _lock = checkpoint_lock::acquire(root, &request.state.task_id)?;
    let current = storage::load(root, &request.state.task_id)?;
    validate::checkpoint(root, &request, current.as_ref())?;
    let bytes = json::encode_state(&request.state)?;
    storage::write_state(root, &request.state.task_id, &bytes)?;
    println!(
        "checkpointed {} revision {}",
        request.state.task_id, request.state.state_revision
    );
    Ok(())
}

fn resume(root: &Path, args: &[String]) -> Result<(), String> {
    let (task_id, profile) = match args {
        [task] => (task.as_str(), "weak"),
        [task, flag, profile]
            if flag == "--profile" && matches!(profile.as_str(), "weak" | "strong") =>
        {
            (task.as_str(), profile.as_str())
        }
        _ => return Err("usage: agent resume-context <task-id> [--profile weak|strong]".into()),
    };
    validate::task_id(task_id)?;
    let _lock = checkpoint_lock::acquire(root, task_id)?;
    let response = context::build(root, task_id, profile)?;
    crate::util::print_json(&response)
}

fn validate_state(root: &Path, task_id: &str) -> Result<(), String> {
    validate::task_id(task_id)?;
    let _lock = checkpoint_lock::acquire(root, task_id)?;
    let state = storage::load(root, task_id)?.ok_or("task state does not exist")?;
    validate::validate(root, &state, true)?;
    println!("validated {task_id} revision {}", state.state_revision);
    Ok(())
}

fn compact_state(root: &Path, task_id: &str) -> Result<(), String> {
    validate::task_id(task_id)?;
    let changed = compact::run(root, task_id)?;
    println!(
        "{} {task_id}",
        if changed {
            "compacted"
        } else {
            "already compact"
        }
    );
    Ok(())
}

fn usage() {
    eprintln!(
        "usage: agent [checkpoint <request.json>|resume-context <task-id> [--profile weak|strong]|\
         validate-state <task-id>|compact-state <task-id>]"
    );
}
