#!/usr/bin/env python3
"""Public-boundary performance observations for the checked lkjstudio product."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import resource
import statistics
import subprocess
import sys
import time
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_STUDIO = ROOT / "target" / "release" / "lkjstudio"
DEFAULT_PROJECT_CLI = ROOT / "target" / "release" / "lkjscript"
APPLICATION = ROOT / "applications" / "lkjstudio" / "lkjstudio.lkja"
PROJECT = ROOT / "applications" / "lkjstudio"


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            value.update(chunk)
    return value.hexdigest()


def percentile(samples: list[float], fraction: float) -> float:
    ordered = sorted(samples)
    index = max(0, math.ceil(len(ordered) * fraction) - 1)
    return ordered[index]


def run(
    arguments: list[str], input_bytes: bytes = b"", timeout: float = 300.0
) -> dict[str, Any]:
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.perf_counter_ns()
    completed = subprocess.run(
        arguments,
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=timeout,
    )
    elapsed = time.perf_counter_ns() - started
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(arguments)}\n"
            f"stdout={completed.stdout.decode(errors='replace')}\n"
            f"stderr={completed.stderr.decode(errors='replace')}"
        )
    return {
        "elapsed_ns": elapsed,
        "user_seconds": after.ru_utime - before.ru_utime,
        "system_seconds": after.ru_stime - before.ru_stime,
        "request_bytes": len(input_bytes),
        "response_bytes": len(completed.stdout),
        "stdout": completed.stdout,
    }


def summarize(samples: list[dict[str, Any]]) -> dict[str, Any]:
    milliseconds = [sample["elapsed_ns"] / 1_000_000 for sample in samples]
    return {
        "samples": len(samples),
        "elapsed_ms": milliseconds,
        "median_ms": statistics.median(milliseconds),
        "p95_ms": percentile(milliseconds, 0.95),
        "request_bytes": [sample["request_bytes"] for sample in samples],
        "response_bytes": [sample["response_bytes"] for sample in samples],
        "user_seconds": [sample["user_seconds"] for sample in samples],
        "system_seconds": [sample["system_seconds"] for sample in samples],
    }


def key(character: str, *, control: bool = False) -> dict[str, Any]:
    return {
        "kind": "key",
        "data": {
            "code": {"character": ord(character)},
            "control": control,
            "alt": False,
            "shift": False,
            "repeat": False,
        },
    }


def special(code: str) -> dict[str, Any]:
    return {
        "kind": "key",
        "data": {
            "code": code,
            "control": False,
            "alt": False,
            "shift": False,
            "repeat": False,
        },
    }


def headless(
    binary: Path, name: str, events: list[dict[str, Any]], timeout: float = 300.0
) -> dict[str, Any]:
    request = {
        "version": 3,
        "rows": 40,
        "columns": 120,
        "events": events,
        "outcomes": [],
    }
    encoded = json.dumps(request, separators=(",", ":")).encode()
    observation = run(
        [str(binary), "headless", "--artifact", str(APPLICATION)],
        encoded,
        timeout,
    )
    envelope = json.loads(observation.pop("stdout"))
    receipt = envelope["result"]
    return {
        "name": name,
        **observation,
        "event_count": receipt["event_count"],
        "action_count": receipt["action_count"],
        "changed_count": receipt["changed_count"],
        "exit_event": receipt["exit_event"],
        "replay_digest": receipt["replay_digest"],
        "final_frame_digest": receipt["final_frame_digest"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=DEFAULT_STUDIO)
    parser.add_argument("--project-cli", type=Path, default=DEFAULT_PROJECT_CLI)
    parser.add_argument("--samples", type=int, default=5)
    arguments = parser.parse_args()
    if arguments.samples < 1 or arguments.samples > 20:
        parser.error("--samples must be in 1..=20")
    for path in [arguments.binary, arguments.project_cli, APPLICATION]:
        if not path.is_file():
            parser.error(f"required file is absent: {path}")

    version_samples = [
        run(
            [
                str(arguments.binary),
                "version",
                "--artifact",
                str(APPLICATION),
            ]
        )
        for _ in range(arguments.samples)
    ]
    for sample in version_samples:
        sample.pop("stdout")

    orientation_samples = [
        run(
            [
                str(arguments.project_cli),
                "orient",
                "--project",
                str(PROJECT),
            ]
        )
        for _ in range(arguments.samples)
    ]
    for sample in orientation_samples:
        sample.pop("stdout")

    function_query = run(
        [
            str(arguments.project_cli),
            "query",
            "function",
            "--root",
            "render_workbench",
            "--project",
            str(PROJECT),
        ]
    )
    function_query.pop("stdout")
    proposal = run(
        [
            str(arguments.project_cli),
            "proposal",
            "render_workbench",
            "--project",
            str(PROJECT),
        ]
    )
    proposal.pop("stdout")

    mixed_events: list[dict[str, Any]] = []
    for _ in range(4_999):
        mixed_events.extend([key("x"), special("backspace")])
    mixed_events.extend(
        [{"kind": "resize", "data": {"rows": 41, "columns": 121}}, {"kind": "close"}]
    )

    observations = {
        "version": 1,
        "environment": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "cpu_count": os.cpu_count(),
            "measurement_clock": "time.perf_counter_ns",
            "page_cache": "not dropped",
            "process_rss": "unavailable",
        },
        "authority": {
            "project": str(PROJECT.relative_to(ROOT)),
            "revision": 48,
            "snapshot": "12898095ee151d9d0c6f46fdbd17838ed88febd17533c6c6badb731b1f4cf83e",
            "artifact_bytes": APPLICATION.stat().st_size,
            "artifact_sha256": digest(APPLICATION),
            "binary_bytes": arguments.binary.stat().st_size,
            "binary_sha256": digest(arguments.binary),
            "project_cli_bytes": arguments.project_cli.stat().st_size,
            "project_cli_sha256": digest(arguments.project_cli),
        },
        "process_inclusive": {
            "version_and_artifact_validation": summarize(version_samples),
            "project_orientation": summarize(orientation_samples),
            "function_query": function_query,
            "function_proposal": proposal,
        },
        "headless": [
            headless(arguments.binary, "mixed_10000", mixed_events, timeout=900.0),
            headless(
                arguments.binary,
                "insert_1000",
                [key("x") for _ in range(1_000)] + [{"kind": "close"}],
            ),
            headless(
                arguments.binary,
                "buffers_100",
                [key("n", control=True) for _ in range(99)] + [{"kind": "close"}],
            ),
            headless(
                arguments.binary,
                "maximum_paste",
                [{"kind": "paste", "data": [ord("x")] * 65_536}, {"kind": "close"}],
            ),
            headless(
                arguments.binary,
                "resize",
                [{"kind": "resize", "data": {"rows": 1, "columns": 1}}, {"kind": "close"}],
            ),
        ],
        "provider_telemetry": "unavailable; no token or monetary inference",
    }
    json.dump(observations, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError, ValueError) as error:
        print(f"lkjstudio workload failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
