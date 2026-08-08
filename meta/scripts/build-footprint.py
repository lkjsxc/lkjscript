#!/usr/bin/env python3
"""Measure reproducible cold/warm lkjscript release builds and binary size."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path


def output(command: list[str]) -> str:
    return subprocess.check_output(command, text=True).strip()


def percentile_95(values: list[float | int]) -> float | int:
    ordered = sorted(values)
    return ordered[math.ceil(0.95 * len(ordered)) - 1]


def controlled_build_environment() -> tuple[dict[str, str], dict[str, object]]:
    environment = os.environ.copy()
    exact = {
        "CARGO_BUILD_JOBS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_TARGET",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_INCREMENTAL",
        "CARGO_TARGET_DIR",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTDOCFLAGS",
        "RUSTFLAGS",
        "SOURCE_DATE_EPOCH",
    }
    scrubbed = sorted(
        name
        for name in environment
        if name in exact
        or name.startswith("CARGO_PROFILE_RELEASE_")
        or name.startswith("CARGO_TARGET_")
    )
    for name in scrubbed:
        environment.pop(name, None)
    return environment, {
        "scrubbed_overrides": scrubbed,
        "cargo_home": environment.get("CARGO_HOME", str(Path.home() / ".cargo")),
        "rustup_home": environment.get("RUSTUP_HOME", str(Path.home() / ".rustup")),
        "cargo_executable": shutil.which("cargo", path=environment.get("PATH")),
        "rustc_executable": shutil.which("rustc", path=environment.get("PATH")),
    }


def publish_json_exclusive(path: Path, record: dict[str, object]) -> None:
    payload = json.dumps(record, indent=2) + "\n"
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError as error:
            raise RuntimeError(f"refusing to overwrite existing output: {path}") from error
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        temporary.unlink(missing_ok=True)


def worktree_identity(root: Path) -> dict[str, object]:
    tracked = subprocess.check_output(["git", "diff", "--binary", "HEAD"], cwd=root)
    untracked_output = subprocess.check_output(
        ["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=root
    )
    untracked = {}
    for raw in untracked_output.split(b"\0"):
        if not raw:
            continue
        relative = Path(os.fsdecode(raw))
        if relative.parts[0] in {"target", ".pi-subagents"} or "__pycache__" in relative.parts:
            continue
        path = root / relative
        if path.is_file():
            untracked[relative.as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    combined = hashlib.sha256()
    combined.update(tracked)
    for path, digest in sorted(untracked.items()):
        combined.update(path.encode())
        combined.update(b"\0")
        combined.update(digest.encode())
        combined.update(b"\0")
    return {
        "tracked_diff_sha256": hashlib.sha256(tracked).hexdigest(),
        "untracked_files": untracked,
        "combined_sha256": combined.hexdigest(),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", required=True)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    if arguments.samples < 1:
        parser.error("--samples must be positive")

    root = Path(__file__).resolve().parents[2]
    command = [
        "cargo",
        "build",
        "--locked",
        "--release",
        "-p",
        "lkjscript-app",
        "--bin",
        "lkjscript",
    ]
    cpu_model = next(
        (
            line.split(":", 1)[1].strip()
            for line in Path("/proc/cpuinfo").read_text().splitlines()
            if line.startswith("model name")
        ),
        None,
    )
    memory_kib = next(
        int(line.split()[1])
        for line in Path("/proc/meminfo").read_text().splitlines()
        if line.startswith("MemTotal:")
    )
    build_environment, build_environment_record = controlled_build_environment()
    record = {
        "label": arguments.label,
        "base_commit": output(["git", "-C", str(root), "rev-parse", "HEAD"]),
        "worktree": worktree_identity(root),
        "uname": platform.uname()._asdict(),
        "cpu_model": cpu_model,
        "logical_cpus": os.cpu_count(),
        "memory_bytes": memory_kib * 1024,
        "rustc": output(["rustc", "--version", "--verbose"]),
        "cargo": output(["cargo", "--version"]),
        "profile": "release (workspace LTO, codegen-units=1, strip=symbols)",
        "build_environment": build_environment_record,
        "command": " ".join(command),
        "cold_definition": "fresh empty CARGO_TARGET_DIR; Cargo registry/cache retained",
        "warm_definition": "immediate unchanged no-op rebuild in the same target directory",
        "samples": [],
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    if arguments.output.exists():
        parser.error(f"refusing to overwrite existing output: {arguments.output}")
    for index in range(1, arguments.samples + 1):
        target = Path(tempfile.mkdtemp(prefix=f"lkjscript-{arguments.label}-{index}-"))
        environment = build_environment.copy()
        environment["CARGO_TARGET_DIR"] = str(target)
        sample: dict[str, float | int] = {"index": index}
        try:
            for state in ("cold", "warm"):
                log = arguments.output.parent / f"{arguments.label}-{state}-{index}.log"
                if log.exists():
                    parser.error(f"refusing to overwrite existing log: {log}")
                started = time.perf_counter()
                with log.open("x") as stream:
                    result = subprocess.run(
                        command,
                        cwd=root,
                        env=environment,
                        stdout=stream,
                        stderr=subprocess.STDOUT,
                        check=False,
                    )
                sample[f"{state}_seconds"] = time.perf_counter() - started
                if result.returncode:
                    raise SystemExit(f"{state} sample {index} failed; see {log}")
            binary = target / "release/lkjscript"
            sample["binary_bytes"] = binary.stat().st_size
            sample["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
            record["samples"].append(sample)
        finally:
            shutil.rmtree(target)

    for key in ("cold_seconds", "warm_seconds", "binary_bytes"):
        values = [sample[key] for sample in record["samples"]]
        record[key] = {
            "samples": values,
            "median": statistics.median(values),
            "p95_nearest_rank": percentile_95(values),
        }
    digests = sorted({sample["binary_sha256"] for sample in record["samples"]})
    record["binary_sha256"] = digests[0] if len(digests) == 1 else digests
    publish_json_exclusive(arguments.output, record)
    print(arguments.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
