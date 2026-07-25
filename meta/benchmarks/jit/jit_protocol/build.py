"""Locked release build evidence."""

import subprocess
import time
from pathlib import Path
from typing import Any

def locked_release_build(root: Path) -> dict[str, Any]:
    command = ["cargo", "build", "--locked", "--workspace", "--release"]
    started = time.monotonic_ns()
    completed = subprocess.run(
        command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    wall_ns = time.monotonic_ns() - started
    if completed.returncode != 0:
        raise RuntimeError(
            f"locked release build failed: stdout={completed.stdout!r} stderr={completed.stderr!r}"
        )
    return {
        "command": command,
        "exit_status": completed.returncode,
        "process_wall_ns": wall_ns,
        "stdout_bytes": len(completed.stdout),
        "stderr_bytes": len(completed.stderr),
    }
