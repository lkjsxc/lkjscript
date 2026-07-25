"""Stable forced-tier protocol constants."""

METRICS_PREFIX = b"LKJSCRIPT_METRICS "
SCHEMA = "lkjscript.optimizing-jit-benchmark.v1"
DEFAULT_SEED = 0x4C4B4A534F505449
CASE_NAMES = (
    "optimizing-workload-baseline",
    "optimizing-workload-optimizing",
    "scalar-workload-baseline",
)
EXACT_I64_3333 = {"kind": "returned", "value_kind": "i64", "exact": "3333"}
EXACT_I64_1 = {"kind": "returned", "value_kind": "i64", "exact": "1"}
SCALAR_ITERATIONS = 100_000
HISTORICAL_NATIVE_MEDIAN_NS = 7_647_935
HISTORICAL_PROCESS_MEDIAN_NS = 9_372_036
REGRESSION_CEILING = 1.05
