#!/usr/bin/env python3
"""Correctness and end-to-end benchmark harness for the lkjscript BF interpreter."""

from __future__ import annotations

import pathlib
import shlex
import subprocess
import sys

from brainfuck_protocol.arguments import parse_args
from brainfuck_protocol.build import prepare_release
from brainfuck_protocol.constants import (
    INPUT_PATH, INPUT_SHA256, OUTPUT_LENGTH, OUTPUT_SHA256, REFERENCE_PATH,
    REFERENCE_SHA256, UPSTREAM_ROOT,
)
from brainfuck_protocol.correctness import run_correctness, run_diagnostic
from brainfuck_protocol.environment import machine_metadata
from brainfuck_protocol.files import fetch_verified
from brainfuck_protocol.oracle import verify_reference
from brainfuck_protocol.process import interpreter_command
from brainfuck_protocol.reference import build_reference
from brainfuck_protocol.reporting import base_result
from brainfuck_protocol.sampling import measure
from brainfuck_protocol.smoke import run_smokes

def main() -> int:
    args = parse_args()
    root = pathlib.Path(__file__).resolve().parents[3]
    work = root / "target/brainfuck-bench"
    source = root / "src/examples/brainfuck/main.lkjscript"
    work.mkdir(parents=True, exist_ok=True)
    binary = prepare_release(root, args.no_build)
    run_smokes(root, binary, source, work)
    if args.mode == "smoke":
        return 0

    input_file = work / "inputs/mandelbrot.bf"
    fetch_verified(f"{UPSTREAM_ROOT}/{INPUT_PATH}", input_file, INPUT_SHA256)
    reference, reference_build = build_reference(root, work)
    reference_output = work / "oracle-output.bin"
    reference_command = [str(reference), str(input_file)]
    command = interpreter_command(binary, source, input_file, args.fold_runs)
    oracle: dict[str, object] = {
        "kind": "independent repository C interpreter",
        "source_path": REFERENCE_PATH, "source_sha256": REFERENCE_SHA256,
        "build": reference_build, "command": shlex.join(reference_command),
        "output_length": OUTPUT_LENGTH, "output_sha256": OUTPUT_SHA256,
        "output_verified": False, "byte_equal": False,
    }
    result = base_result(args, command, reference_command, reference_build,
                         machine_metadata(root, binary), oracle)
    status = verify_reference(root, work, reference_command, reference_output,
                              OUTPUT_LENGTH, OUTPUT_SHA256, args.timeout, result, oracle)
    if status is not None:
        return status
    status = run_diagnostic(root, work, command, args, result, reference_output,
                            OUTPUT_LENGTH, OUTPUT_SHA256)
    if status is not None:
        return status
    status = run_correctness(root, work, command, args, result, oracle, reference_output,
                             OUTPUT_LENGTH, OUTPUT_SHA256)
    if status is not None:
        return status
    return measure(root, work, command, args, result, reference_output,
                   OUTPUT_LENGTH, OUTPUT_SHA256)

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
        print(f"benchmark: {error}", file=sys.stderr)
        raise SystemExit(1)
