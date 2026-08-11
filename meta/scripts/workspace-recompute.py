#!/usr/bin/env python3
"""Measure source-free semantic-workspace recomputation in fresh release test processes."""

from __future__ import annotations

import argparse
from collections import defaultdict
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import time
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parents[2]
MARKER = "LKJSCRIPT_WORKSPACE_RECOMPUTE "
TEST = "workspace::recompute_measurement::workspace_recompute_scale_sample"
ENV_WORKLOAD = "LKJSCRIPT_WORKSPACE_WORKLOAD"
ENV_FUNCTIONS = "LKJSCRIPT_WORKSPACE_FUNCTIONS"
RSS_INTERVAL_SECONDS = 0.01


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
    return {
        "hostname": platform.node(),
        "os": platform.platform(),
        "kernel": platform.release(),
        "architecture": platform.machine(),
        "cpu": cpu,
        "logical_cpus": os.cpu_count(),
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


def build_release_test_binary() -> tuple[Path, list[str], int]:
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
    executables: list[Path] = []
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
            executables.append(Path(executable))
    unique = sorted(set(executables))
    if len(unique) != 1:
        raise RuntimeError(
            "release build did not identify exactly one compiler lib test binary: "
            + repr([str(path) for path in unique])
        )
    return unique[0], command, wall_ns


def output_lines(value: str) -> int:
    return len(value.splitlines())


def run_sample(binary: Path, workload: str, helper_functions: int) -> dict[str, Any]:
    command = [
        str(binary),
        TEST,
        "--exact",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]
    environment = os.environ.copy()
    environment[ENV_WORKLOAD] = workload
    if workload == "W0":
        environment.pop(ENV_FUNCTIONS, None)
    else:
        environment[ENV_FUNCTIONS] = str(helper_functions)
    started = time.monotonic_ns()
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    peak_rss: int | None = None
    while process.poll() is None:
        observed = process_tree_rss_bytes(process.pid)
        if observed is not None:
            peak_rss = observed if peak_rss is None else max(peak_rss, observed)
        time.sleep(RSS_INTERVAL_SECONDS)
    stdout, stderr = process.communicate()
    elapsed_ns = time.monotonic_ns() - started
    if process.returncode != 0:
        raise RuntimeError(
            f"workspace sample {workload}/{helper_functions} failed with exit "
            f"{process.returncode}\n{stdout}\n{stderr}"
        )
    markers = [
        line[len(MARKER) :]
        for stream in (stdout, stderr)
        for line in stream.splitlines()
        if line.startswith(MARKER)
    ]
    if len(markers) != 1:
        raise RuntimeError(
            f"workspace sample {workload}/{helper_functions} emitted {len(markers)} "
            f"{MARKER.strip()} markers\n{stdout}\n{stderr}"
        )
    try:
        measured = json.loads(markers[0])
    except json.JSONDecodeError as error:
        raise RuntimeError(
            f"workspace sample {workload}/{helper_functions} emitted malformed JSON: {error}"
        ) from error
    if measured.get("schema") != "lkjscript.workspace-recompute-sample.v1":
        raise RuntimeError("workspace sample emitted an unknown schema")
    if measured.get("workload") != workload:
        raise RuntimeError("workspace sample workload does not match its requested cell")
    measured.update(
        {
            "process_tree_peak_rss_bytes": peak_rss,
            "process_wall_ns": elapsed_ns,
            "stdout_bytes": len(stdout.encode("utf-8")),
            "stdout_lines": output_lines(stdout),
            "stderr_bytes": len(stderr.encode("utf-8")),
            "stderr_lines": output_lines(stderr),
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
    return {
        "median": int(statistics.median(values)),
        "p95_nearest_rank_orientation": nearest_rank_p95(values),
        "minimum": min(values),
        "maximum": max(values),
    }


def summarize(results: list[dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[tuple[str, int], list[dict[str, Any]]] = defaultdict(list)
    for result in results:
        grouped[(result["workload"], result["geometry"]["helper_functions"])].append(
            result
        )
    metric_paths = [
        "transaction.wall_ns",
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
        for path in metric_paths:
            values = [
                value
                for sample in samples
                if (value := nested_number(sample, path)) is not None
            ]
            measured = distribution(values)
            if measured is not None:
                timings[path] = measured
        work = [
            transaction.get("work") if isinstance(transaction, dict) else None
            for sample in samples
            for transaction in [sample.get("transaction")]
        ]
        deterministic_work = work[0] if work and all(item == work[0] for item in work) else None
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
        "tail_method": "nearest-rank p95; with five samples this is the maximum and is orientation only",
        "cells": cells,
    }


def parse_csv(value: str) -> list[str]:
    return [item.strip() for item in value.split(",") if item.strip()]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", required=True)
    parser.add_argument("--workloads", default="W0,W1,W2")
    parser.add_argument("--sizes", default="16,128,512")
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--decision", default="pending")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    workloads = parse_csv(args.workloads)
    allowed = {"W0", "W1", "W2"}
    if not workloads or len(set(workloads)) != len(workloads) or any(
        workload not in allowed for workload in workloads
    ):
        parser.error("workloads must be a unique comma-separated subset of W0,W1,W2")
    try:
        sizes = [int(value) for value in parse_csv(args.sizes)]
    except ValueError:
        parser.error("sizes must be comma-separated positive integers")
    if args.samples < 1 or not sizes or any(size < 1 for size in sizes):
        parser.error("sizes and samples must select positive measurement geometry")

    worktree_before = worktree_metadata()
    binary, build_command, build_wall_ns = build_release_test_binary()
    output = args.output or ROOT / "target" / "workspace-recompute" / f"{args.label}.json"
    output.parent.mkdir(parents=True, exist_ok=True)

    results: list[dict[str, Any]] = []
    for workload in workloads:
        workload_sizes = [0] if workload == "W0" else sizes
        for helper_functions in workload_sizes:
            for sample_number in range(1, args.samples + 1):
                print(
                    f"{args.label}: workload={workload} helpers={helper_functions} "
                    f"sample={sample_number}/{args.samples}",
                    flush=True,
                )
                measured = run_sample(binary, workload, helper_functions)
                measured["sample"] = sample_number
                results.append(measured)

    worktree = worktree_metadata()
    if worktree != worktree_before:
        raise RuntimeError("worktree changed while workspace samples were running")
    sample_command = (
        f"{binary} {TEST} --exact --ignored --nocapture --test-threads=1"
    )
    document = {
        "schema": "lkjscript.workspace-recompute-results.v1",
        "label": args.label,
        "commit": command_output("git", "rev-parse", "HEAD"),
        "worktree": worktree,
        "worktree_stable_during_run": True,
        "machine": machine_metadata(),
        "build": {
            "command": " ".join(build_command),
            "profile": "release test profile; workspace release LTO/codegen/strip settings apply",
            "locked": True,
            "cache_state": "warm Cargo dependencies and release artifacts; build completed once before samples",
            "wall_ns": build_wall_ns,
            "test_binary": str(binary),
        },
        "sample_command": sample_command,
        "sample_environment": {
            ENV_WORKLOAD: "W0|W1|W2",
            ENV_FUNCTIONS: "positive helper-function count for W1/W2",
            "test_threads": 1,
        },
        "workloads": {
            "W0": "tiny complete source-free scalar control, 1,000 direct identity queries, compile, VM result 7",
            "W1": "metadata-only main-hole goal refinement in an incomplete workspace of independent complete scalar helpers",
            "W2": "one counted-loop limit literal replacement, compact queries, selected body projection, immediate compile, VM result 101, retained old result 100",
        },
        "sizes": sizes,
        "samples_per_cell": args.samples,
        "rss": {
            "method": "10 ms /proc polling; sum resident pages for the direct release test process and descendants",
            "interval_ms": 10,
            "limitation": "approximate process-tree RSS, not unique physical memory; may miss short-lived peaks, may double-count shared pages, and process wall includes up to one polling interval of exit-detection delay",
        },
        "selection": {
            "inclusion": "fresh process exited zero, emitted exactly one decodable marker, and passed all in-sample semantic assertions",
            "exclusion": "no failed or malformed sample is retained",
            "tail": "nearest-rank p95 orientation; five-sample p95 is the maximum",
        },
        "samples": results,
        "summary": summarize(results),
        "decision": args.decision,
        "limitations": [
            "single-host timing is orientation, not a product guarantee or CI gate",
            "total allocator counts and allocated bytes are unavailable",
            "exact retained snapshot bytes are unavailable",
            "fixture construction is reported separately from transaction/query/projection/compile timers",
            "stdout/stderr include the opt-in test protocol and libtest harness, not normal quiet product check output",
            "process wall includes up to one 10 ms RSS-poll interval after the test process exits",
        ],
    }
    output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(output)


if __name__ == "__main__":
    main()
