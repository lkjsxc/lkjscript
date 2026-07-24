#!/usr/bin/env python3
"""Decision-grade same-binary VM/baseline-JIT/auto benchmark harness."""

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
DEFAULT_SEED = 0x4C4B4A534D455452
VARIANTS = ("vm", "baseline-jit", "auto")
EXPECTED_ITERATIONS = 100_000


def repository_root() -> Path:
    return Path(__file__).resolve().parents[3]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def exact_oracle(iterations: int) -> dict[str, str]:
    accumulator = 0.0
    for index in range(iterations):
        accumulator += 1.0 / (2.0 * float(index) + 1.0)
    bits = struct.unpack("!Q", struct.pack("!d", accumulator))[0]
    return {
        "kind": "returned",
        "value_kind": "f64-bits",
        "exact": f"0x{bits:016x}",
    }


def read_peak_rss_kib(pid: int) -> int:
    try:
        status = Path(f"/proc/{pid}/status").read_text(encoding="utf-8")
    except (FileNotFoundError, ProcessLookupError, PermissionError):
        return 0
    for line in status.splitlines():
        if line.startswith("VmRSS:"):
            fields = line.split()
            if len(fields) >= 2:
                try:
                    return int(fields[1])
                except ValueError:
                    return 0
    return 0


def variant_command(
    binary: Path, workload: Path, variant: str, auto_threshold: int
) -> list[str]:
    command = [str(binary), "run", "--engine", variant]
    if variant == "auto":
        command.extend(["--auto-jit-threshold", str(auto_threshold)])
    command.append(str(workload))
    return command


def run_sample(
    root: Path,
    binary: Path,
    workload: Path,
    variant: str,
    auto_threshold: int,
    expected: dict[str, str],
) -> dict[str, Any]:
    environment = os.environ.copy()
    environment.pop("LKJSCRIPT_JIT_DIAGNOSTICS", None)
    environment.pop("LKJSCRIPT_JIT_DUMP_DIR", None)
    environment.pop("LKJSCRIPT_METRICS_FILE", None)
    environment["LKJSCRIPT_METRICS"] = "1"
    command = variant_command(binary, workload, variant, auto_threshold)
    started = time.monotonic_ns()
    process = subprocess.Popen(
        command,
        cwd=root,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    peak_rss_kib = 0
    while process.poll() is None:
        peak_rss_kib = max(peak_rss_kib, read_peak_rss_kib(process.pid))
        time.sleep(0.0005)
    peak_rss_kib = max(peak_rss_kib, read_peak_rss_kib(process.pid))
    stdout, stderr = process.communicate()
    wall_ns = time.monotonic_ns() - started

    if process.returncode != 0:
        raise RuntimeError(
            f"{variant} exited {process.returncode}: stdout={stdout!r} stderr={stderr!r}"
        )
    if stdout:
        raise RuntimeError(f"{variant} produced unexpected stdout: {stdout!r}")
    lines = stderr.splitlines()
    if len(lines) != 1 or not lines[0].startswith(METRICS_PREFIX):
        raise RuntimeError(f"{variant} produced unexpected stderr: {stderr!r}")
    try:
        metrics = json.loads(lines[0][len(METRICS_PREFIX) :])
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RuntimeError(f"{variant} emitted malformed metrics: {error}") from error
    if metrics.get("schema") != "lkjscript.metrics.v1":
        raise RuntimeError(f"{variant} emitted an unknown metrics schema")
    if metrics.get("engine") != variant:
        raise RuntimeError(f"{variant} metrics reported engine {metrics.get('engine')!r}")
    if metrics.get("outcome") != expected:
        raise RuntimeError(
            f"{variant} outcome mismatch: {metrics.get('outcome')!r} != {expected!r}"
        )
    jit = metrics.get("jit")
    if variant == "vm":
        if jit is not None:
            raise RuntimeError("VM sample unexpectedly reported JIT state")
    elif not isinstance(jit, dict):
        raise RuntimeError(f"{variant} omitted JIT state")
    if variant == "baseline-jit":
        if jit["native_entries"] <= 0 or jit["vm_fallbacks"] != 0:
            raise RuntimeError("forced baseline sample did not prove fallback-free native entry")
        if jit["compile_failures"] != 0:
            raise RuntimeError("forced baseline sample reported a compile failure")
    if variant == "auto":
        if jit["compile_failures"] != 0 or jit["native_entries"] <= 0:
            raise RuntimeError("auto sample did not complete successful later-call tiering")
    return {
        "variant": variant,
        "command": command,
        "wall_ns": wall_ns,
        "peak_rss_kib": peak_rss_kib,
        "exit_status": process.returncode,
        "stdout_bytes": len(stdout),
        "stderr_metric_lines": 1,
        "metrics": metrics,
    }


def nearest_rank_p95(values: list[int]) -> int:
    ordered = sorted(values)
    return ordered[math.ceil(0.95 * len(ordered)) - 1]


def distribution(values: list[int]) -> dict[str, int | float]:
    median = statistics.median(values)
    deviations = [abs(value - median) for value in values]
    return {
        "median": median,
        "mad": statistics.median(deviations),
        "p95": nearest_rank_p95(values),
        "min": min(values),
        "max": max(values),
    }


def numeric_series(samples: list[dict[str, Any]]) -> dict[str, list[int]]:
    series: dict[str, list[int]] = {
        "process_wall_ns": [sample["wall_ns"] for sample in samples],
        "peak_rss_kib": [sample["peak_rss_kib"] for sample in samples],
    }
    timing_names = samples[0]["metrics"]["timings_ns"].keys()
    for name in timing_names:
        values = [sample["metrics"]["timings_ns"][name] for sample in samples]
        series[f"timings_ns.{name}"] = [0 if value is None else value for value in values]
    if samples[0]["metrics"]["jit"] is not None:
        for name in (
            "compile_failures",
            "vm_fallbacks",
            "native_entries",
            "direct_native_calls",
            "poll_v1_calls",
            "native_invocations",
            "code_cache_peak_objects",
            "code_cache_peak_bytes",
            "metadata_cache_peak_bytes",
            "accounted_allocation_peak_bytes",
        ):
            series[f"jit.{name}"] = [sample["metrics"]["jit"][name] for sample in samples]
    return series


def summaries(samples: list[dict[str, Any]]) -> dict[str, dict[str, int | float]]:
    return {name: distribution(values) for name, values in numeric_series(samples).items()}


def median_metric(
    summary: dict[str, dict[str, dict[str, int | float]]], variant: str, metric: str
) -> float:
    return float(summary[variant][metric]["median"])


def comparisons(summary: dict[str, dict[str, dict[str, int | float]]]) -> dict[str, Any]:
    vm_execution = median_metric(summary, "vm", "timings_ns.vm_execution")
    native_execution = median_metric(
        summary, "baseline-jit", "timings_ns.native_execution"
    )
    native_compile = median_metric(
        summary, "baseline-jit", "timings_ns.native_lowering_encoding"
    ) + median_metric(
        summary, "baseline-jit", "timings_ns.relocation_wx_installation"
    )
    saved_per_invocation = vm_execution - native_execution
    break_even = (
        math.ceil(native_compile / saved_per_invocation)
        if saved_per_invocation > 0
        else None
    )
    vm_wall = median_metric(summary, "vm", "process_wall_ns")
    forced_wall = median_metric(summary, "baseline-jit", "process_wall_ns")
    auto_wall = median_metric(summary, "auto", "process_wall_ns")
    return {
        "native_execution_speedup_over_vm": vm_execution / native_execution,
        "forced_end_to_end_speedup_over_vm": vm_wall / forced_wall,
        "auto_end_to_end_speedup_over_vm": vm_wall / auto_wall,
        "native_compile_install_median_ns": native_compile,
        "break_even_repeated_invocations": break_even,
    }


def command_version(command: list[str]) -> str:
    completed = subprocess.run(command, check=True, stdout=subprocess.PIPE, text=True)
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


def parse_args() -> argparse.Namespace:
    root = repository_root()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=root / "target/release/lkjscript")
    parser.add_argument(
        "--workload", type=Path, default=root / "src/examples/jit-scalar/main.lkjscript"
    )
    parser.add_argument("--warmups", type=int, default=4)
    parser.add_argument("--samples", type=int, default=31)
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=DEFAULT_SEED)
    parser.add_argument("--auto-threshold", type=int, default=64)
    parser.add_argument(
        "--output",
        type=Path,
        default=root
        / "meta/benchmarks/jit/results/callable-baseline-jit-linux-x86_64.json",
    )
    arguments = parser.parse_args()
    if arguments.warmups < 4:
        parser.error("decision-grade runs require at least four warmups per variant")
    if arguments.samples < 31:
        parser.error("decision-grade runs require at least 31 measured samples per variant")
    if arguments.auto_threshold <= 0:
        parser.error("--auto-threshold must be positive")
    return arguments


def main() -> int:
    arguments = parse_args()
    root = repository_root()
    binary = arguments.binary.resolve()
    workload = arguments.workload.resolve()
    output = arguments.output.resolve()
    if not binary.is_file():
        raise RuntimeError(f"missing release binary: {binary}")
    if not workload.is_file():
        raise RuntimeError(f"missing workload: {workload}")

    expected = exact_oracle(EXPECTED_ITERATIONS)
    randomizer = random.Random(arguments.seed)
    verification = [
        run_sample(
            root,
            binary,
            workload,
            variant,
            arguments.auto_threshold,
            expected,
        )
        for variant in VARIANTS
    ]

    warmup_order = [variant for variant in VARIANTS for _ in range(arguments.warmups)]
    randomizer.shuffle(warmup_order)
    warmups = []
    for ordinal, variant in enumerate(warmup_order):
        sample = run_sample(
            root,
            binary,
            workload,
            variant,
            arguments.auto_threshold,
            expected,
        )
        sample["ordinal"] = ordinal
        warmups.append(sample)

    measured_order = [variant for variant in VARIANTS for _ in range(arguments.samples)]
    randomizer.shuffle(measured_order)
    measured = []
    for ordinal, variant in enumerate(measured_order):
        sample = run_sample(
            root,
            binary,
            workload,
            variant,
            arguments.auto_threshold,
            expected,
        )
        sample["ordinal"] = ordinal
        measured.append(sample)

    by_variant = {
        variant: [sample for sample in measured if sample["variant"] == variant]
        for variant in VARIANTS
    }
    summary = {variant: summaries(samples) for variant, samples in by_variant.items()}
    dirty = git_output(root, ["status", "--porcelain"])
    result = {
        "schema": "lkjscript.jit-benchmark.v1",
        "repository": {
            "commit": git_output(root, ["rev-parse", "HEAD"]),
            "dirty": bool(dirty),
            "dirty_paths": dirty.splitlines(),
        },
        "environment": {
            "platform": platform.platform(),
            "kernel": platform.release(),
            "machine": platform.machine(),
            "cpu": cpu_model(),
            "logical_cpus": os.cpu_count(),
            "memory_kib": memory_kib(),
            "python": sys.version.splitlines()[0],
            "rustc": command_version(["rustc", "--version"]),
            "cargo": command_version(["cargo", "--version"]),
        },
        "protocol": {
            "seed": arguments.seed,
            "seed_hex": hex(arguments.seed),
            "warmups_per_variant": arguments.warmups,
            "measured_samples_per_variant": arguments.samples,
            "variants": list(VARIANTS),
            "auto_threshold": arguments.auto_threshold,
            "rss_source": "/proc/<pid>/status VmRSS polled at approximately 0.5 ms",
            "wall_clock": "Python time.monotonic_ns around process creation through wait",
            "p95": "nearest-rank",
            "samples_removed": 0,
        },
        "artifacts": {
            "binary": str(binary),
            "binary_size_bytes": binary.stat().st_size,
            "binary_sha256": sha256(binary),
            "workload": str(workload),
            "workload_size_bytes": workload.stat().st_size,
            "workload_sha256": sha256(workload),
        },
        "oracle": {
            "mechanism": "independent Python IEEE-F64 loop over 100000 iterations",
            "iterations": EXPECTED_ITERATIONS,
            "expected_outcome": expected,
        },
        "pre_timing_verification": verification,
        "warmup_order": warmup_order,
        "warmups": warmups,
        "measured_order": measured_order,
        "samples": measured,
        "summary": summary,
        "comparisons": comparisons(summary),
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {output}")
    for variant in VARIANTS:
        wall = summary[variant]["process_wall_ns"]
        print(
            f"{variant}: wall median={wall['median']} ns MAD={wall['mad']} ns "
            f"p95={wall['p95']} ns RSS median={summary[variant]['peak_rss_kib']['median']} KiB"
        )
    print(json.dumps(result["comparisons"], sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"benchmark: {error}", file=sys.stderr)
        raise SystemExit(1)
