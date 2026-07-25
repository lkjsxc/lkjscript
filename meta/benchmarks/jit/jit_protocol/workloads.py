"""Workload identities, expected outcomes, and independent oracle."""

import struct
from pathlib import Path

from jit_protocol.constants import (
    EXACT_I64_3333, SCALAR_ITERATIONS,
)

def scalar_oracle(iterations: int) -> dict[str, str]:
    accumulator = 0.0
    for index in range(iterations):
        accumulator += 1.0 / (2.0 * float(index) + 1.0)
    bits = struct.unpack("!Q", struct.pack("!d", accumulator))[0]
    return {
        "kind": "returned",
        "value_kind": "f64-bits",
        "exact": f"0x{bits:016x}",
    }


def prepare_cases(root: Path):
    optimizing = root / "src/examples/jit-optimizing/main.lkjscript"
    scalar = root / "src/examples/jit-scalar/main.lkjscript"
    allocation = root / "crates/lkjscript-app/tests/fixtures/allocation-graph.lkjscript"
    historical = root / "meta/benchmarks/jit/results/callable-baseline-jit-linux-x86_64.json"
    if any(not path.is_file() for path in (optimizing, scalar, allocation, historical)):
        raise RuntimeError("a required workload or retained baseline is missing")
    scalar_expected = scalar_oracle(SCALAR_ITERATIONS)
    cases = {
        "optimizing-workload-baseline": {
            "workload": optimizing, "engine": "baseline-jit",
            "expected": EXACT_I64_3333, "tier": "baseline", "proof_required": False,
        },
        "optimizing-workload-optimizing": {
            "workload": optimizing, "engine": "optimizing-jit",
            "expected": EXACT_I64_3333, "tier": "optimizing", "proof_required": True,
        },
        "scalar-workload-baseline": {
            "workload": scalar, "engine": "baseline-jit",
            "expected": scalar_expected, "tier": "baseline", "proof_required": False,
        },
    }
    return optimizing, scalar, allocation, historical, scalar_expected, cases
