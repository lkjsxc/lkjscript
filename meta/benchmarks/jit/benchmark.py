#!/usr/bin/env python3
"""Retained forced optimizing-JIT benchmark protocol for Linux x86-64."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import random
import statistics
import struct
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

METRICS_PREFIX = b"LKJSCRIPT_METRICS "
SCHEMA = "lkjscript.optimizing-jit-benchmark.v1"
DEFAULT_SEED = 0x4C4B4A534F505449
CASE_NAMES = (
    "optimizing-workload-baseline",
    "optimizing-workload-optimizing",
    "scalar-workload-baseline",
)
EXACT_I64_3333 = {"kind": "returned", "value_kind": "i64", "exact": "3333"}
EXACT_I64_1 = {"kind": "returned", "value_kind": "i64", "exact": "1"}
SCALAR_ITERATIONS = 100_000
HISTORICAL_NATIVE_MEDIAN_NS = 7_647_935
HISTORICAL_PROCESS_MEDIAN_NS = 9_372_036
REGRESSION_CEILING = 1.05


def repository_root() -> Path:
    return Path(__file__).resolve().parents[3]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def artifact(path: Path, root: Path) -> dict[str, Any]:
    return {
        "path": str(path.relative_to(root)) if path.is_relative_to(root) else str(path),
        "size_bytes": path.stat().st_size,
        "sha256": sha256(path),
    }


def scalar_oracle(iterations: int) -> dict[str, str]:
    accumulator = 0.0
    for index in range(iterations):
        accumulator += 1.0 / (2.0 * float(index) + 1.0)
    bits = struct.unpack("!Q", struct.pack("!d", accumulator))[0]
    return {
        "kind": "returned",
        "value_kind": "f64-bits",
        "exact": f"0x{bits:016x}",
    }


def clean_environment(metrics: bool) -> dict[str, str]:
    environment = os.environ.copy()
    for name in (
        "LKJSCRIPT_JIT_DIAGNOSTICS",
        "LKJSCRIPT_JIT_DUMP_DIR",
        "LKJSCRIPT_METRICS",
        "LKJSCRIPT_METRICS_FILE",
    ):
        environment.pop(name, None)
    if metrics:
        environment["LKJSCRIPT_METRICS"] = "1"
    return environment


def engine_command(binary: Path, workload: Path, engine: str) -> list[str]:
    return [str(binary), "run", "--engine", engine, str(workload)]


def read_rss_kib(pid: int) -> int:
    try:
        text = Path(f"/proc/{pid}/status").read_text(encoding="utf-8")
    except (FileNotFoundError, ProcessLookupError, PermissionError):
        return 0
    for line in text.splitlines():
        if line.startswith("VmRSS:"):
            fields = line.split()
            try:
                return int(fields[1])
            except (IndexError, ValueError):
                return 0
    return 0


def execute_process(
    root: Path, command: list[str], *, metrics: bool, poll_rss: bool
) -> dict[str, Any]:
    started = time.monotonic_ns()
    process = subprocess.Popen(
        command,
        cwd=root,
        env=clean_environment(metrics),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    peak_rss_kib = 0
    while process.poll() is None:
        if poll_rss:
            peak_rss_kib = max(peak_rss_kib, read_rss_kib(process.pid))
        time.sleep(0.0005)
    if poll_rss:
        peak_rss_kib = max(peak_rss_kib, read_rss_kib(process.pid))
    stdout, stderr = process.communicate()
    wall_ns = time.monotonic_ns() - started
    return {
        "command": command,
        "exit_status": process.returncode,
        "stdout": stdout,
        "stderr": stderr,
        "process_wall_ns": wall_ns,
        "peak_rss_kib": peak_rss_kib,
    }


def require_success(process: dict[str, Any], label: str) -> None:
    if process["exit_status"] != 0:
        raise RuntimeError(
            f"{label} exited {process['exit_status']}: "
            f"stdout={process['stdout']!r} stderr={process['stderr']!r}"
        )


def run_silent(root: Path, command: list[str], label: str) -> dict[str, Any]:
    process = execute_process(root, command, metrics=False, poll_rss=False)
    require_success(process, label)
    if process["stdout"] or process["stderr"]:
        raise RuntimeError(
            f"{label} normal streams were not silent: "
            f"stdout={process['stdout']!r} stderr={process['stderr']!r}"
        )
    return {
        "label": label,
        "command": command,
        "exit_status": process["exit_status"],
        "stdout_bytes": 0,
        "stderr_bytes": 0,
    }


def parse_metrics(process: dict[str, Any], engine: str, expected: dict[str, str]) -> dict[str, Any]:
    require_success(process, engine)
    if process["stdout"]:
        raise RuntimeError(f"{engine} produced unexpected stdout: {process['stdout']!r}")
    lines = process["stderr"].splitlines()
    if len(lines) != 1 or not lines[0].startswith(METRICS_PREFIX):
        raise RuntimeError(f"{engine} produced unexpected metrics stderr: {process['stderr']!r}")
    try:
        metrics = json.loads(lines[0][len(METRICS_PREFIX) :])
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RuntimeError(f"{engine} emitted malformed metrics: {error}") from error
    if metrics.get("schema") != "lkjscript.metrics.v1":
        raise RuntimeError(f"{engine} emitted unknown metrics schema {metrics.get('schema')!r}")
    if metrics.get("engine") != engine:
        raise RuntimeError(f"{engine} metrics reported {metrics.get('engine')!r}")
    if metrics.get("outcome") != expected:
        raise RuntimeError(f"{engine} outcome {metrics.get('outcome')!r} != {expected!r}")
    return metrics


def wx_is_verified(jit: dict[str, Any]) -> bool:
    objects = jit.get("objects")
    return isinstance(objects, list) and bool(objects) and all(
        obj.get("wx_verified") is True for obj in objects
    )


def validate_forced_metrics(metrics: dict[str, Any], tier: str, *, proof_required: bool) -> None:
    jit = metrics.get("jit")
    if not isinstance(jit, dict):
        raise RuntimeError(f"forced {tier} sample omitted JIT metrics")
    if jit.get("compile_failures") != 0 or jit.get("vm_fallbacks") != 0:
        raise RuntimeError(f"forced {tier} sample reported failure/fallback")
    if not wx_is_verified(jit):
        raise RuntimeError(f"forced {tier} sample did not prove W^X on every object")
    objects = jit["objects"]
    functions = jit.get("functions")
    if not isinstance(functions, list) or not functions:
        raise RuntimeError(f"forced {tier} sample omitted function tier facts")
    if tier == "baseline":
        if jit.get("baseline_native_entries", 0) <= 0:
            raise RuntimeError("forced baseline sample had no baseline entry")
        if jit.get("optimizing_native_entries") != 0:
            raise RuntimeError("forced baseline sample entered optimizing code")
        if jit.get("baseline_code_objects", 0) <= 0 or jit.get("optimizing_code_objects") != 0:
            raise RuntimeError("forced baseline sample reported wrong object tier counts")
        if any(obj.get("tier") != "Baseline" for obj in objects):
            raise RuntimeError("forced baseline sample retained a non-baseline object")
        if any(function.get("state") != "BaselineNative" for function in functions):
            raise RuntimeError("forced baseline sample retained a non-baseline function state")
    else:
        if jit.get("optimizing_native_entries", 0) <= 0:
            raise RuntimeError("forced optimizing sample had no optimizing entry")
        if jit.get("baseline_native_entries") != 0:
            raise RuntimeError("forced optimizing sample entered baseline code")
        if jit.get("optimizing_code_objects", 0) <= 0 or jit.get("baseline_code_objects") != 0:
            raise RuntimeError("forced optimizing sample reported wrong object tier counts")
        if any(obj.get("tier") != "Optimizing" for obj in objects):
            raise RuntimeError("forced optimizing sample retained a non-optimizing object")
        if any(function.get("state") != "OptimizedNative" for function in functions):
            raise RuntimeError("forced optimizing sample retained a non-optimized function state")
        if proof_required and (
            jit.get("optimization_certificate_records", 0) <= 0
            or jit.get("checked_i64_rewrites", 0) <= 0
            or jit.get("optimizing_passes", 0) <= 0
        ):
            raise RuntimeError("forced optimizing sample omitted executed proof evidence")


def exact_jit_facts(metrics: dict[str, Any]) -> dict[str, Any] | None:
    jit = metrics.get("jit")
    if jit is None:
        return None
    top_names = (
        "compile_failures",
        "vm_fallbacks",
        "native_entries",
        "baseline_native_entries",
        "optimizing_native_entries",
        "baseline_code_objects",
        "optimizing_code_objects",
        "optimizing_passes",
        "optimization_discovery_passes",
        "optimization_checker_passes",
        "optimization_reconstruction_passes",
        "optimization_cleanup_passes",
        "optimization_validation_passes",
        "optimization_certificate_records",
        "optimization_certificate_bytes_estimate",
        "algebraic_rewrites",
        "gvn_rewrites",
        "checked_i64_rewrites",
        "direct_native_calls",
        "poll_v1_calls",
        "native_invocations",
        "code_cache_peak_objects",
        "code_cache_peak_bytes",
        "metadata_cache_peak_bytes",
        "accounted_allocation_peak_bytes",
        "allocations",
        "allocation_bytes_estimate",
        "collections",
        "peak_live_heap_bytes_estimate",
        "maximum_roots",
        "runtime_heap_attempts",
        "runtime_heap_successes",
        "barrier_count",
        "peak_native_frame_depth",
        "vm_to_native_transitions",
        "native_to_vm_transitions",
    )
    object_names = (
        "identity",
        "tier",
        "functions",
        "code_bytes",
        "metadata_bytes",
        "optimization_metadata_bytes_estimate",
        "accounted_allocation_bytes",
        "relocations",
        "safepoints",
        "work_units",
        "optimization_work_units",
        "input_instructions",
        "output_instructions",
        "instruction_growth",
        "cleanup_removed_instructions",
        "iterations",
        "optimizing_passes",
        "discovery_passes",
        "checker_passes",
        "reconstruction_passes",
        "cleanup_passes",
        "validation_passes",
        "certificate_records",
        "certificate_bytes_estimate",
        "algebraic_rewrites",
        "gvn_rewrites",
        "checked_i64_rewrites",
        "native_entries",
        "wx_verified",
    )
    function_names = (
        "id",
        "name",
        "state",
        "calls",
        "attempts",
        "failure",
        "code_object",
        "epoch",
        "native_entries",
    )
    return {
        "top": {name: jit[name] for name in top_names},
        "functions": [
            {name: function[name] for name in function_names}
            for function in jit["functions"]
        ],
        "objects": [
            {name: obj[name] for name in object_names} for obj in jit["objects"]
        ],
    }


def run_metric_sample(
    root: Path,
    binary: Path,
    workload: Path,
    engine: str,
    expected: dict[str, str],
    *,
    tier: str | None,
    proof_required: bool = False,
) -> dict[str, Any]:
    command = engine_command(binary, workload, engine)
    process = execute_process(root, command, metrics=True, poll_rss=True)
    metrics = parse_metrics(process, engine, expected)
    if tier is not None:
        validate_forced_metrics(metrics, tier, proof_required=proof_required)
    return {
        "command": command,
        "process_wall_ns": process["process_wall_ns"],
        "peak_rss_kib": process["peak_rss_kib"],
        "exit_status": process["exit_status"],
        "stdout_bytes": 0,
        "stderr_metric_lines": 1,
        "exact_jit_facts": exact_jit_facts(metrics),
        "metrics": metrics,
    }


def validate_allocation_sample(sample: dict[str, Any]) -> None:
    jit = sample["metrics"]["jit"]
    if jit["allocations"] < 7 or jit["maximum_roots"] <= 0:
        raise RuntimeError("allocation graph did not report expected allocations and roots")
    if jit["runtime_heap_attempts"] < 14:
        raise RuntimeError("allocation graph did not reach expected heap operation count")
    if jit["runtime_heap_attempts"] != jit["runtime_heap_successes"]:
        raise RuntimeError("allocation graph reported a failed heap operation")


def nearest_rank_p95(values: list[int]) -> int:
    return sorted(values)[math.ceil(0.95 * len(values)) - 1]


def distribution(values: list[int]) -> dict[str, int | float]:
    median = statistics.median(values)
    return {
        "median": median,
        "mad": statistics.median(abs(value - median) for value in values),
        "p95": nearest_rank_p95(values),
        "min": min(values),
        "max": max(values),
    }


def numeric_series(samples: list[dict[str, Any]]) -> dict[str, list[int]]:
    series = {
        "process_wall_ns": [sample["process_wall_ns"] for sample in samples],
        "peak_rss_kib": [sample["peak_rss_kib"] for sample in samples],
    }
    for name in samples[0]["metrics"]["timings_ns"]:
        series[f"timings_ns.{name}"] = [
            0 if sample["metrics"]["timings_ns"][name] is None
            else sample["metrics"]["timings_ns"][name]
            for sample in samples
        ]
    jit_names = (
        "compile_failures",
        "vm_fallbacks",
        "native_entries",
        "baseline_native_entries",
        "optimizing_native_entries",
        "baseline_code_objects",
        "optimizing_code_objects",
        "optimizing_passes",
        "optimization_discovery_passes",
        "optimization_checker_passes",
        "optimization_reconstruction_passes",
        "optimization_cleanup_passes",
        "optimization_validation_passes",
        "optimization_certificate_records",
        "optimization_certificate_bytes_estimate",
        "algebraic_rewrites",
        "gvn_rewrites",
        "checked_i64_rewrites",
        "code_cache_peak_objects",
        "code_cache_peak_bytes",
        "metadata_cache_peak_bytes",
        "accounted_allocation_peak_bytes",
    )
    for name in jit_names:
        series[f"jit.{name}"] = [sample["metrics"]["jit"][name] for sample in samples]
    for name in ("code_bytes", "metadata_bytes", "optimization_metadata_bytes_estimate"):
        series[f"jit.objects_total.{name}"] = [
            sum(obj[name] for obj in sample["metrics"]["jit"]["objects"])
            for sample in samples
        ]
    return series


def summarize(samples: list[dict[str, Any]]) -> dict[str, dict[str, int | float]]:
    return {name: distribution(values) for name, values in numeric_series(samples).items()}


def median(summary: dict[str, dict[str, int | float]], name: str) -> float:
    return float(summary[name]["median"])


def command_version(command: list[str]) -> str:
    completed = subprocess.run(
        command, check=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
    )
    return completed.stdout.strip()


def git_output(root: Path, arguments: list[str]) -> str:
    completed = subprocess.run(
        ["git", *arguments], cwd=root, check=True, stdout=subprocess.PIPE, text=True
    )
    return completed.stdout.strip()


def cpu_model() -> str:
    try:
        for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return "not measured"


def memory_kib() -> int | None:
    try:
        for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemTotal:"):
                return int(line.split()[1])
    except (OSError, ValueError):
        pass
    return None


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


def parse_args() -> argparse.Namespace:
    root = repository_root()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=root / "target/release/lkjscript")
    parser.add_argument("--warmups", type=int, default=4)
    parser.add_argument("--samples", type=int, default=31)
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=DEFAULT_SEED)
    parser.add_argument(
        "--output",
        type=Path,
        default=root / "meta/benchmarks/jit/results/optimizing-jit-linux-x86_64.json",
    )
    arguments = parser.parse_args()
    if arguments.warmups < 4:
        parser.error("retained runs require at least four warmups per case")
    if arguments.samples < 31:
        parser.error("retained runs require at least 31 measured samples per case")
    return arguments


def main() -> int:
    arguments = parse_args()
    root = repository_root()
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        raise RuntimeError("the retained protocol requires Linux x86-64")

    repository_before = git_output(root, ["status", "--porcelain"])
    build = locked_release_build(root)
    binary = arguments.binary.resolve()
    if not binary.is_file():
        raise RuntimeError(f"missing release binary {binary}")

    optimizing = root / "src/examples/jit-optimizing/main.lkjscript"
    scalar = root / "src/examples/jit-scalar/main.lkjscript"
    allocation = root / "crates/lkjscript-app/tests/fixtures/allocation-graph.lkjscript"
    historical = root / "meta/benchmarks/jit/results/callable-baseline-jit-linux-x86_64.json"
    paths = (optimizing, scalar, allocation, historical)
    if any(not path.is_file() for path in paths):
        raise RuntimeError("a required workload or retained baseline is missing")

    scalar_expected = scalar_oracle(SCALAR_ITERATIONS)
    cases = {
        "optimizing-workload-baseline": {
            "workload": optimizing,
            "engine": "baseline-jit",
            "expected": EXACT_I64_3333,
            "tier": "baseline",
            "proof_required": False,
        },
        "optimizing-workload-optimizing": {
            "workload": optimizing,
            "engine": "optimizing-jit",
            "expected": EXACT_I64_3333,
            "tier": "optimizing",
            "proof_required": True,
        },
        "scalar-workload-baseline": {
            "workload": scalar,
            "engine": "baseline-jit",
            "expected": scalar_expected,
            "tier": "baseline",
            "proof_required": False,
        },
    }

    silence_checks = [
        run_silent(
            root,
            engine_command(binary, optimizing, "vm"),
            "optimizing workload reference VM",
        )
    ]
    for name in CASE_NAMES:
        case = cases[name]
        silence_checks.append(
            run_silent(
                root,
                engine_command(binary, case["workload"], case["engine"]),
                name,
            )
        )

    vm_oracle = run_metric_sample(
        root,
        binary,
        optimizing,
        "vm",
        EXACT_I64_3333,
        tier=None,
    )
    if vm_oracle["metrics"]["jit"] is not None:
        raise RuntimeError("reference VM oracle unexpectedly reported JIT state")

    allocation_check = run_metric_sample(
        root,
        binary,
        allocation,
        "optimizing-jit",
        EXACT_I64_1,
        tier="optimizing",
    )
    validate_allocation_sample(allocation_check)

    randomizer = random.Random(arguments.seed)
    warmup_order = [name for name in CASE_NAMES for _ in range(arguments.warmups)]
    randomizer.shuffle(warmup_order)
    warmups: list[dict[str, Any]] = []
    exact_signatures: dict[str, dict[str, Any]] = {}
    for ordinal, name in enumerate(warmup_order):
        case = cases[name]
        sample = run_metric_sample(
            root,
            binary,
            case["workload"],
            case["engine"],
            case["expected"],
            tier=case["tier"],
            proof_required=case["proof_required"],
        )
        sample["ordinal"] = ordinal
        sample["case"] = name
        signature = sample["exact_jit_facts"]
        if name in exact_signatures and signature != exact_signatures[name]:
            raise RuntimeError(f"{name} warmup changed exact JIT facts")
        exact_signatures.setdefault(name, signature)
        warmups.append(sample)

    measured_order = [name for name in CASE_NAMES for _ in range(arguments.samples)]
    randomizer.shuffle(measured_order)
    measured: list[dict[str, Any]] = []
    for ordinal, name in enumerate(measured_order):
        case = cases[name]
        sample = run_metric_sample(
            root,
            binary,
            case["workload"],
            case["engine"],
            case["expected"],
            tier=case["tier"],
            proof_required=case["proof_required"],
        )
        sample["ordinal"] = ordinal
        sample["case"] = name
        if sample["exact_jit_facts"] != exact_signatures[name]:
            raise RuntimeError(f"{name} measured sample changed exact JIT facts")
        measured.append(sample)

    by_case = {
        name: [sample for sample in measured if sample["case"] == name]
        for name in CASE_NAMES
    }
    summary = {name: summarize(by_case[name]) for name in CASE_NAMES}
    baseline_summary = summary["optimizing-workload-baseline"]
    optimizing_summary = summary["optimizing-workload-optimizing"]
    scalar_summary = summary["scalar-workload-baseline"]
    baseline_native = median(baseline_summary, "timings_ns.native_execution")
    optimizing_native = median(optimizing_summary, "timings_ns.native_execution")
    baseline_mad = float(baseline_summary["timings_ns.native_execution"]["mad"])
    optimizing_mad = float(optimizing_summary["timings_ns.native_execution"]["mad"])
    improvement = baseline_native - optimizing_native
    combined_mad = baseline_mad + optimizing_mad

    retained = json.loads(historical.read_text(encoding="utf-8"))
    retained_native = float(
        retained["summary"]["baseline-jit"]["timings_ns.native_execution"]["median"]
    )
    retained_process = float(
        retained["summary"]["baseline-jit"]["process_wall_ns"]["median"]
    )
    if retained_native != HISTORICAL_NATIVE_MEDIAN_NS or retained_process != HISTORICAL_PROCESS_MEDIAN_NS:
        raise RuntimeError("retained callable baseline medians changed unexpectedly")
    current_scalar_native = median(scalar_summary, "timings_ns.native_execution")
    current_scalar_process = median(scalar_summary, "process_wall_ns")

    criteria = {
        "optimizing_native_speedup_at_least_1_20x": baseline_native / optimizing_native >= 1.20,
        "native_improvement_greater_than_twice_combined_mad": improvement > 2.0 * combined_mad,
        "all_exact_outcomes_and_stream_checks": True,
        "optimizing_nonzero_entries_zero_baseline_entries_and_fallback": True,
        "forced_baseline_nonzero_entries_zero_optimizing_entries_and_fallback": True,
        "all_native_objects_wx_verified": True,
        "optimizing_checked_proof_nonzero": True,
        "scalar_native_median_no_more_than_5_percent_over_retained": current_scalar_native
        <= retained_native * REGRESSION_CEILING,
        "scalar_process_median_no_more_than_5_percent_over_retained": current_scalar_process
        <= retained_process * REGRESSION_CEILING,
        "allocation_graph_exact_and_accounted_once": True,
    }
    comparisons = {
        "optimizing_native_speedup_over_same_commit_baseline": baseline_native
        / optimizing_native,
        "native_median_improvement_ns": improvement,
        "combined_native_mad_ns": combined_mad,
        "twice_combined_native_mad_ns": 2.0 * combined_mad,
        "scalar_native_current_over_retained": current_scalar_native / retained_native,
        "scalar_process_current_over_retained": current_scalar_process / retained_process,
        "retained_scalar_native_median_ns": retained_native,
        "retained_scalar_process_median_ns": retained_process,
        "current_scalar_native_median_ns": current_scalar_native,
        "current_scalar_process_median_ns": current_scalar_process,
        "historical_comparison_caveat": (
            "sentinel only: source is retained, but compiler, metrics, native ABI, stack "
            "checks, and surrounding generated code evolved after the callable-baseline commit"
        ),
    }
    adopted = all(criteria.values())

    sources = [
        root / "src/examples/jit-optimizing/main.lkjscript",
        root / "src/examples/jit-optimizing/kernel.lkjscript",
        root / "src/examples/jit-scalar/main.lkjscript",
        root / "src/examples/jit-scalar/kernel.lkjscript",
        allocation,
        root / "meta/benchmarks/jit/benchmark.py",
        root / "Cargo.lock",
    ]
    result = {
        "schema": SCHEMA,
        "verdict": {
            "status": "Adopted" if adopted else "Rejected",
            "scope": "forced first optimizing-tier performance gate only",
            "automatic_promotion": "disabled and unmeasured",
            "criteria": criteria,
        },
        "repository": {
            "commit": git_output(root, ["rev-parse", "HEAD"]),
            "tree": git_output(root, ["rev-parse", "HEAD^{tree}"]),
            "dirty_before_benchmark": bool(repository_before),
            "dirty_paths_before_benchmark": repository_before.splitlines(),
        },
        "environment": {
            "os": platform.system(),
            "platform": platform.platform(),
            "kernel": platform.release(),
            "machine": platform.machine(),
            "cpu": cpu_model(),
            "logical_cpus": os.cpu_count(),
            "memory_kib": memory_kib(),
            "python": sys.version.splitlines()[0],
            "rustc": command_version(["rustc", "--version", "--verbose"]),
            "cargo": command_version(["cargo", "--version", "--verbose"]),
            "git": command_version(["git", "--version"]),
        },
        "build": build,
        "protocol": {
            "seed": arguments.seed,
            "seed_hex": hex(arguments.seed),
            "warmups_per_case": arguments.warmups,
            "measured_samples_per_case": arguments.samples,
            "cases": list(CASE_NAMES),
            "interleaving": "one deterministic randomized order across all three cases",
            "samples_removed": 0,
            "metrics_transport": "one LKJSCRIPT_METRICS JSON stderr line per measured execution",
            "normal_stream_policy": "stdout and stderr both empty when metrics are disabled",
            "wall_clock": "Python time.monotonic_ns around process creation through collection",
            "rss_source": "/proc/<pid>/status VmRSS polled approximately every 0.5 ms; maximum observed",
            "p95": "nearest-rank",
            "mad": "median absolute deviation",
            "combined_mad": "sum of same-commit baseline and optimizing native MAD",
            "allocation_graph_runs": 1,
        },
        "artifacts": {
            "binary": artifact(binary, root),
            "sources": [artifact(path, root) for path in sources],
            "retained_callable_baseline": artifact(historical, root),
        },
        "oracles": {
            "optimizing_workload": {
                "mechanism": "separate reference VM engine process",
                "expected_outcome": EXACT_I64_3333,
                "sample": vm_oracle,
            },
            "scalar_workload": {
                "mechanism": "independent Python IEEE-F64 recurrence",
                "iterations": SCALAR_ITERATIONS,
                "expected_outcome": scalar_expected,
            },
            "allocation_graph": {
                "mechanism": "exact known graph result plus runtime accounting invariants",
                "expected_outcome": EXACT_I64_1,
                "sample": allocation_check,
            },
        },
        "normal_stream_checks": silence_checks,
        "exact_case_jit_facts": exact_signatures,
        "warmup_order": warmup_order,
        "warmups": warmups,
        "measured_order": measured_order,
        "samples": measured,
        "summary": summary,
        "comparisons": comparisons,
    }
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print(f"wrote {output}")
    for name in CASE_NAMES:
        native = summary[name]["timings_ns.native_execution"]
        wall = summary[name]["process_wall_ns"]
        print(
            f"{name}: native median={native['median']} ns MAD={native['mad']} ns; "
            f"wall median={wall['median']} ns MAD={wall['mad']} ns"
        )
    print(
        f"speedup={comparisons['optimizing_native_speedup_over_same_commit_baseline']:.6f}x "
        f"verdict={result['verdict']['status']}"
    )
    for name, passed in criteria.items():
        print(f"criterion {name}={'pass' if passed else 'fail'}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError, KeyError, ValueError) as error:
        print(f"benchmark: {error}", file=sys.stderr)
        raise SystemExit(1)
