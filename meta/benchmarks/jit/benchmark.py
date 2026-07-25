#!/usr/bin/env python3
"""Retained forced optimizing-JIT benchmark protocol for Linux x86-64."""

from __future__ import annotations

import platform
import subprocess
import sys

from jit_protocol.analysis import analyze
from jit_protocol.arguments import parse_args
from jit_protocol.artifacts import repository_root
from jit_protocol.build import locked_release_build
from jit_protocol.campaign import collect_samples, run_oracles
from jit_protocol.environment import git_output
from jit_protocol.reporting import assemble, write_and_summarize
from jit_protocol.workloads import prepare_cases

def main() -> int:
    arguments = parse_args()
    root = repository_root()
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        raise RuntimeError("the retained protocol requires Linux x86-64")
    repository_before = git_output(root, ["status", "--porcelain"])
    build = locked_release_build(root)
    binary = arguments.binary.resolve()
    if not binary.is_file():
        raise RuntimeError(f"missing release binary {binary}")
    optimizing, scalar, allocation, historical, scalar_expected, cases = prepare_cases(root)
    oracle_data = run_oracles(root, binary, optimizing, allocation, cases)
    sample_data = collect_samples(root, binary, cases, arguments)
    analysis = analyze(sample_data[-1], historical)
    result = assemble(
        root, arguments, repository_before, build, binary,
        (optimizing, scalar, allocation, historical, scalar_expected),
        oracle_data, sample_data, analysis,
    )
    write_and_summarize(arguments.output, result)
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError, KeyError, ValueError) as error:
        print(f"benchmark: {error}", file=sys.stderr)
        raise SystemExit(1)
