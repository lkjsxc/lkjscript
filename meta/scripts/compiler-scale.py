#!/usr/bin/env python3
"""Run the release borrow-call compiler scale fixture and sample process-tree RSS."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import time
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
MARKER = "LKJSCRIPT_BORROW_SCALE "
TEST = "borrow_call_scale_sample"
EXACT_TEST = "sixteen_thousand_three_hundred_eighty_five_calls_and_borrow_scopes_execute_in_vm"


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
    digest.update(b"lkjscript.compiler-scale-worktree\0")
    digest.update(tracked_diff)
    untracked_hashes = {}
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
        "architecture": platform.machine(),
        "cpu": cpu,
        "logical_cpus": os.cpu_count(),
        "memory_bytes": memory_bytes,
        "rustc": command_output("rustc", "--version"),
        "cargo": command_output("cargo", "--version"),
    }


def process_tree_rss_bytes(root_pid: int) -> int:
    processes: dict[int, tuple[int, int]] = {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat = (entry / "stat").read_text(encoding="utf-8")
            close = stat.rfind(")")
            fields = stat[close + 2 :].split()
            ppid = int(fields[1])
            rss_pages = int(fields[21])
            processes[int(entry.name)] = (ppid, rss_pages)
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


def run_sample(calls: int, test: str = TEST, configured_geometry: bool = True) -> dict[str, Any]:
    command = [
        "cargo",
        "test",
        "--locked",
        "--release",
        "-p",
        "lkjscript-app",
        "--test",
        "source_scale",
        test,
        "--",
        "--ignored",
        "--exact",
        "--nocapture",
        "--test-threads=1",
    ]
    environment = os.environ.copy()
    if configured_geometry:
        environment["LKJSCRIPT_BORROW_CALLS"] = str(calls)
    else:
        environment.pop("LKJSCRIPT_BORROW_CALLS", None)
    started = time.monotonic_ns()
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    peak_rss = 0
    while process.poll() is None:
        peak_rss = max(peak_rss, process_tree_rss_bytes(process.pid))
        time.sleep(0.01)
    stdout, stderr = process.communicate()
    peak_rss = max(peak_rss, process_tree_rss_bytes(process.pid))
    elapsed_ns = time.monotonic_ns() - started
    if process.returncode != 0:
        raise RuntimeError(
            f"scale sample {calls} failed with exit {process.returncode}\n{stdout}\n{stderr}"
        )
    marker = next(
        (line[len(MARKER) :] for line in stderr.splitlines() if line.startswith(MARKER)),
        None,
    )
    if marker is None:
        raise RuntimeError(f"scale sample {calls} emitted no {MARKER.strip()} marker\n{stderr}")
    measured = json.loads(marker)
    measured.update(
        {
            "process_tree_peak_rss_bytes": peak_rss,
            "process_wall_ns": elapsed_ns,
        }
    )
    return measured


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", default="1024,2048,4096")
    parser.add_argument("--samples", type=int, default=1)
    parser.add_argument("--label", required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--exact-stress", action="store_true")
    args = parser.parse_args()
    sizes = [int(value) for value in args.sizes.split(",") if value]
    if not sizes or args.samples < 1 or any(size < 1 for size in sizes):
        parser.error("sizes and samples must select positive test geometry")

    build_command = [
        "cargo",
        "test",
        "--locked",
        "--release",
        "-p",
        "lkjscript-app",
        "--test",
        "source_scale",
        "--no-run",
    ]
    subprocess.run(build_command, cwd=ROOT, check=True)
    output = args.output or ROOT / "target" / "compiler-scale" / f"{args.label}.json"
    output.parent.mkdir(parents=True, exist_ok=True)

    results = []
    for size in sizes:
        for sample in range(args.samples):
            print(f"{args.label}: calls={size} sample={sample + 1}/{args.samples}", flush=True)
            measured = run_sample(size)
            measured["sample"] = sample + 1
            results.append(measured)

    exact_stress = None
    if args.exact_stress:
        print(f"{args.label}: exact stress test", flush=True)
        exact_stress = run_sample(16_385, EXACT_TEST, configured_geometry=False)

    worktree = worktree_metadata()
    document = {
        "label": args.label,
        "commit": command_output("git", "rev-parse", "HEAD"),
        "dirty": worktree["dirty"],
        "worktree": worktree,
        "cache_state": "warm Cargo dependencies and release artifacts; fresh test process per sample",
        "rss_method": "10 ms /proc polling; sum of resident pages for the cargo process tree",
        "command": " ".join(build_command[:-1])
        + f" {TEST} -- --ignored --exact --nocapture --test-threads=1",
        "environment_variable": "LKJSCRIPT_BORROW_CALLS",
        "exact_command": (
            "cargo test --locked --release -p lkjscript-app --test source_scale "
            f"{EXACT_TEST} -- --ignored --exact --nocapture --test-threads=1"
            if args.exact_stress
            else None
        ),
        "exact_stress": exact_stress,
        "machine": machine_metadata(),
        "sizes": sizes,
        "samples_per_size": args.samples,
        "results": results,
    }
    output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(output)


if __name__ == "__main__":
    main()
