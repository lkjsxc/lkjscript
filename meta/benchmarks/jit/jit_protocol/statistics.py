"""Sampling execution and deterministic distribution summaries."""

from __future__ import annotations

import math
import statistics
from pathlib import Path
from typing import Any

from jit_protocol.evidence import exact_jit_facts, validate_forced_metrics
from jit_protocol.process import engine_command, execute_process, parse_metrics

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
