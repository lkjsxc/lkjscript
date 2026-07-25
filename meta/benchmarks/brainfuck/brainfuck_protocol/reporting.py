"""Stable Brainfuck evidence assembly and serialization."""

from __future__ import annotations

import datetime as dt
import json
import pathlib
import shlex
import sys

from brainfuck_protocol.constants import (
    INPUT_PATH, INPUT_SHA256, LICENSE_PATH, LICENSE_SHA256, REFERENCE_PATH,
    REFERENCE_SHA256, UPSTREAM_COMMIT, UPSTREAM_ROOT,
)

def write_result(work: pathlib.Path, result: dict[str, object]) -> pathlib.Path:
    results = work / "results"
    results.mkdir(parents=True, exist_ok=True)
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    path = results / f"brainfuck-mandelbrot-{stamp}.json"
    encoded = json.dumps(result, indent=2, sort_keys=True) + "\n"
    path.write_text(encoded)
    (results / "latest.json").write_text(encoded)
    return path


def base_result(args, command, reference_command, reference_build, metadata, oracle):
    return {
        "status": "running",
        "metric": "end-to-end process wall time (compile + initialize + interpret + output)",
        "mode": "release",
        "release_build_performed": not args.no_build,
        "interpreter_variant": "run-folded" if args.fold_runs else "direct",
        "optional_run_folding": args.fold_runs,
        "command": shlex.join(command),
        "harness_command": shlex.join([sys.executable, *sys.argv]),
        "timeout_seconds": args.timeout,
        "diagnostic": {"status": "not-run"},
        "warmups": args.warmups,
        "measured_runs": args.runs,
        "upstream": {
            "repository": "https://github.com/pablojorge/brainfuck",
            "commit": UPSTREAM_COMMIT,
            "input_path": INPUT_PATH,
            "input_url": f"{UPSTREAM_ROOT}/{INPUT_PATH}",
            "input_sha256": INPUT_SHA256,
            "attribution": "Mandelbrot Brainfuck program by Erik Bosman",
            "license_path": LICENSE_PATH,
            "license_sha256": LICENSE_SHA256,
        },
        "oracle": oracle,
        "environment": metadata,
    }
