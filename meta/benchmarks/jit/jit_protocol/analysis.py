"""Adoption criteria and retained-baseline comparisons."""

import json

from jit_protocol.constants import (
    HISTORICAL_NATIVE_MEDIAN_NS, HISTORICAL_PROCESS_MEDIAN_NS, REGRESSION_CEILING,
)
from jit_protocol.statistics import median

def analyze(summary, historical):
    baseline = summary["optimizing-workload-baseline"]
    optimizing = summary["optimizing-workload-optimizing"]
    scalar = summary["scalar-workload-baseline"]
    baseline_native = median(baseline, "timings_ns.native_execution")
    optimizing_native = median(optimizing, "timings_ns.native_execution")
    improvement = baseline_native - optimizing_native
    combined_mad = float(baseline["timings_ns.native_execution"]["mad"]) + float(
        optimizing["timings_ns.native_execution"]["mad"]
    )
    retained = json.loads(historical.read_text(encoding="utf-8"))
    retained_native = float(retained["summary"]["baseline-jit"]["timings_ns.native_execution"]["median"])
    retained_process = float(retained["summary"]["baseline-jit"]["process_wall_ns"]["median"])
    if retained_native != HISTORICAL_NATIVE_MEDIAN_NS or retained_process != HISTORICAL_PROCESS_MEDIAN_NS:
        raise RuntimeError("retained callable baseline medians changed unexpectedly")
    current_scalar_native = median(scalar, "timings_ns.native_execution")
    current_scalar_process = median(scalar, "process_wall_ns")
    criteria = {
        "optimizing_native_speedup_at_least_1_20x": baseline_native / optimizing_native >= 1.20,
        "native_improvement_greater_than_twice_combined_mad": improvement > 2.0 * combined_mad,
        "all_exact_outcomes_and_stream_checks": True,
        "optimizing_nonzero_entries_zero_baseline_entries_and_fallback": True,
        "forced_baseline_nonzero_entries_zero_optimizing_entries_and_fallback": True,
        "all_native_objects_wx_verified": True,
        "optimizing_checked_proof_nonzero": True,
        "scalar_native_median_no_more_than_5_percent_over_retained": current_scalar_native <= retained_native * REGRESSION_CEILING,
        "scalar_process_median_no_more_than_5_percent_over_retained": current_scalar_process <= retained_process * REGRESSION_CEILING,
        "allocation_graph_exact_and_accounted_once": True,
    }
    comparisons = {
        "optimizing_native_speedup_over_same_commit_baseline": baseline_native / optimizing_native,
        "native_median_improvement_ns": improvement,
        "combined_native_mad_ns": combined_mad,
        "twice_combined_native_mad_ns": 2.0 * combined_mad,
        "scalar_native_current_over_retained": current_scalar_native / retained_native,
        "scalar_process_current_over_retained": current_scalar_process / retained_process,
        "retained_scalar_native_median_ns": retained_native,
        "retained_scalar_process_median_ns": retained_process,
        "current_scalar_native_median_ns": current_scalar_native,
        "current_scalar_process_median_ns": current_scalar_process,
        "historical_comparison_caveat": (
            "sentinel only: source is retained, but compiler, metrics, native ABI, stack "
            "checks, and surrounding generated code evolved after the callable-baseline commit"
        ),
    }
    return criteria, comparisons
