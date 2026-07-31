"""Stable evidence assembly, serialization, and CLI summary."""

import json
import os
import platform
import sys

from jit_protocol.artifacts import artifact
from jit_protocol.constants import (
    CASE_NAMES, CONTRACT, EXACT_I64_42, EXACT_I64_3333, SCALAR_ITERATIONS, SCHEMA,
)
from jit_protocol.environment import command_version, cpu_model, git_output, memory_kib

def assemble(root, arguments, repository_before, build, binary, workloads, oracle_data, sample_data, analysis):
    optimizing, scalar, allocation, historical, scalar_expected = workloads
    silence_checks, vm_oracle, allocation_check = oracle_data
    warmup_order, warmups, measured_order, measured, signatures, summary = sample_data
    criteria, comparisons = analysis
    sources = [
        root / "src/examples/jit-optimizing/main.lkjscript",
        root / "src/examples/jit-optimizing/kernel.lkjscript",
        root / "src/examples/jit-optimizing/kernel/redundant-divisions.lkjscript",
        root / "src/examples/jit-scalar/main.lkjscript",
        root / "src/examples/jit-scalar/kernel.lkjscript", allocation,
        root / "meta/benchmarks/jit/benchmark.py",
        *sorted((root / "meta/benchmarks/jit/jit_protocol").glob("*.py")),
        root / "Cargo.lock",
    ]
    return {
        "schema": SCHEMA,
        "contract": CONTRACT,
        "verdict": {
            "status": "Adopted" if all(criteria.values()) else "Rejected",
            "scope": "forced first optimizing-tier performance gate only",
            "automatic_promotion": "disabled and unmeasured", "criteria": criteria,
        },
        "repository": {
            "commit": git_output(root, ["rev-parse", "HEAD"]),
            "tree": git_output(root, ["rev-parse", "HEAD^{tree}"]),
            "dirty_before_benchmark": bool(repository_before),
            "dirty_paths_before_benchmark": repository_before.splitlines(),
        },
        "environment": {
            "os": platform.system(), "platform": platform.platform(),
            "kernel": platform.release(), "machine": platform.machine(),
            "cpu": cpu_model(), "logical_cpus": os.cpu_count(),
            "memory_kib": memory_kib(), "python": sys.version.splitlines()[0],
            "rustc": command_version(["rustc", "--version", "--verbose"]),
            "cargo": command_version(["cargo", "--version", "--verbose"]),
            "git": command_version(["git", "--version"]),
        },
        "build": build,
        "protocol": {
            "seed": arguments.seed, "seed_hex": hex(arguments.seed),
            "warmups_per_case": arguments.warmups,
            "measured_samples_per_case": arguments.samples, "cases": list(CASE_NAMES),
            "interleaving": "one deterministic randomized order across all three cases",
            "samples_removed": 0,
            "metrics_transport": "one LKJSCRIPT_METRICS JSON stderr line per measured execution",
            "normal_stream_policy": "stdout and stderr both empty when metrics are disabled",
            "wall_clock": "Python time.monotonic_ns around process creation through collection",
            "rss_source": "/proc/<pid>/status VmRSS polled approximately every 0.5 ms; maximum observed",
            "p95": "nearest-rank", "mad": "median absolute deviation",
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
                "expected_outcome": EXACT_I64_3333, "sample": vm_oracle,
            },
            "scalar_workload": {
                "mechanism": "independent Python IEEE-F64 recurrence",
                "iterations": SCALAR_ITERATIONS, "expected_outcome": scalar_expected,
            },
            "allocation_graph": {
                "mechanism": "exact known graph result plus runtime-value accounting invariants",
                "expected_outcome": EXACT_I64_42, "sample": allocation_check,
            },
        },
        "normal_stream_checks": silence_checks, "exact_case_jit_facts": signatures,
        "warmup_order": warmup_order, "warmups": warmups,
        "measured_order": measured_order, "samples": measured,
        "summary": summary, "comparisons": comparisons,
    }

def write_and_summarize(output, result):
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {output}")
    for name in CASE_NAMES:
        native = result["summary"][name]["timings_ns.native_execution"]
        wall = result["summary"][name]["process_wall_ns"]
        print(
            f"{name}: native median={native['median']} ns MAD={native['mad']} ns; "
            f"wall median={wall['median']} ns MAD={wall['mad']} ns"
        )
    comparisons = result["comparisons"]
    print(
        f"speedup={comparisons['optimizing_native_speedup_over_same_commit_baseline']:.6f}x "
        f"verdict={result['verdict']['status']}"
    )
    for name, passed in result["verdict"]["criteria"].items():
        print(f"criterion {name}={'pass' if passed else 'fail'}")
