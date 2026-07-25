"""Reproducible host and source-closure evidence."""

import hashlib
import pathlib
import platform

from brainfuck_protocol.constants import REFERENCE_PATH
from brainfuck_protocol.files import sha256_file
from brainfuck_protocol.process import checked_output

def first_matching_line(path: pathlib.Path, prefix: str) -> str:
    try:
        for line in path.read_text(errors="replace").splitlines():
            if line.startswith(prefix):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return "unknown"


def source_closure_hash(root: pathlib.Path, paths: list[pathlib.Path]) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths):
        digest.update(str(path.relative_to(root)).encode())
        digest.update(b"\x00")
        digest.update(path.read_bytes())
        digest.update(b"\x00")
    return digest.hexdigest()


def workload_hash(source_dir: pathlib.Path) -> str:
    return source_closure_hash(
        source_dir, list(source_dir.rglob("*.lkjscript"))
    )


def machine_metadata(
    root: pathlib.Path, binary: pathlib.Path
) -> dict[str, object]:
    status = checked_output(["git", "status", "--short"], root)
    memory_kib = first_matching_line(pathlib.Path("/proc/meminfo"), "MemTotal")
    cpu = first_matching_line(pathlib.Path("/proc/cpuinfo"), "model name")
    return {
        "repository_commit": checked_output(["git", "rev-parse", "HEAD"], root),
        "tree_state": "clean" if not status else "dirty",
        "git_status_short": status.splitlines(),
        "interpreter_source_sha256": workload_hash(
            root / "src/examples/brainfuck"
        ),
        "release_binary_sha256": sha256_file(binary),
        "harness_sha256": source_closure_hash(
            root,
            [
                root / "meta/benchmarks/brainfuck/benchmark.py",
                *list(
                    (root / "meta/benchmarks/brainfuck/brainfuck_protocol").glob(
                        "*.py"
                    )
                ),
            ],
        ),
        "reference_source_sha256": sha256_file(root / REFERENCE_PATH),
        "cpu": cpu,
        "ram": memory_kib,
        "operating_system": platform.platform(),
        "kernel": platform.release(),
        "machine": platform.machine(),
        "rustc": checked_output(["rustc", "--version"], root),
        "cargo": checked_output(["cargo", "--version"], root),
        "python": platform.python_version(),
    }
