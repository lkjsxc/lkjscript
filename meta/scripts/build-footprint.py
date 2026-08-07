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
    diff = subprocess.check_output(["git", "diff", "--binary"], cwd=root)
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
    record = {
        "label": arguments.label,
        "base_commit": output(["git", "-C", str(root), "rev-parse", "HEAD"]),
        "working_tree_diff_sha256": hashlib.sha256(diff).hexdigest(),
        "dirty": bool(output(["git", "-C", str(root), "status", "--porcelain"])),
        "uname": platform.uname()._asdict(),
        "cpu_model": cpu_model,
        "logical_cpus": os.cpu_count(),
        "memory_bytes": memory_kib * 1024,
        "rustc": output(["rustc", "--version", "--verbose"]),
        "cargo": output(["cargo", "--version"]),
        "profile": "release (workspace LTO, codegen-units=1, strip=symbols)",
        "command": " ".join(command),
        "cold_definition": "fresh empty CARGO_TARGET_DIR; Cargo registry/cache retained",
        "warm_definition": "immediate unchanged no-op rebuild in the same target directory",
        "samples": [],
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    for index in range(1, arguments.samples + 1):
        target = Path(tempfile.mkdtemp(prefix=f"lkjscript-{arguments.label}-{index}-"))
        environment = os.environ.copy()
        environment["CARGO_TARGET_DIR"] = str(target)
        sample: dict[str, float | int] = {"index": index}
        try:
            for state in ("cold", "warm"):
                log = arguments.output.parent / f"{arguments.label}-{state}-{index}.log"
                started = time.perf_counter()
                with log.open("w") as stream:
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
            sample["binary_bytes"] = (target / "release/lkjscript").stat().st_size
            record["samples"].append(sample)
        finally:
            shutil.rmtree(target, ignore_errors=True)

    for key in ("cold_seconds", "warm_seconds", "binary_bytes"):
        values = [sample[key] for sample in record["samples"]]
        record[key] = {
            "samples": values,
            "median": statistics.median(values),
            "p95_nearest_rank": percentile_95(values),
        }
    arguments.output.write_text(json.dumps(record, indent=2) + "\n")
    print(arguments.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
