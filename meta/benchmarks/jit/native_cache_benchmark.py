#!/usr/bin/env python3
import hashlib
import json
import math
import os
import platform
import random
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]
BINARY = ROOT / "target/release/lkjscript"
CACHE = ROOT / "target/lkjscript/native-cache"
WORK = ROOT / "target/lkjscript/cache-benchmark"
RESULT = ROOT / "meta/benchmarks/jit/results/native-image-cache-selection.json"
SAMPLES = 30
WARMUPS = 5
WORKLOADS = [
    ("scalar", "baseline-jit", "src/examples/jit-scalar/main.lkjscript", []),
    ("allocation", "baseline-jit", "crates/lkjscript-app/tests/fixtures/allocation-graph.lkjscript", []),
    ("brainfuck", "auto", "src/examples/brainfuck/main.lkjscript",
     [str(ROOT / "meta/benchmarks/brainfuck/fixtures/hello.bf")]),
    ("editor", "auto", "src/examples/lkjedit/main.lkjscript", []),
    ("sqlite", "auto", "src/examples/sqlite/main.lkjscript", []),
]
CONDITIONS = ["disabled", "cold_miss", "warm_hit"]

def percentile(values, ratio):
    ordered = sorted(values)
    return ordered[math.ceil(len(ordered) * ratio) - 1]


def empty_cache():
    shutil.rmtree(CACHE, ignore_errors=True)
    CACHE.mkdir(mode=0o700, parents=True)

def prepare(condition, template):
    if condition == "cold_miss":
        empty_cache()
    elif condition == "warm_hit":
        shutil.rmtree(CACHE, ignore_errors=True)
        shutil.copytree(template, CACHE)
def invoke(workload, condition, measured):
    name, engine, source, extra = workload
    if name == "sqlite":
        Path("/tmp/lkjscript-sqlite-example.db").unlink(missing_ok=True)
    metrics = Path(tempfile.mktemp(prefix="lkj-cache-metrics-", dir=WORK))
    mode = "disabled" if condition == "disabled" else "local"
    command = [str(BINARY), "run", "--engine", engine]
    if engine == "auto":
        command += ["--auto-jit-threshold", "1"]
    command += ["--native-cache", mode, source, *extra]
    env = os.environ.copy()
    env.update(LKJSCRIPT_METRICS="1", LKJSCRIPT_METRICS_FILE=str(metrics))
    started = time.perf_counter_ns()
    completed = subprocess.Popen(command, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    peak_rss = 0
    while completed.poll() is None:
        try:
            status = Path(f"/proc/{completed.pid}/status").read_text()
            rss_line = next(line for line in status.splitlines() if line.startswith("VmRSS:"))
            peak_rss = max(peak_rss, int(rss_line.split()[1]))
        except (FileNotFoundError, StopIteration):
            pass
        time.sleep(0.0005)
    stdout, stderr = completed.communicate()
    wall = time.perf_counter_ns() - started
    if completed.returncode != 0:
        raise RuntimeError(f"{name}/{condition}: {stderr.decode()}")
    text = metrics.read_text().strip().removeprefix("LKJSCRIPT_METRICS ")
    data = json.loads(text)
    metrics.unlink()
    if not measured:
        return None
    jit = data["jit"]
    timings = data["timings_ns"]
    return {
        "workload": name,
        "condition": condition,
        "wall_ns": wall,
        "rss_kib": peak_rss,
        "compile_ns": timings["compile_total"] or 0,
        "first_native_ns": timings["time_to_first_native_entry"] or 0,
        "engine_ns": timings["engine_execution"] or 0,
        "cache_lookups": jit["cache_lookups"],
        "cache_hits": jit["cache_hits"],
        "cache_misses": jit["cache_misses"],
        "cache_lookup_ns": jit["cache_lookup_ns"],
        "cache_publication_ns": jit["cache_publication_ns"],
        "native_entries": jit["native_entries"],
        "vm_fallbacks": jit["vm_fallbacks"],
    }
def source_digest(path):
    return hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
def aggregate(samples):
    result = {}
    for name, *_ in WORKLOADS:
        result[name] = {}
        for condition in CONDITIONS:
            rows = [row for row in samples if row["workload"] == name
                    and row["condition"] == condition]
            result[name][condition] = {
                key: {"p50": int(statistics.median(row[key] for row in rows)),
                      "p95": percentile([row[key] for row in rows], 0.95)}
                for key in ["wall_ns", "rss_kib", "compile_ns", "first_native_ns",
                            "engine_ns", "cache_lookup_ns", "cache_publication_ns"]
            }
            result[name][condition]["cache_hits"] = sum(row["cache_hits"] for row in rows)
            result[name][condition]["cache_misses"] = sum(row["cache_misses"] for row in rows)
    return result
def decisions(summary):
    decisions = {}
    for name, *_ in WORKLOADS:
        disabled = summary[name]["disabled"]
        cold = summary[name]["cold_miss"]
        warm = summary[name]["warm_hit"]
        wall_gain = 1 - warm["wall_ns"]["p50"] / disabled["wall_ns"]["p50"]
        first = disabled["first_native_ns"]["p50"]
        first_gain = 0 if first == 0 else 1 - warm["first_native_ns"]["p50"] / first
        cold_p50 = cold["wall_ns"]["p50"] / disabled["wall_ns"]["p50"] - 1
        cold_p95 = cold["wall_ns"]["p95"] / disabled["wall_ns"]["p95"] - 1
        rss = warm["rss_kib"]["p50"] / disabled["rss_kib"]["p50"] - 1
        saving = disabled["wall_ns"]["p50"] - warm["wall_ns"]["p50"]
        cost = cold["wall_ns"]["p50"] - disabled["wall_ns"]["p50"]
        break_even = None if saving <= 0 else max(1, math.ceil(max(0, cost) / saving))
        decisions[name] = {"warm_wall_gain": wall_gain, "warm_first_native_gain": first_gain,
                           "cold_p50_regression": cold_p50, "cold_p95_regression": cold_p95,
                           "warm_rss_regression": rss, "break_even_executions": break_even}
    return decisions
def main():
    subprocess.run(["cargo", "build", "--locked", "--workspace", "--release"],
                   cwd=ROOT, check=True)
    shutil.rmtree(WORK, ignore_errors=True)
    WORK.mkdir(parents=True)
    keys = {}
    for workload in WORKLOADS:
        empty_cache()
        invoke(workload, "warm_hit", False)
        keys[workload[0]] = sorted(path.stem for path in (CACHE / "objects").glob("*.image"))
    empty_cache()
    for workload in WORKLOADS:
        invoke(workload, "warm_hit", False)
    template = WORK / "warm-template"
    shutil.copytree(CACHE, template)
    for workload in WORKLOADS:
        for condition in CONDITIONS:
            for _ in range(WARMUPS):
                prepare(condition, template)
                invoke(workload, condition, False)
    samples = []
    randomizer = random.Random(0x4C4B4A)
    combinations = [(workload, condition) for workload in WORKLOADS for condition in CONDITIONS]
    for _ in range(SAMPLES):
        randomizer.shuffle(combinations)
        for workload, condition in combinations:
            prepare(condition, template)
            samples.append(invoke(workload, condition, True))
    summary = aggregate(samples)
    comparison = decisions(summary)
    eligible = [name for name in ["scalar", "allocation", "brainfuck"] if keys[name]]
    passes = [name for name in eligible if comparison[name]["warm_wall_gain"] >= 0.10
              and comparison[name]["warm_first_native_gain"] >= 0.20]
    adopted = len(passes) >= 3 and all(
        comparison[name]["cold_p50_regression"] <= 0.10
        and comparison[name]["cold_p95_regression"] <= 0.05
        and comparison[name]["warm_rss_regression"] <= 0.05
        and comparison[name]["break_even_executions"] is not None
        and comparison[name]["break_even_executions"] <= 5 for name in eligible)
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    result = {
        "schema": "lkjscript.native-image-cache-benchmark",
        "candidate_commit": commit,
        "samples_per_combination": SAMPLES,
        "warmups_per_combination": WARMUPS,
        "random_seed": "0x4c4b4a",
        "environment": {"uname": platform.platform(),
                        "cpu": platform.processor() or Path("/proc/cpuinfo").read_text()
                        .split("model name\t: ")[1].splitlines()[0],
                        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True).strip()},
        "command": "python3 meta/benchmarks/jit/native_cache_benchmark.py",
        "workloads": [{"name": name, "engine": engine, "source": source,
                       "source_sha256": source_digest(source), "artifact_keys": keys[name]}
                      for name, engine, source, _ in WORKLOADS],
        "package_lock_sha256": source_digest("lkjscript.lock.json"),
        "summary": summary,
        "comparisons": comparison,
        "adoption": {"eligible_workloads": eligible, "passing_workloads": passes,
                     "adopted": adopted},
        "samples": samples,
    }
    RESULT.write_text(json.dumps(result, indent=2) + "\n")
    shutil.rmtree(CACHE, ignore_errors=True)
    shutil.rmtree(WORK, ignore_errors=True)
    print(json.dumps(result["adoption"], sort_keys=True))
if __name__ == "__main__":
    main()
