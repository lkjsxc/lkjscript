#!/usr/bin/env python3
"""Bounded public-boundary performance observations for the checked lkjedit product."""

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
DEFAULT_EDITOR = ROOT / "target" / "release" / "lkjedit"
DEFAULT_PROJECT_CLI = ROOT / "target" / "release" / "lkjscript"
APPLICATION = Path(
    os.environ.get(
        "LKJEDIT_TEST_ARTIFACT",
        ROOT / "applications" / "lkjedit" / "lkjedit.lkja",
    )
)
PROJECT = ROOT / "applications" / "lkjedit"


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
    arguments: list[str],
    input_bytes: bytes = b"",
    timeout: float = 300.0,
    *,
    allow_failure: bool = False,
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
    if completed.returncode != 0 and not allow_failure:
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
        "returncode": completed.returncode,
        "stderr": completed.stderr.decode(errors="replace"),
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


def event(kind: str, data: Any | None = None) -> dict[str, Any]:
    value: dict[str, Any] = {"kind": "event", "data": {"kind": kind}}
    if data is not None:
        value["data"]["data"] = data
    return value


def key(character: str, *, control: bool = False) -> dict[str, Any]:
    return event(
        "key",
        {
            "code": {"character": ord(character)},
            "control": control,
            "alt": False,
            "shift": False,
            "repeat": False,
        },
    )


def special(code: str) -> dict[str, Any]:
    return event(
        "key",
        {
            "code": code,
            "control": False,
            "alt": False,
            "shift": False,
            "repeat": False,
        },
    )


def command(value: str) -> list[dict[str, Any]]:
    return [key(":"), *(key(character) for character in value), special("enter")]


def headless(
    binary: Path,
    name: str,
    transitions: list[dict[str, Any]],
    timeout: float = 300.0,
) -> dict[str, Any]:
    request = {
        "version": 4,
        "rows": 40,
        "columns": 120,
        "transitions": transitions,
    }
    encoded = json.dumps(request, separators=(",", ":")).encode()
    started = time.perf_counter_ns()
    try:
        observation = run(
            [str(binary), "headless", "--artifact", str(APPLICATION)],
            encoded,
            timeout,
            allow_failure=True,
        )
    except subprocess.TimeoutExpired as error:
        return {
            "name": name,
            "status": "exhausted",
            "timeout_seconds": timeout,
            "elapsed_ns_lower_bound": time.perf_counter_ns() - started,
            "transition_count": len(transitions),
            "request_bytes": len(encoded),
            "response_bytes": len(error.stdout or b""),
            "action_count": None,
            "changed_count": None,
            "exit_transition": None,
            "replay_digest": None,
            "final_frame_digest": None,
        }
    envelope = json.loads(observation.pop("stdout"))
    if observation["returncode"] != 0:
        error = envelope.get("error", {})
        code = str(error.get("code", "unknown"))
        status = "exhausted" if "policy_exceeded" in code or "exhausted" in code else "failed"
        return {
            "name": name,
            "status": status,
            **observation,
            "transition_count": len(transitions),
            "action_count": None,
            "changed_count": None,
            "exit_transition": None,
            "replay_digest": None,
            "final_frame_digest": None,
            "failure_code": code,
            "failure_message": error.get("message", "headless command failed"),
        }
    receipt = envelope["result"]
    return {
        "name": name,
        "status": "completed",
        **observation,
        "transition_count": receipt["transition_count"],
        "action_count": receipt["action_count"],
        "changed_count": receipt["changed_count"],
        "exit_transition": receipt["exit_transition"],
        "replay_digest": receipt["replay_digest"],
        "final_frame_digest": receipt["final_frame_digest"],
    }


def changed_orientation(stdout: bytes) -> dict[str, Any]:
    result = json.loads(stdout)["result"]
    if result["kind"] != "orientation":
        raise RuntimeError("project orientation returned the wrong result kind")
    orientation = result["data"]
    if orientation["kind"] != "changed":
        raise RuntimeError("project orientation unexpectedly returned unchanged")
    return orientation["data"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=DEFAULT_EDITOR)
    parser.add_argument("--project-cli", type=Path, default=DEFAULT_PROJECT_CLI)
    parser.add_argument("--samples", type=int, default=5)
    arguments = parser.parse_args()
    if arguments.samples < 1 or arguments.samples > 20:
        parser.error("--samples must be in 1..=20")
    for path in [arguments.binary, arguments.project_cli, APPLICATION]:
        if not path.is_file():
            parser.error(f"required file is absent: {path}")

    version_samples = [
        run([str(arguments.binary), "version", "--artifact", str(APPLICATION)])
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
    orientation = changed_orientation(orientation_samples[0]["stdout"])
    for sample in orientation_samples:
        sample.pop("stdout")

    function_query = run(
        [
            str(arguments.project_cli),
            "query",
            "function",
            "--root",
            "tab_segment_width_model",
            "--project",
            str(PROJECT),
        ]
    )
    function_query.pop("stdout")
    proposal = run(
        [
            str(arguments.project_cli),
            "proposal",
            "tab_segment_width_model",
            "--project",
            str(PROJECT),
        ]
    )
    proposal.pop("stdout")

    mixed_transitions: list[dict[str, Any]] = [key("i")]
    for _ in range(4_997):
        mixed_transitions.extend([key("x"), special("backspace")])
    mixed_transitions.extend([special("escape"), *command("q!")])
    if len(mixed_transitions) != 10_000:
        raise RuntimeError("mixed workload construction is not exactly 10,000 transitions")

    tab_transitions: list[dict[str, Any]] = []
    for _ in range(99):
        tab_transitions.extend(command("tabnew"))
    tab_transitions.extend(command("q"))

    observations = {
        "version": 2,
        "environment": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "cpu_count": os.cpu_count(),
            "measurement_clock": "time.perf_counter_ns",
            "classification": "optimized release; warm host caches not dropped",
            "process_rss": "unavailable",
        },
        "authority": {
            "project": str(PROJECT.relative_to(ROOT)),
            "revision": orientation["revision"],
            "snapshot": orientation["snapshot"],
            "revision_record": orientation["revision_record"],
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
            headless(arguments.binary, "mixed_10000", mixed_transitions, timeout=120.0),
            headless(
                arguments.binary,
                "growing_insert_1000",
                [
                    key("i"),
                    *[key("x") for _ in range(1_000)],
                    special("escape"),
                    *command("q!"),
                ],
                timeout=120.0,
            ),
            headless(arguments.binary, "tabs_100", tab_transitions, timeout=120.0),
            headless(
                arguments.binary,
                "maximum_paste_65536_scalars",
                [
                    key("i"),
                    event("paste", [ord("x")] * 65_536),
                    special("escape"),
                    *command("q!"),
                ],
                timeout=120.0,
            ),
            headless(
                arguments.binary,
                "resize_1_by_1",
                [event("resize", {"rows": 1, "columns": 1}), *command("q")],
                timeout=120.0,
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
        print(f"lkjedit workload failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
