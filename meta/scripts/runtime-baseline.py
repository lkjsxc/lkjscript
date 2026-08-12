#!/usr/bin/env python3
"""Measure the selected lkjscript product path across representative workloads."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shutil
import sqlite3
import statistics
import subprocess
import tempfile
import time
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
METRICS_PREFIX = "LKJSCRIPT_METRICS "
METRICS_SCHEMA = "lkjscript.metrics"
METRICS_CONTRACT = "ae737b579e63cbf518ed4d917698f4222e8752e4471dfb1bb11d6ef3f3e638ec"
EMPTY_SHA256 = hashlib.sha256(b"").hexdigest()
RELEASE_PROFILE = "release (workspace LTO, codegen-units=1, strip=symbols)"


@dataclass(frozen=True)
class Workload:
    name: str
    path: str
    families: tuple[str, ...]
    status: int
    outcome_kind: str
    outcome_value: tuple[str, str] | None
    stdout_sha256: str
    stderr_sha256: str = EMPTY_SHA256
    effect_name: str | None = None
    file_result: bytes | None = None
    sqlite_result: int | None = None
    execution_path: str | None = None
    native_entered: bool | None = None
    require_unique_cleanup: bool = False


WORKLOADS = (
    Workload(
        "scalar-loop",
        "crates/lkjscript-app/tests/fixtures/scalar-loop.lkjscript",
        ("scalar", "comparisons", "branches", "loops", "direct-calls"),
        0,
        "returned",
        ("exact", "0"),
        EMPTY_SHA256,
        execution_path="baseline-native",
        native_entered=True,
    ),
    Workload(
        "scalar-calls",
        "src/examples/scalar-calls/main.lkjscript",
        ("scalar", "comparisons", "loops", "direct-calls", "numeric-conversion"),
        0,
        "returned",
        ("exact", "0x401af3ef5a48f5f0"),
        EMPTY_SHA256,
        execution_path="baseline-native",
        native_entered=True,
    ),
    Workload(
        "scalar-redundancy",
        "src/examples/scalar-redundancy/main.lkjscript",
        ("scalar", "loops", "wider-reachable-call-group"),
        0,
        "returned",
        ("exact", "3333"),
        EMPTY_SHA256,
        execution_path="baseline-native",
        native_entered=True,
    ),
    Workload(
        "product-list",
        "crates/lkjscript-app/tests/fixtures/allocation-graph.lkjscript",
        ("products", "lists", "structural-operations"),
        0,
        "returned",
        ("exact", "42"),
        EMPTY_SHA256,
        execution_path="baseline-native",
        native_entered=True,
    ),
    Workload(
        "enum-match",
        "crates/lkjscript-app/tests/fixtures/enum-match.lkjscript",
        ("enums", "matching", "structural-operations"),
        0,
        "returned",
        ("exact", "42"),
        EMPTY_SHA256,
        execution_path="baseline-native",
        native_entered=True,
    ),
    Workload(
        "ownership-control",
        "crates/lkjscript-app/tests/fixtures/ownership-control.lkjscript",
        ("byte-vectors", "borrowing", "cleanup", "early-return"),
        0,
        "returned",
        ("exact", "7"),
        EMPTY_SHA256,
        execution_path="baseline-native",
        native_entered=True,
        require_unique_cleanup=True,
    ),
    Workload(
        "checked-failure",
        "crates/lkjscript-app/tests/fixtures/checked-failure.lkjscript",
        ("checked-failure", "entered-native-trap"),
        1,
        "trapped",
        ("detail", "div: I64 division by zero"),
        EMPTY_SHA256,
        stderr_sha256="5726afb505adb497a8210955a2f8d223b684f2c516ad4c8273f724182583881a",
        execution_path="baseline-native",
        native_entered=True,
    ),
    Workload(
        "hello",
        "src/examples/hello/main.lkjscript",
        ("recursive-calls", "strings", "stdio"),
        0,
        "returned",
        ("exact", "unit"),
        "4b8c4c67c5066c2d71cc7650eeb3f6e774e9cbbcea3252eacba190af6e87b0f9",
        execution_path="vm-fallback",
        native_entered=False,
    ),
    Workload(
        "bench",
        "src/examples/bench/main.lkjscript",
        ("application-loop", "floating-point", "direct-calls", "stdio"),
        0,
        "returned",
        ("exact", "unit"),
        "414920863f63e5d9c1179704e1905d9ba2b10498fbf1c33231c186e0ea0dedf0",
        execution_path="vm-fallback",
        native_entered=False,
    ),
    Workload(
        "mandel",
        "src/examples/mandel/main.lkjscript",
        ("application-branches", "floating-point", "wider-call-group", "stdio"),
        0,
        "returned",
        ("exact", "unit"),
        "222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907",
        execution_path="vm-fallback",
        native_entered=False,
    ),
    Workload(
        "polymorphic-transport",
        "src/examples/polymorphic-transport/main.lkjscript",
        ("generic-calls", "products", "structural-ownership"),
        0,
        "returned",
        ("exact", "42"),
        EMPTY_SHA256,
        execution_path="vm-fallback",
        native_entered=False,
    ),
    Workload(
        "bytes-hash",
        "src/examples/sha256/main.lkjscript",
        ("bytes", "byte-vectors", "borrowing", "stdio"),
        0,
        "returned",
        ("exact", "unit"),
        "2811745d7b8d8874f6e653d176cefdd19e05e920ce389b9b7e83e5b2dfa546c7",
        execution_path="vm-fallback",
        native_entered=False,
    ),
    Workload(
        "bulk-bytes-filesystem",
        "src/examples/bulk-bytes/main.lkjscript",
        ("strings", "bytes", "movement", "filesystem", "stdio"),
        0,
        "returned",
        ("exact", "unit"),
        "d7bfed85b17b8187c981ac6c952fe8af6903e4179d3d1e4b6970ced1297565b9",
        effect_name="bulk-bytes.txt",
        file_result="exact bytes: é".encode(),
        execution_path="vm-fallback",
        native_entered=False,
    ),
    Workload(
        "durable-filesystem",
        "src/examples/durable-files/main.lkjscript",
        ("resources", "cleanup", "filesystem", "stdio"),
        0,
        "returned",
        ("exact", "unit"),
        "e7f6c011776e8db7cd330b54174fd76f7d0216b612387a5ffcfb81e6f0919683",
        effect_name="durable-records.txt",
        file_result=b"record",
        execution_path="vm-fallback",
        native_entered=False,
    ),
    Workload(
        "sqlite",
        "src/examples/sqlite/main.lkjscript",
        ("resources", "cleanup", "sqlite", "filesystem", "stdio"),
        0,
        "returned",
        ("exact", "unit"),
        "73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049",
        effect_name="sqlite-example.db",
        sqlite_result=42,
        execution_path="vm-fallback",
        native_entered=False,
    ),
)


def command_output(*command: str) -> str:
    return subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ).stdout.strip()


def controlled_build_environment() -> tuple[dict[str, str], dict[str, Any]]:
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


def publish_json_exclusive(path: Path, record: dict[str, Any]) -> None:
    payload = json.dumps(record, indent=2, sort_keys=True) + "\n"
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


def generated_path(path: str) -> bool:
    parts = Path(path).parts
    return (
        path.startswith((".pi-subagents/", "target/"))
        or "__pycache__" in parts
        or path.endswith(".pyc")
    )


def worktree_metadata() -> dict[str, Any]:
    status_output = subprocess.run(
        ["git", "status", "--short", "--untracked-files=all"],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout
    status = [
        line for line in status_output.splitlines() if not generated_path(line[3:])
    ]
    tracked_diff = subprocess.run(
        ["git", "diff", "--binary", "HEAD", "--"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    untracked_output = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard", "-z"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    untracked = sorted(
        path.decode("utf-8")
        for path in untracked_output.split(b"\0")
        if path and not generated_path(path.decode("utf-8"))
    )
    digest = hashlib.sha256()
    digest.update(b"lkjscript.runtime-baseline-worktree\0")
    digest.update(tracked_diff)
    untracked_hashes = {}
    for relative in untracked:
        content = (ROOT / relative).read_bytes()
        content_hash = hashlib.sha256(content).hexdigest()
        untracked_hashes[relative] = content_hash
        digest.update(relative.encode())
        digest.update(b"\0")
        digest.update(content)
    return {
        "dirty": bool(status),
        "status": status,
        "tracked_diff_sha256": hashlib.sha256(tracked_diff).hexdigest(),
        "untracked_sha256": untracked_hashes,
        "combined_sha256": digest.hexdigest(),
    }


def machine_metadata() -> dict[str, Any]:
    cpu = "unknown"
    for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines():
        if line.startswith("model name"):
            cpu = line.split(":", 1)[1].strip()
            break
    memory_bytes = None
    for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
        if line.startswith("MemTotal:"):
            memory_bytes = int(line.split()[1]) * 1024
            break
    return {
        "hostname": platform.node(),
        "uname": platform.uname()._asdict(),
        "os": platform.platform(),
        "architecture": platform.machine(),
        "cpu": cpu,
        "configured_logical_cpus": os.cpu_count(),
        "available_logical_cpus": len(os.sched_getaffinity(0)),
        "memory_bytes": memory_bytes,
        "rustc": command_output("rustc", "--version", "--verbose"),
        "cargo": command_output("cargo", "--version"),
    }


def child_pids(pid: int) -> list[int]:
    path = Path("/proc") / str(pid) / "task" / str(pid) / "children"
    try:
        return [int(value) for value in path.read_text().split()]
    except (FileNotFoundError, PermissionError, ValueError):
        return []


def resident_bytes(pid: int) -> int:
    try:
        for line in (Path("/proc") / str(pid) / "status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except (FileNotFoundError, PermissionError, ValueError, IndexError):
        pass
    return 0


def process_tree_rss_bytes(root_pid: int) -> int:
    total = 0
    pending = [root_pid]
    visited = set()
    while pending:
        pid = pending.pop()
        if pid in visited:
            continue
        visited.add(pid)
        total += resident_bytes(pid)
        pending.extend(child_pids(pid))
    return total


def owned_sample_paths(effect_path: Path | None, metrics_path: Path) -> list[Path]:
    paths = [] if effect_path is None else [effect_path]
    if effect_path is not None and effect_path.suffix == ".db":
        paths.extend(
            Path(f"{effect_path}{suffix}") for suffix in ("-journal", "-wal", "-shm")
        )
    paths.append(metrics_path)
    return paths


def require_absent(paths: list[Path]) -> None:
    existing = [str(path) for path in paths if os.path.lexists(path)]
    if existing:
        raise RuntimeError(
            "refusing to overwrite or delete pre-existing measurement paths: "
            + ", ".join(existing)
        )


def remove_owned(paths: list[Path]) -> None:
    for path in paths:
        try:
            path.unlink()
        except FileNotFoundError:
            pass


def validate_effect(workload: Workload, effect_path: Path | None) -> None:
    if workload.file_result is not None:
        if effect_path is None:
            raise RuntimeError(f"{workload.name} has no private effect path")
        actual = effect_path.read_bytes()
        if actual != workload.file_result:
            raise RuntimeError(
                f"{workload.name} wrote {actual!r}; expected {workload.file_result!r}"
            )
    if workload.sqlite_result is not None:
        if effect_path is None:
            raise RuntimeError(f"{workload.name} has no private SQLite path")
        with sqlite3.connect(effect_path) as database:
            rows = database.execute("SELECT number FROM sample").fetchall()
            columns = database.execute("PRAGMA table_info(sample)").fetchall()
        expected_rows = [(workload.sqlite_result,)]
        expected_columns = [(0, "number", "INTEGER", 0, None, 0)]
        if rows != expected_rows or columns != expected_columns:
            raise RuntimeError(
                f"{workload.name} SQLite rows/schema are {rows!r}/{columns!r}; "
                f"expected {expected_rows!r}/{expected_columns!r}"
            )


def parse_metrics(path: Path) -> dict[str, Any]:
    line = path.read_text(encoding="utf-8")
    if not line.startswith(METRICS_PREFIX):
        raise RuntimeError(f"metrics file has no {METRICS_PREFIX.strip()} marker: {path}")
    metrics = json.loads(line[len(METRICS_PREFIX) :])
    if metrics.get("schema") != METRICS_SCHEMA:
        raise RuntimeError(f"metrics file has unknown schema: {metrics.get('schema')!r}")
    if metrics.get("contract") != METRICS_CONTRACT:
        raise RuntimeError(
            f"metrics file has unknown contract: {metrics.get('contract')!r}"
        )
    return metrics


def run_sample(
    binary: Path,
    workload: Workload,
    metrics_path: Path,
    effect_root: Path,
    poll_seconds: float,
) -> dict[str, Any]:
    effect_path = (
        None if workload.effect_name is None else effect_root / workload.effect_name
    )
    owned_paths = owned_sample_paths(effect_path, metrics_path)
    require_absent(owned_paths)
    environment = os.environ.copy()
    for name in (
        "LKJSCRIPT_JIT_DIAGNOSTICS",
        "LKJSCRIPT_JIT_DUMP_DIR",
        "LKJSCRIPT_METRICS",
        "LKJSCRIPT_METRICS_FILE",
    ):
        environment.pop(name, None)
    environment["LKJSCRIPT_METRICS_FILE"] = str(metrics_path)
    command = [str(binary), "run", workload.path]
    if effect_path is not None:
        command.extend(("--", str(effect_path)))
    process: subprocess.Popen[bytes] | None = None
    try:
        started = time.monotonic_ns()
        process = subprocess.Popen(
            command,
            cwd=ROOT,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        peak_rss = 0
        while process.poll() is None:
            peak_rss = max(peak_rss, process_tree_rss_bytes(process.pid))
            time.sleep(poll_seconds)
        stdout, stderr = process.communicate()
        wall_ns = time.monotonic_ns() - started
        metrics = parse_metrics(metrics_path)
        if process.returncode != workload.status:
            raise RuntimeError(
                f"{workload.name} exited {process.returncode}; expected {workload.status}; "
                f"stderr={stderr.decode(errors='replace')!r}"
            )
        stdout_digest = hashlib.sha256(stdout).hexdigest()
        if stdout_digest != workload.stdout_sha256:
            raise RuntimeError(
                f"{workload.name} stdout SHA-256 {stdout_digest}; "
                f"expected {workload.stdout_sha256}"
            )
        stderr_digest = hashlib.sha256(stderr).hexdigest()
        if stderr_digest != workload.stderr_sha256:
            raise RuntimeError(
                f"{workload.name} stderr SHA-256 {stderr_digest}; "
                f"expected {workload.stderr_sha256}"
            )
        outcome = metrics.get("outcome", {})
        if outcome.get("kind") != workload.outcome_kind:
            raise RuntimeError(
                f"{workload.name} outcome {outcome!r}; expected {workload.outcome_kind}"
            )
        if workload.outcome_value is not None:
            field, expected = workload.outcome_value
            if outcome.get(field) != expected:
                raise RuntimeError(
                    f"{workload.name} outcome {field}={outcome.get(field)!r}; expected {expected!r}"
                )
        if (
            workload.execution_path is not None
            and metrics.get("execution_path") != workload.execution_path
        ):
            raise RuntimeError(
                f"{workload.name} selected {metrics.get('execution_path')!r}; "
                f"expected {workload.execution_path!r}"
            )
        if (
            workload.native_entered is not None
            and metrics.get("native_entered") is not workload.native_entered
        ):
            raise RuntimeError(
                f"{workload.name} native_entered={metrics.get('native_entered')!r}; "
                f"expected {workload.native_entered!r}"
            )
        runtime = metrics.get("native_runtime")
        entry_validation = "unavailable-in-metrics"
        cleanup_validation = "not-requested"
        if isinstance(runtime, dict) and workload.native_entered is not None:
            expected_invocations = 1 if workload.native_entered else 0
            if int(runtime.get("invocations", -1)) != expected_invocations:
                raise RuntimeError(
                    f"{workload.name} native invocation count is {runtime!r}; "
                    f"expected {expected_invocations}"
                )
            entries = int(runtime.get("entries", -1))
            if (workload.native_entered and entries < 1) or (
                not workload.native_entered and entries != 0
            ):
                raise RuntimeError(f"{workload.name} native entry count is {runtime!r}")
            entry_validation = "verified"
        if workload.require_unique_cleanup:
            if isinstance(runtime, dict) and "unique_drops" in runtime:
                expected_unique = {
                    "unique_allocations": 1,
                    "unique_drops": 1,
                    "unique_cleanup_attempts": 0,
                    "unique_cleanup_releases": 0,
                    "unique_live_owners": 0,
                    "unique_live_loans": 0,
                    "unique_release_backlog": 0,
                    "unique_teardown_failures": 0,
                }
                for field, expected in expected_unique.items():
                    if int(runtime[field]) != expected:
                        raise RuntimeError(
                            f"{workload.name} recorded {field}={runtime[field]!r}; "
                            f"expected {expected}"
                        )
                cleanup_validation = "verified"
            else:
                cleanup_validation = "unavailable-in-metrics"
        validate_effect(workload, effect_path)
        compile_ns = int(metrics["timings_ns"]["compile_total"])
        execution_ns = int(metrics["timings_ns"]["total"])
        outside_ns = wall_ns - compile_ns - execution_ns
        if outside_ns < 0:
            raise RuntimeError(
                f"{workload.name} compiler and execution timers exceed process wall time"
            )
        return {
            "process_wall_ns": wall_ns,
            "outside_compile_execution_ns": outside_ns,
            "timer_decomposition_consistent": outside_ns >= 0,
            "process_tree_peak_rss_bytes": peak_rss,
            "exit_status": process.returncode,
            "stdout_bytes": len(stdout),
            "stdout_sha256": stdout_digest,
            "stderr": stderr.decode(errors="replace"),
            "stderr_sha256": stderr_digest,
            "semantic_checks": {
                "execution_path": "verified",
                "native_entry_counters": entry_validation,
                "unique_cleanup": cleanup_validation,
                "effects": "verified" if workload.file_result or workload.sqlite_result else "not-applicable",
            },
            "metrics": metrics,
        }
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        remove_owned(owned_paths)


def nearest_rank_p95(values: list[int]) -> int:
    ordered = sorted(values)
    return ordered[math.ceil(0.95 * len(ordered)) - 1]


def distribution(values: list[int]) -> dict[str, Any]:
    return {
        "samples": values,
        "median": statistics.median(values),
        "p95_nearest_rank": nearest_rank_p95(values),
    }


def numeric_metric_summary(samples: list[dict[str, Any]], path: tuple[str, ...]) -> dict[str, Any] | None:
    values = []
    for sample in samples:
        value: Any = sample["metrics"]
        for component in path:
            if not isinstance(value, dict) or component not in value:
                return None
            value = value[component]
        if value is None:
            return None
        if not isinstance(value, int):
            return None
        values.append(value)
    return distribution(values)


def summarize(samples: list[dict[str, Any]]) -> dict[str, Any]:
    summary: dict[str, Any] = {
        "process_wall_ns": distribution([item["process_wall_ns"] for item in samples]),
        "outside_compile_execution_ns": distribution(
            [item["outside_compile_execution_ns"] for item in samples]
        ),
        "process_tree_peak_rss_bytes": distribution(
            [item["process_tree_peak_rss_bytes"] for item in samples]
        ),
    }
    timings = sorted(samples[0]["metrics"]["timings_ns"])
    summary["timings_ns"] = {
        name: numeric_metric_summary(samples, ("timings_ns", name)) for name in timings
    }
    for section in ("native_artifact", "native_runtime"):
        value = samples[0]["metrics"].get(section)
        if not isinstance(value, dict):
            summary[section] = None
            continue
        summary[section] = {
            name: numeric_metric_summary(samples, (section, name))
            for name in sorted(value)
        }
    paths = {item["metrics"].get("execution_path") for item in samples}
    declines = {
        json.dumps(item["metrics"].get("native_decline"), sort_keys=True) for item in samples
    }
    summary["execution_path"] = next(iter(paths)) if len(paths) == 1 else sorted(paths)
    summary["native_decline"] = (
        json.loads(next(iter(declines))) if len(declines) == 1 else sorted(declines)
    )
    return summary


def select_workloads(names: str | None) -> list[Workload]:
    if names is None:
        return list(WORKLOADS)
    requested = [name for name in names.split(",") if name]
    available = {workload.name: workload for workload in WORKLOADS}
    unknown = [name for name in requested if name not in available]
    if unknown:
        raise ValueError(f"unknown workloads: {', '.join(unknown)}")
    return [available[name] for name in requested]


def input_identities(workloads: list[Workload]) -> dict[str, dict[str, Any]]:
    paths = {
        Path("lkjscript.package.json"),
        Path("lkjscript.lock.json"),
        *(Path(workload.path) for workload in workloads),
    }
    if any(workload.path.startswith("src/examples/polymorphic-transport/") for workload in workloads):
        paths.update(
            {
                Path("src/examples/polymorphic-transport/lkjscript.package.json"),
                Path("src/examples/polymorphic-transport/lkjscript.lock.json"),
            }
        )
    identities = {}
    for relative in sorted(paths):
        path = ROOT / relative
        identities[relative.as_posix()] = {
            "bytes": path.stat().st_size,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }
    return identities


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", required=True)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--binary", type=Path, default=Path("target/release/lkjscript"))
    parser.add_argument("--binary-commit")
    parser.add_argument("--binary-worktree")
    parser.add_argument("--binary-build-command")
    parser.add_argument("--binary-profile")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--workloads")
    parser.add_argument("--poll-ms", type=float, default=1.0)
    parser.add_argument("--no-build", action="store_true")
    arguments = parser.parse_args()
    if arguments.samples < 1:
        parser.error("--samples must be positive")
    if arguments.warmups < 0:
        parser.error("--warmups must not be negative")
    if arguments.poll_ms <= 0:
        parser.error("--poll-ms must be positive")
    if arguments.no_build and not all(
        (
            arguments.binary_commit,
            arguments.binary_worktree,
            arguments.binary_build_command,
            arguments.binary_profile,
        )
    ):
        parser.error(
            "--no-build requires --binary-commit, --binary-worktree, and "
            "--binary-build-command, and --binary-profile operator attestations"
        )
    if not arguments.no_build and arguments.binary_profile is not None:
        parser.error("--binary-profile is only an attestation for --no-build")
    try:
        workloads = select_workloads(arguments.workloads)
    except ValueError as error:
        parser.error(str(error))

    binary = arguments.binary
    if not binary.is_absolute():
        binary = ROOT / binary
    default_binary = ROOT / "target/release/lkjscript"
    if not arguments.no_build and binary != default_binary:
        parser.error("a harness build can only produce target/release/lkjscript")
    build_command = [
        "cargo",
        "build",
        "--locked",
        "--release",
        "-p",
        "lkjscript-app",
        "--bin",
        "lkjscript",
    ]
    checkout_commit = command_output("git", "rev-parse", "HEAD")
    checkout_worktree = worktree_metadata()
    build_environment, build_environment_record = controlled_build_environment()
    build_ns = None
    if not arguments.no_build:
        started = time.monotonic_ns()
        subprocess.run(build_command, cwd=ROOT, env=build_environment, check=True)
        build_ns = time.monotonic_ns() - started
        binary_provenance = {
            "kind": "built-by-harness",
            "commit": checkout_commit,
            "worktree": checkout_worktree,
            "profile": RELEASE_PROFILE,
            "build_command": " ".join(build_command),
        }
    else:
        binary_provenance = {
            "kind": "operator-attested-retained-binary",
            "commit": arguments.binary_commit,
            "worktree": arguments.binary_worktree,
            "profile": arguments.binary_profile,
            "build_command": arguments.binary_build_command,
        }
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error(f"binary is not executable: {binary}")

    output = arguments.output or (
        ROOT / "target" / "runtime-baseline" / f"{arguments.label}.json"
    )
    if not output.is_absolute():
        output = ROOT / output
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        parser.error(f"refusing to overwrite existing output: {output}")
    p95_rank = math.ceil(0.95 * arguments.samples)
    record: dict[str, Any] = {
        "label": arguments.label,
        "measurement_checkout": {
            "commit": checkout_commit,
            "worktree": checkout_worktree,
        },
        "binary_provenance": binary_provenance,
        "input_identities": input_identities(workloads),
        "machine": machine_metadata(),
        "cache_state": (
            f"{arguments.warmups} validated fresh-process warmup(s) per workload precede "
            "measurement; measured samples use fresh processes; ordinary filesystem and CPU "
            "cache state remains uncontrolled beyond those warmups; no persistent runtime state"
        ),
        "sample_protocol": (
            "sequential workloads, fresh process per sample, median and nearest-rank p95; "
            f"nearest-rank p95 selects ordered sample {p95_rank} of {arguments.samples}"
        ),
        "rss_method": (
            f"{arguments.poll_ms:g} ms /proc polling; sum current resident bytes for the "
            "lkjscript process tree; approximate, may miss short-lived processes or a final peak, "
            "may report zero for a very short run, and may double-count shared pages"
        ),
        "allocation_measurement": (
            "total allocator counts and bytes unavailable: no supported allocator profiler is "
            "installed; published native mapping bytes and selected saturating native service "
            "event counters are recorded and are not total allocation measurements"
        ),
        "incremental_build_ns": build_ns,
        "build_environment": build_environment_record,
        "effect_isolation": (
            "host-effect paths are arguments beneath one harness-owned mode-0700 temporary "
            "directory; every sample cleans its owned files and final cleanup removes the directory"
        ),
        "run_command": f"{binary} run <workload> [-- <private-effect-path>]",
        "binary": str(binary),
        "binary_bytes": binary.stat().st_size,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "warmups_per_workload": arguments.warmups,
        "samples_per_workload": arguments.samples,
        "workloads": [],
    }

    poll_seconds = arguments.poll_ms / 1_000
    with tempfile.TemporaryDirectory(
        prefix=".lkjscript-runtime-effects-", dir=output.parent
    ) as private_directory:
        effect_root = Path(private_directory)
        effect_root.chmod(0o700)
        for workload in workloads:
            for index in range(1, arguments.warmups + 1):
                metrics_path = effect_root / f"{workload.name}-warmup-{index}.metrics"
                run_sample(binary, workload, metrics_path, effect_root, poll_seconds)
                print(
                    f"{arguments.label}: {workload.name} "
                    f"warmup={index}/{arguments.warmups}",
                    flush=True,
                )
            samples = []
            for index in range(1, arguments.samples + 1):
                metrics_path = effect_root / f"{workload.name}-{index}.metrics"
                measured = run_sample(
                    binary, workload, metrics_path, effect_root, poll_seconds
                )
                measured["sample"] = index
                samples.append(measured)
                metrics = measured["metrics"]
                print(
                    f"{arguments.label}: {workload.name} sample={index}/{arguments.samples} "
                    f"wall_ms={measured['process_wall_ns'] / 1_000_000:.3f} "
                    f"rss_mib={measured['process_tree_peak_rss_bytes'] / (1024 * 1024):.1f} "
                    f"path={metrics.get('execution_path')} "
                    f"decline={metrics.get('native_decline')}",
                    flush=True,
                )
            record["workloads"].append(
                {
                    "name": workload.name,
                    "path": workload.path,
                    "families": workload.families,
                    "expected": {
                        "exit_status": workload.status,
                        "outcome_kind": workload.outcome_kind,
                        "outcome_value": workload.outcome_value,
                        "stdout_sha256": workload.stdout_sha256,
                        "stderr_sha256": workload.stderr_sha256,
                        "execution_path": workload.execution_path,
                        "native_entered": workload.native_entered,
                        "unique_cleanup": workload.require_unique_cleanup,
                        "private_effect": workload.effect_name,
                    },
                    "samples": samples,
                    "summary": summarize(samples),
                }
            )

    publish_json_exclusive(output, record)
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
