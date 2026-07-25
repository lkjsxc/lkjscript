"""Oracle checks and deterministic interleaved sample campaign."""

from __future__ import annotations

import random
from pathlib import Path
from typing import Any

from jit_protocol.constants import CASE_NAMES, EXACT_I64_1, EXACT_I64_3333
from jit_protocol.evidence import validate_allocation_sample
from jit_protocol.process import engine_command, run_silent
from jit_protocol.statistics import run_metric_sample, summarize

def run_oracles(root, binary, optimizing, allocation, cases):
    checks = [run_silent(root, engine_command(binary, optimizing, "vm"),
                         "optimizing workload reference VM")]
    for name in CASE_NAMES:
        case = cases[name]
        checks.append(run_silent(root, engine_command(binary, case["workload"], case["engine"]), name))
    vm_oracle = run_metric_sample(root, binary, optimizing, "vm", EXACT_I64_3333, tier=None)
    if vm_oracle["metrics"]["jit"] is not None:
        raise RuntimeError("reference VM oracle unexpectedly reported JIT state")
    allocation_check = run_metric_sample(
        root, binary, allocation, "optimizing-jit", EXACT_I64_1, tier="optimizing"
    )
    validate_allocation_sample(allocation_check)
    return checks, vm_oracle, allocation_check

def collect_samples(root, binary, cases, arguments):
    randomizer = random.Random(arguments.seed)
    warmup_order = [name for name in CASE_NAMES for _ in range(arguments.warmups)]
    randomizer.shuffle(warmup_order)
    warmups: list[dict[str, Any]] = []
    signatures: dict[str, dict[str, Any]] = {}
    for ordinal, name in enumerate(warmup_order):
        case = cases[name]
        sample = run_metric_sample(
            root, binary, case["workload"], case["engine"], case["expected"],
            tier=case["tier"], proof_required=case["proof_required"],
        )
        sample["ordinal"] = ordinal
        sample["case"] = name
        signature = sample["exact_jit_facts"]
        if name in signatures and signature != signatures[name]:
            raise RuntimeError(f"{name} warmup changed exact JIT facts")
        signatures.setdefault(name, signature)
        warmups.append(sample)
    measured_order = [name for name in CASE_NAMES for _ in range(arguments.samples)]
    randomizer.shuffle(measured_order)
    measured: list[dict[str, Any]] = []
    for ordinal, name in enumerate(measured_order):
        case = cases[name]
        sample = run_metric_sample(
            root, binary, case["workload"], case["engine"], case["expected"],
            tier=case["tier"], proof_required=case["proof_required"],
        )
        sample["ordinal"] = ordinal
        sample["case"] = name
        if sample["exact_jit_facts"] != signatures[name]:
            raise RuntimeError(f"{name} measured sample changed exact JIT facts")
        measured.append(sample)
    by_case = {name: [sample for sample in measured if sample["case"] == name] for name in CASE_NAMES}
    summary = {name: summarize(by_case[name]) for name in CASE_NAMES}
    return warmup_order, warmups, measured_order, measured, signatures, summary
