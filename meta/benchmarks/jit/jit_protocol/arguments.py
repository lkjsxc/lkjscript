"""Retained forced optimizing-JIT benchmark protocol for Linux x86-64."""

import argparse
from pathlib import Path

from jit_protocol.artifacts import repository_root
from jit_protocol.constants import DEFAULT_SEED

def parse_args() -> argparse.Namespace:
    root = repository_root()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=root / "target/release/lkjscript")
    parser.add_argument("--warmups", type=int, default=4)
    parser.add_argument("--samples", type=int, default=31)
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=DEFAULT_SEED)
    parser.add_argument(
        "--output",
        type=Path,
        default=root / "meta/benchmarks/jit/results/optimizing-jit-linux-x86_64.json",
    )
    arguments = parser.parse_args()
    if arguments.warmups < 4:
        parser.error("retained runs require at least four warmups per case")
    if arguments.samples < 31:
        parser.error("retained runs require at least 31 measured samples per case")
    return arguments
