"""Controlled benchmark process execution and metric parsing."""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path
from typing import Any

from jit_protocol.constants import METRICS_PREFIX

METRICS_CONTRACT = "a78abe6aeed3631290f28f28d4503acba4678e9c1f2e2fcd05bab78a4136b41a"

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
    if metrics.get("schema") != "lkjscript.metrics":
        raise RuntimeError(f"{engine} emitted unknown metrics schema {metrics.get('schema')!r}")
    if metrics.get("contract") != METRICS_CONTRACT:
        raise RuntimeError(
            f"{engine} emitted mismatched metrics contract {metrics.get('contract')!r}"
        )
    if metrics.get("engine") != engine:
        raise RuntimeError(f"{engine} metrics reported {metrics.get('engine')!r}")
    if metrics.get("outcome") != expected:
        raise RuntimeError(f"{engine} outcome {metrics.get('outcome')!r} != {expected!r}")
    return metrics
