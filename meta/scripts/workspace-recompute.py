#!/usr/bin/env python3
"""Measure source-free semantic-workspace recomputation in fresh release test processes."""

from __future__ import annotations

import argparse
from collections import defaultdict
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shlex
import subprocess
import sys
import time
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parents[2]
MARKER = "LKJSCRIPT_WORKSPACE_RECOMPUTE "
TEST = "workspace::recompute_measurement::workspace_recompute_scale_sample"
ENV_WORKLOAD = "LKJSCRIPT_WORKSPACE_WORKLOAD"
ENV_FUNCTIONS = "LKJSCRIPT_WORKSPACE_FUNCTIONS"
ENV_REFINEMENT_MODE = "LKJSCRIPT_WORKSPACE_REFINEMENT_MODE"
RSS_INTERVAL_SECONDS = 0.01
PERFORMANCE_ENVIRONMENT_KEYS = {
    "CARGO_BUILD_JOBS",
    "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_TARGET",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_INCREMENTAL",
    "CARGO_TARGET_DIR",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "RUSTDOCFLAGS",
    "RUSTFLAGS",
}
SAMPLE_ENVIRONMENT_KEYS = {
    "DYLD_LIBRARY_PATH",
    "LANG",
    "LC_ALL",
    "LD_LIBRARY_PATH",
    "PATH",
    "RUST_BACKTRACE",
    "TEMP",
    "TMP",
    "TMPDIR",
}
EXPECTED_OLD_RESULTS = {
    "W0": 7,
    "W2": 100,
    "W3": 7,
    "W4": 42,
    "W5": 10,
    "W6": 5,
    "W7": 7,
}
EXPECTED_NEW_RESULTS = {
    "W0": 8,
    "W2": 101,
    "W3": 9,
    "W4": 43,
    "W5": 11,
    "W6": 5,
    "W7": 9,
}
EXPECTED_TRUE_CORRECTNESS = {
    "W0": ("root_identity_preserved",),
    "W1": (
        "hole_identity_preserved",
        "hole_owner_preserved",
        "old_snapshot_goal_preserved",
        "projection_deterministic",
    ),
    "W2": (
        "target_identity_preserved",
        "unaffected_entity_identity_preserved",
        "unaffected_node_identity_preserved",
        "projection_deterministic",
    ),
    "W3": ("return_identity_preserved", "return_type_preserved"),
    "W4": (
        "match_exhaustive",
        "selected_arm_identity_preserved",
        "payload_binding_identity_preserved",
        "payload_member_identity_preserved",
    ),
    "W5": (
        "call_identity_preserved",
        "argument_identity_preserved",
        "substitutions_unchanged",
        "witnesses_unchanged",
    ),
    "W6": (
        "survivor_identity_preserved",
        "relocated_survivor_identity_preserved",
        "relocated_survivor_node_identity_preserved",
        "deleted_identity_tombstoned",
        "old_snapshot_preserved",
        "failed_edit_atomic",
        "private_binding_relocated",
        "private_compaction_observed",
    ),
    "W7": (
        "root_identity_preserved",
        "typed_hole_expected_i64",
        "compile_stopped_before_lowering",
    ),
}


def command_output(*command: str) -> str:
    return subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ).stdout.strip()


def generated_path(path: str) -> bool:
    parts = Path(path).parts
    return (
        path.startswith((".pi-subagents/", "target/"))
        or "__pycache__" in parts
        or path.endswith(".pyc")
    )


def worktree_metadata() -> dict[str, Any]:
    status = [
        line
        for line in command_output(
            "git", "status", "--short", "--untracked-files=all"
        ).splitlines()
        if not generated_path(line[3:])
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
    digest.update(b"lkjscript.workspace-recompute-worktree\0")
    digest.update(tracked_diff)
    untracked_hashes: dict[str, str] = {}
    for relative in untracked:
        content = (ROOT / relative).read_bytes()
        content_hash = hashlib.sha256(content).hexdigest()
        untracked_hashes[relative] = content_hash
        digest.update(relative.encode("utf-8"))
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
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.exists():
        for line in cpuinfo.read_text(encoding="utf-8").splitlines():
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    memory_bytes = None
    meminfo = Path("/proc/meminfo")
    if meminfo.exists():
        for line in meminfo.read_text(encoding="utf-8").splitlines():
            if line.startswith("MemTotal:"):
                memory_bytes = int(line.split()[1]) * 1024
                break
    governor_path = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    governor = (
        governor_path.read_text(encoding="utf-8").strip()
        if governor_path.is_file()
        else None
    )
    affinity = (
        len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None
    )
    load_average = list(os.getloadavg()) if hasattr(os, "getloadavg") else None
    return {
        "hostname": platform.node(),
        "os": platform.platform(),
        "kernel": platform.release(),
        "architecture": platform.machine(),
        "cpu": cpu,
        "logical_cpus": os.cpu_count(),
        "affinity_logical_cpus": affinity,
        "cpu_governor": governor,
        "load_average_at_capture": load_average,
        "memory_bytes": memory_bytes,
        "rustc": command_output("rustc", "--version"),
        "cargo": command_output("cargo", "--version"),
        "python": platform.python_version(),
    }


def process_tree_rss_bytes(root_pid: int) -> int | None:
    proc = Path("/proc")
    if not proc.is_dir():
        return None
    processes: dict[int, tuple[int, int]] = {}
    for entry in proc.iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat = (entry / "stat").read_text(encoding="utf-8")
            close = stat.rfind(")")
            fields = stat[close + 2 :].split()
            processes[int(entry.name)] = (int(fields[1]), int(fields[21]))
        except (FileNotFoundError, PermissionError, ValueError, IndexError):
            continue
    descendants = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, (ppid, _) in processes.items():
            if ppid in descendants and pid not in descendants:
                descendants.add(pid)
                changed = True
    pages = sum(processes.get(pid, (0, 0))[1] for pid in descendants)
    return pages * os.sysconf("SC_PAGE_SIZE")


def build_release_test_binary() -> tuple[Path, list[str], int, bool]:
    command = [
        "cargo",
        "test",
        "--locked",
        "--release",
        "-p",
        "lkjscript-compiler",
        "--lib",
        "--no-run",
        "--message-format=json",
    ]
    started = time.monotonic_ns()
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    wall_ns = time.monotonic_ns() - started
    if completed.returncode != 0:
        raise RuntimeError(
            "release measurement build failed with exit "
            f"{completed.returncode}\n{completed.stdout}\n{completed.stderr}"
        )
    executables: list[tuple[Path, bool]] = []
    for line in completed.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        target = message.get("target", {})
        profile = message.get("profile", {})
        executable = message.get("executable")
        if (
            message.get("reason") == "compiler-artifact"
            and executable
            and profile.get("test") is True
            and target.get("name") == "lkjscript_compiler"
            and "lib" in target.get("kind", [])
        ):
            executables.append((Path(executable), message.get("fresh") is True))
    unique_paths = sorted({path for path, _ in executables})
    if len(unique_paths) != 1:
        raise RuntimeError(
            "release build did not identify exactly one compiler lib test binary: "
            + repr([str(path) for path in unique_paths])
        )
    binary = unique_paths[0]
    fresh = all(value for path, value in executables if path == binary)
    return binary, command, wall_ns, fresh


def output_lines(value: bytes) -> int:
    return len(value.splitlines())


def bounded_output(value: bytes) -> str:
    limit = 16 * 1024
    suffix = value[-limit:]
    prefix = "<truncated>\n" if len(value) > limit else ""
    return prefix + suffix.decode("utf-8", errors="replace")


def sample_environment() -> dict[str, str]:
    return {
        key: os.environ[key]
        for key in sorted(SAMPLE_ENVIRONMENT_KEYS)
        if key in os.environ
    }


def validate_sample(
    measured: dict[str, Any], workload: str, helper_functions: int, refinement_mode: str
) -> None:
    if measured.get("schema") != "lkjscript.workspace-recompute-sample.v2":
        raise RuntimeError("workspace sample emitted an unknown schema")
    if measured.get("workload") != workload:
        raise RuntimeError("workspace sample workload does not match its requested cell")
    geometry = measured.get("geometry")
    correctness = measured.get("correctness")
    agent_loop = measured.get("agent_loop")
    if not all(isinstance(value, dict) for value in (geometry, correctness, agent_loop)):
        raise RuntimeError("workspace sample omitted structured geometry or correctness facts")
    expected_helpers = helper_functions if workload in {"W1", "W2", "W5"} else 0
    if nested_number(measured, "geometry.helper_functions") != expected_helpers:
        raise RuntimeError("workspace sample geometry does not match the requested cell")
    if workload == "W1" and measured.get("refinement_mode") != refinement_mode:
        raise RuntimeError("workspace sample refinement mode does not match the request")
    required_paths = [
        "transaction.wall_ns",
        "queries.wall_ns",
        "projection.wall_ns",
        "compile.wall_ns",
        "agent_loop.edit_inspect_check_wall_ns",
        "agent_loop.authoring_loop_wall_ns",
        "agent_loop.selected_api_operations",
        "correctness.source_load_invocations",
        "correctness.parser_invocations",
    ]
    if workload != "W1":
        required_paths.extend(["vm.wall_ns", "vm.result_i64"])
    if any(nested_number(measured, path) is None for path in required_paths):
        raise RuntimeError("workspace sample omitted a required nonnegative integer metric")
    if any(nested_number(measured, path) < 0 for path in required_paths):
        raise RuntimeError("workspace sample emitted a negative metric")
    expected_geometry = {
        "W0": (1, 1),
        "W1": (helper_functions + 1, helper_functions + 1),
        "W2": (helper_functions + 2, helper_functions + 12),
        "W3": (2, 8),
        "W4": (10, 10),
        "W5": (helper_functions + 17, helper_functions + 26),
        "W6": (3, 3),
        "W7": (1, 1),
    }[workload]
    observed_geometry = (
        nested_number(measured, "geometry.total_entities"),
        nested_number(measured, "geometry.total_semantic_nodes"),
    )
    if observed_geometry != expected_geometry:
        raise RuntimeError("workspace sample entity/node geometry is inconsistent")
    if (
        nested_number(measured, "correctness.source_load_invocations") != 0
        or nested_number(measured, "correctness.parser_invocations") != 0
    ):
        raise RuntimeError("workspace sample unexpectedly loaded or parsed source")
    for fact in EXPECTED_TRUE_CORRECTNESS[workload]:
        if correctness.get(fact) is not True:
            raise RuntimeError(
                f"workspace sample correctness fact {fact} is not exactly true"
            )
    if workload == "W1":
        if measured.get("vm") is not None or measured.get("compile", {}).get("status") != "incomplete":
            raise RuntimeError("W1 must remain incomplete and have no VM result")
        if nested_number(measured, "compile.lowering_invocations") != 0:
            raise RuntimeError("W1 incomplete compile entered lowering")
        expected_shared = refinement_mode == "narrow"
        if (
            correctness.get("program_arc_shared") is not expected_shared
            or correctness.get("index_arc_shared") is not expected_shared
            or measured.get("transaction", {})
            .get("work", {})
            .get("metadata_only_path_used")
            is not expected_shared
        ):
            raise RuntimeError("W1 refinement route facts do not match the selected mode")
    else:
        if measured.get("compile", {}).get("status") != "complete":
            raise RuntimeError("complete workload did not report complete compilation")
        if (
            nested_number(measured, "correctness.old_snapshot_result_i64")
            != EXPECTED_OLD_RESULTS[workload]
            or nested_number(measured, "correctness.new_snapshot_result_i64")
            != EXPECTED_NEW_RESULTS[workload]
            or nested_number(measured, "vm.result_i64")
            != EXPECTED_NEW_RESULTS[workload]
        ):
            raise RuntimeError("workspace sample VM outcomes are inconsistent")
    projection = measured.get("projection")
    digest = projection.get("sha256") if isinstance(projection, dict) else None
    if (
        not isinstance(projection, dict)
        or nested_number(measured, "projection.bytes") is None
        or nested_number(measured, "projection.lines") is None
        or not isinstance(digest, list)
        or len(digest) != 32
        or any(
            not isinstance(value, int)
            or isinstance(value, bool)
            or not 0 <= value <= 255
            for value in digest
        )
    ):
        raise RuntimeError("workspace sample projection facts are malformed")
    if (
        nested_number(measured, "agent_loop.edit_inspect_check_wall_ns")
        > nested_number(measured, "agent_loop.authoring_loop_wall_ns")
    ):
        raise RuntimeError("workspace sample authoring-loop totals are inconsistent")


def run_sample(
    binary: Path,
    workload: str,
    helper_functions: int,
    refinement_mode: str,
    stdout_path: Path,
    stderr_path: Path,
    timeout_seconds: float,
) -> dict[str, Any]:
    command = [
        str(binary),
        TEST,
        "--exact",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]
    environment = sample_environment()
    environment[ENV_WORKLOAD] = workload
    environment[ENV_REFINEMENT_MODE] = refinement_mode
    if workload in {"W1", "W2", "W5"}:
        environment[ENV_FUNCTIONS] = str(helper_functions)
    else:
        environment.pop(ENV_FUNCTIONS, None)
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    started_ns = time.monotonic_ns()
    deadline = time.monotonic() + timeout_seconds
    timed_out = False
    with stdout_path.open("wb") as stdout_file, stderr_path.open("wb") as stderr_file:
        process = subprocess.Popen(
            command,
            cwd=ROOT,
            env=environment,
            stdout=stdout_file,
            stderr=stderr_file,
        )
        peak_rss: int | None = None
        while process.poll() is None:
            if time.monotonic() >= deadline:
                timed_out = True
                process.kill()
                process.wait()
                break
            observed = process_tree_rss_bytes(process.pid)
            if observed is not None:
                peak_rss = observed if peak_rss is None else max(peak_rss, observed)
            time.sleep(RSS_INTERVAL_SECONDS)
    elapsed_ns = time.monotonic_ns() - started_ns
    stdout_bytes = stdout_path.read_bytes()
    stderr_bytes = stderr_path.read_bytes()
    if timed_out:
        raise RuntimeError(
            f"workspace sample {workload}/{helper_functions} exceeded "
            f"{timeout_seconds} seconds\n{bounded_output(stderr_bytes)}"
        )
    if process.returncode != 0:
        raise RuntimeError(
            f"workspace sample {workload}/{helper_functions} failed with exit "
            f"{process.returncode}\nstdout:\n{bounded_output(stdout_bytes)}"
            f"\nstderr:\n{bounded_output(stderr_bytes)}"
        )
    marker = MARKER.encode("utf-8")
    markers = [
        line[len(marker) :]
        for stream in (stdout_bytes, stderr_bytes)
        for line in stream.splitlines()
        if line.startswith(marker)
    ]
    if len(markers) != 1:
        raise RuntimeError(
            f"workspace sample {workload}/{helper_functions} emitted {len(markers)} "
            f"{MARKER.strip()} markers\nstdout:\n{bounded_output(stdout_bytes)}"
            f"\nstderr:\n{bounded_output(stderr_bytes)}"
        )
    try:
        measured = json.loads(markers[0].decode("utf-8", errors="strict"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RuntimeError(
            f"workspace sample {workload}/{helper_functions} emitted malformed JSON: {error}"
        ) from error
    if not isinstance(measured, dict):
        raise RuntimeError("workspace sample JSON root is not an object")
    validate_sample(measured, workload, helper_functions, refinement_mode)
    measured.update(
        {
            "process_tree_peak_rss_bytes": peak_rss,
            "process_wall_ns": elapsed_ns,
            "stdout_bytes": len(stdout_bytes),
            "stdout_lines": output_lines(stdout_bytes),
            "stdout_sha256": hashlib.sha256(stdout_bytes).hexdigest(),
            "stdout_path": str(stdout_path),
            "stderr_bytes": len(stderr_bytes),
            "stderr_lines": output_lines(stderr_bytes),
            "stderr_sha256": hashlib.sha256(stderr_bytes).hexdigest(),
            "stderr_path": str(stderr_path),
        }
    )
    return measured


def nested_number(value: dict[str, Any], path: str) -> int | None:
    current: Any = value
    for component in path.split("."):
        if not isinstance(current, dict) or component not in current:
            return None
        current = current[component]
    return current if isinstance(current, int) and not isinstance(current, bool) else None


def nearest_rank_p95(values: Iterable[int]) -> int:
    ordered = sorted(values)
    rank = max(1, math.ceil(0.95 * len(ordered)))
    return ordered[rank - 1]


def distribution(values: list[int]) -> dict[str, int] | None:
    if not values:
        return None
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2 == 1:
        median_numerator = ordered[middle]
        median_denominator = 1
    else:
        median_numerator = ordered[middle - 1] + ordered[middle]
        median_denominator = 2
    return {
        "observations": len(ordered),
        "median": median_numerator // median_denominator,
        "median_numerator": median_numerator,
        "median_denominator": median_denominator,
        "p95_nearest_rank_orientation": nearest_rank_p95(ordered),
        "minimum": ordered[0],
        "maximum": ordered[-1],
    }


def required_summary_metrics(workload: str) -> set[str]:
    required = {
        "agent_loop.edit_inspect_check_wall_ns",
        "agent_loop.authoring_loop_wall_ns",
        "transaction.wall_ns",
        "transaction.stage_wall_ns",
        "queries.wall_ns",
        "projection.wall_ns",
        "compile.wall_ns",
        "process_wall_ns",
        "stdout_bytes",
        "stderr_bytes",
    }
    if workload != "W1":
        required.update(
            {
                "compile.complete_hir_derivation_ns",
                "compile.memory_planning_ns",
                "compile.ssa_construction_ns",
                "compile.ssa_verification_ns",
                "compile.normalization_ns",
                "compile.bytecode_lowering_ns",
                "compile.bytecode_validation_ns",
                "vm.wall_ns",
            }
        )
    if workload == "W6":
        required.update(
            {
                "sequence.create_function.wall_ns",
                "sequence.complete_function_body.wall_ns",
                "sequence.rename.wall_ns",
                "sequence.invalid_stale_edit.wall_ns",
            }
        )
    if workload == "W7":
        required.update(
            {
                "sequence.introduce_hole.wall_ns",
                "sequence.incomplete_compile.wall_ns",
            }
        )
    return required


def summarize(results: list[dict[str, Any]], samples_per_cell: int) -> dict[str, Any]:
    grouped: dict[tuple[str, int], list[dict[str, Any]]] = defaultdict(list)
    for result in results:
        grouped[(result["workload"], result["geometry"]["helper_functions"])].append(
            result
        )
    metric_paths = [
        "agent_loop.edit_inspect_check_wall_ns",
        "agent_loop.authoring_loop_wall_ns",
        "transaction.wall_ns",
        "sequence.create_function.wall_ns",
        "sequence.complete_function_body.wall_ns",
        "sequence.rename.wall_ns",
        "sequence.invalid_stale_edit.wall_ns",
        "sequence.introduce_hole.wall_ns",
        "sequence.incomplete_compile.wall_ns",
        "transaction.stage_wall_ns",
        "queries.wall_ns",
        "projection.wall_ns",
        "compile.wall_ns",
        "compile.complete_hir_derivation_ns",
        "compile.memory_planning_ns",
        "compile.ssa_construction_ns",
        "compile.ssa_verification_ns",
        "compile.normalization_ns",
        "compile.bytecode_lowering_ns",
        "compile.bytecode_validation_ns",
        "vm.wall_ns",
        "process_wall_ns",
        "process_tree_peak_rss_bytes",
        "stdout_bytes",
        "stderr_bytes",
    ]
    cells = []
    for (workload, helpers), samples in sorted(grouped.items()):
        timings: dict[str, Any] = {}
        required_metrics = required_summary_metrics(workload)
        for path in metric_paths:
            values = [
                value
                for sample in samples
                if (value := nested_number(sample, path)) is not None
            ]
            if path in required_metrics and len(values) != len(samples):
                raise RuntimeError(
                    f"required metric {path} is missing within {workload}/{helpers}"
                )
            if values and len(values) != len(samples):
                raise RuntimeError(
                    f"optional metric {path} is only partially observed within "
                    f"{workload}/{helpers}"
                )
            measured = distribution(values)
            if measured is not None:
                timings[path] = measured
        work = [
            transaction.get("work") if isinstance(transaction, dict) else None
            for sample in samples
            for transaction in [sample.get("transaction")]
        ]
        if not work or not all(item == work[0] for item in work):
            raise RuntimeError(
                f"deterministic transaction work changed within {workload}/{helpers}"
            )
        deterministic_work = work[0]
        geometry = samples[0]["geometry"]
        if any(sample["geometry"] != geometry for sample in samples):
            raise RuntimeError(f"geometry changed within {workload}/{helpers} samples")
        cells.append(
            {
                "workload": workload,
                "helper_functions": helpers,
                "samples": len(samples),
                "geometry": geometry,
                "deterministic_transaction_work": deterministic_work,
                "distributions": timings,
            }
        )
    return {
        "median_method": "exact integer numerator/denominator; median is the floor only for convenience when the exact value is half-integral",
        "tail_method": (
            "nearest-rank p95 (ceil(0.95*n)); it is the maximum when fewer "
            "than 20 samples are collected and remains orientation only"
        ),
        "samples_per_cell": samples_per_cell,
        "cells": cells,
    }


def parse_csv(value: str) -> list[str]:
    return [item.strip() for item in value.split(",") if item.strip()]


def source_state() -> dict[str, Any]:
    return {
        "commit": command_output("git", "rev-parse", "HEAD"),
        "branch": command_output("git", "branch", "--show-current"),
        "worktree": worktree_metadata(),
    }


def performance_environment() -> dict[str, str]:
    return {
        key: os.environ[key]
        for key in sorted(os.environ)
        if key in PERFORMANCE_ENVIRONMENT_KEYS or key.startswith("CARGO_PROFILE_")
    }


def relativize_stream_paths(sample: dict[str, Any], base: Path) -> None:
    for key in ("stdout_path", "stderr_path"):
        sample[key] = str(Path(sample[key]).relative_to(base))


def raw_stream_manifest(sample: dict[str, Any]) -> dict[str, Any]:
    return {
        "workload": sample["workload"],
        "helper_functions": sample["geometry"]["helper_functions"],
        "stdout": {
            "path": sample["stdout_path"],
            "bytes": sample["stdout_bytes"],
            "lines": sample["stdout_lines"],
            "sha256": sample["stdout_sha256"],
        },
        "stderr": {
            "path": sample["stderr_path"],
            "bytes": sample["stderr_bytes"],
            "lines": sample["stderr_lines"],
            "sha256": sample["stderr_sha256"],
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", required=True)
    parser.add_argument("--workloads", default="W0,W1,W2,W3,W4,W5,W6,W7")
    parser.add_argument("--sizes", default="16,128,512")
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument(
        "--refinement-mode", choices=("narrow", "full"), default="narrow"
    )
    parser.add_argument("--sample-timeout-seconds", type=float, default=300.0)
    parser.add_argument("--progress", action="store_true")
    parser.add_argument("--decision", default="pending")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    workloads = parse_csv(args.workloads)
    allowed = {f"W{index}" for index in range(8)}
    if not workloads or len(set(workloads)) != len(workloads) or any(
        workload not in allowed for workload in workloads
    ):
        parser.error("workloads must be a unique comma-separated subset of W0 through W7")
    try:
        sizes = [int(value) for value in parse_csv(args.sizes)]
    except ValueError:
        parser.error("sizes must be comma-separated positive integers")
    if (
        args.samples < 1
        or args.warmups < 0
        or args.sample_timeout_seconds <= 0
        or not sizes
        or any(size < 1 for size in sizes)
    ):
        parser.error(
            "sizes/samples/timeout must be positive and warmups must be nonnegative"
        )
    if len(set(sizes)) != len(sizes):
        parser.error("sizes must not contain duplicates")
    if args.refinement_mode == "full" and any(workload != "W1" for workload in workloads):
        parser.error("full refinement mode is a W1 comparison control only")

    started_at = datetime.now(timezone.utc).isoformat()
    invocation_cwd = Path.cwd().resolve()
    requested_output = args.output or Path("target/workspace-recompute") / f"{args.label}.json"
    output = (
        requested_output.resolve()
        if requested_output.is_absolute()
        else (invocation_cwd / requested_output).resolve()
    )
    raw_directory = output.parent / f"{output.stem}-raw"
    if output.exists() or raw_directory.exists():
        raise RuntimeError(
            f"result or raw path already exists; select a fresh label/output: {output}"
        )
    output.parent.mkdir(parents=True, exist_ok=True)
    raw_directory.mkdir(parents=True)

    source_before = source_state()
    machine = machine_metadata()
    binary, build_command, build_wall_ns, build_fresh = build_release_test_binary()
    binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    results: list[dict[str, Any]] = []
    warmup_streams: list[dict[str, Any]] = []
    fixed_workloads = {"W0", "W3", "W4", "W6", "W7"}
    for workload in workloads:
        workload_sizes = [0] if workload in fixed_workloads else sizes
        for helper_functions in workload_sizes:
            cell = f"{workload}-n{helper_functions}"
            for warmup_number in range(1, args.warmups + 1):
                if args.progress:
                    print(
                        f"{args.label}: workload={workload} helpers={helper_functions} "
                        f"discarded-warmup={warmup_number}/{args.warmups}",
                        file=sys.stderr,
                        flush=True,
                    )
                prefix = raw_directory / f"{cell}-warmup-{warmup_number:02}"
                warmup = run_sample(
                    binary,
                    workload,
                    helper_functions,
                    args.refinement_mode,
                    prefix.with_suffix(".stdout"),
                    prefix.with_suffix(".stderr"),
                    args.sample_timeout_seconds,
                )
                relativize_stream_paths(warmup, output.parent)
                manifest = raw_stream_manifest(warmup)
                manifest["warmup"] = warmup_number
                warmup_streams.append(manifest)
            for sample_number in range(1, args.samples + 1):
                if args.progress:
                    print(
                        f"{args.label}: workload={workload} helpers={helper_functions} "
                        f"sample={sample_number}/{args.samples}",
                        file=sys.stderr,
                        flush=True,
                    )
                prefix = raw_directory / f"{cell}-sample-{sample_number:02}"
                measured = run_sample(
                    binary,
                    workload,
                    helper_functions,
                    args.refinement_mode,
                    prefix.with_suffix(".stdout"),
                    prefix.with_suffix(".stderr"),
                    args.sample_timeout_seconds,
                )
                relativize_stream_paths(measured, output.parent)
                measured["sample"] = sample_number
                results.append(measured)

    source_after = source_state()
    if source_after != source_before:
        raise RuntimeError("source commit, branch, or worktree changed during sampling")
    if hashlib.sha256(binary.read_bytes()).hexdigest() != binary_sha256:
        raise RuntimeError("release measurement binary changed during sampling")
    summary = summarize(results, args.samples)
    scaled_workloads = {"W1", "W2", "W5"}
    expected_cells = sum(
        len(sizes) if workload in scaled_workloads else 1 for workload in workloads
    )
    if len(summary["cells"]) != expected_cells or any(
        cell["samples"] != args.samples for cell in summary["cells"]
    ):
        raise RuntimeError("summary does not contain the requested cells and samples")
    if any(
        nested_number(sample, "correctness.source_load_invocations") != 0
        or nested_number(sample, "correctness.parser_invocations") != 0
        or nested_number(sample, "agent_loop.authoring_loop_wall_ns") is None
        for sample in results
    ):
        raise RuntimeError("sample correctness or authoring-loop summary facts are invalid")

    sample_command = (
        f"{binary} {TEST} --exact --ignored --nocapture --test-threads=1"
    )
    rust_host = next(
        (
            line.split(":", 1)[1].strip()
            for line in command_output("rustc", "-vV").splitlines()
            if line.startswith("host:")
        ),
        "unknown",
    )
    selected_target = os.environ.get("CARGO_BUILD_TARGET", rust_host)
    driver_invocation = shlex.join([sys.executable, *sys.argv])
    document = {
        "schema": "lkjscript.workspace-recompute-results.v2",
        "started_at_utc": started_at,
        "finished_at_utc": datetime.now(timezone.utc).isoformat(),
        "driver_invocation": driver_invocation,
        "invocation_cwd": str(invocation_cwd),
        "driver_output_contract": {
            "success_stdout": "absolute result path plus one newline",
            "success_stderr": "empty unless --progress is selected",
        },
        "branch": source_before["branch"],
        "rust_host_triple": rust_host,
        "target_triple": selected_target,
        "performance_environment": performance_environment(),
        "label": args.label,
        "commit": source_before["commit"],
        "worktree": source_before["worktree"],
        "source_stable_during_run": True,
        "worktree_stable_during_run": True,
        "machine": machine,
        "build": {
            "command": " ".join(build_command),
            "profile": "release test profile; workspace release LTO/codegen/strip settings apply",
            "locked": True,
            "cache_state": (
                "compiler test artifact was already fresh"
                if build_fresh
                else "compiler test artifact was rebuilt once before samples"
            ),
            "compiler_test_artifact_fresh": build_fresh,
            "wall_ns": build_wall_ns,
            "test_binary": str(binary),
            "test_binary_sha256": binary_sha256,
        },
        "sample_command": sample_command,
        "sample_environment": {
            ENV_WORKLOAD: "W0|W1|W2|W3|W4|W5|W6|W7",
            ENV_FUNCTIONS: "positive helper-function count for W1/W2/W5",
            ENV_REFINEMENT_MODE: args.refinement_mode,
            "inherited_allowlisted_environment": sample_environment(),
            "sample_timeout_seconds": args.sample_timeout_seconds,
            "test_threads": 1,
        },
        "workloads": {
            "W0": "tiny source-free scalar replacement with queries, projection, compile, and VM 7 to 8",
            "W1": "metadata-only main-hole goal refinement in an incomplete workspace of independent complete scalar helpers",
            "W2": "counted-loop limit replacement with retained old VM 100 and new VM 101",
            "W3": "owned byte-vector shared borrow plus early return; return-subtree edit 7 to 9 with cleanup obligations",
            "W4": "product construction plus exhaustive payload enum match; selected-arm edit 42 to 43",
            "W5": "exact Copy-bounded generic identity call value edit 10 to 11 in scaled mixed nominal/control geometry",
            "W6": "function lifecycle, rename, atomic stale failure, middle-function deletion/compaction, tombstone and relocated-survivor checks",
            "W7": "complete to typed hole to blocked compile-before-lowering to refilled complete VM 7 to 9",
        },
        "sizes": sizes,
        "samples_per_cell": args.samples,
        "discarded_warmups_per_cell": args.warmups,
        "warmup_policy": "each cell runs explicit fresh-process warmups first; warmup metrics are discarded but raw streams are retained",
        "raw_directory": str(raw_directory.relative_to(output.parent)),
        "warmup_raw_streams": warmup_streams,
        "rss": {
            "method": "10 ms /proc polling; sum resident pages for the direct release test process and descendants",
            "interval_ms": 10,
            "limitation": "approximate process-tree RSS, not unique physical memory; may miss short-lived peaks, may double-count shared pages, and process wall includes up to one polling interval of exit-detection delay",
        },
        "selection": {
            "inclusion": "fresh process exited zero, emitted exactly one decodable marker, and passed all in-sample semantic assertions",
            "exclusion": "no failed or malformed sample is retained",
            "tail": "nearest-rank ceil(0.95*n); it is the maximum for fewer than 20 samples",
        },
        "samples": results,
        "summary": summary,
        "decision": args.decision,
        "decision_protocol": {
            "retain_correction_threshold": "at least 10% representative-medium end-to-end improvement or 20% in a clearly dominant phase",
            "semantic_requirement": "exact outcomes, identities, diffs, diagnostics, blockers, queries, cleanup, and old snapshots must remain equivalent",
            "full_recomputation_reversal": "reconsider when representative semantic edit/query/projection work dominates the local loop, retained memory becomes material, or scaling departs from recorded whole-program traversal shape",
        },
        "limitations": [
            "single-host timing is orientation, not a product guarantee or CI gate",
            "total allocator counts and allocated bytes are unavailable",
            "exact retained snapshot bytes are unavailable",
            "fixture construction is reported separately from transaction/query/projection/compile timers",
            "stdout/stderr include the opt-in test protocol and libtest harness, not normal quiet product check output",
            "process wall includes up to one 10 ms RSS-poll interval after the test process exits",
        ],
    }
    encoded = json.dumps(document, indent=2, sort_keys=True) + "\n"
    temporary_output = output.with_suffix(output.suffix + ".tmp")
    if temporary_output.exists():
        raise RuntimeError(f"temporary result path already exists: {temporary_output}")
    temporary_output.write_text(encoded, encoding="utf-8")
    os.replace(temporary_output, output)
    print(output)


if __name__ == "__main__":
    main()
