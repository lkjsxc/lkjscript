"""Exact forced-tier and allocation evidence checks."""

from __future__ import annotations

from typing import Any

def wx_is_verified(jit: dict[str, Any]) -> bool:
    objects = jit.get("objects")
    return isinstance(objects, list) and bool(objects) and all(
        obj.get("wx_verified") is True for obj in objects
    )


def validate_forced_metrics(metrics: dict[str, Any], tier: str, *, proof_required: bool) -> None:
    jit = metrics.get("jit")
    if not isinstance(jit, dict):
        raise RuntimeError(f"forced {tier} sample omitted JIT metrics")
    if jit.get("compile_failures") != 0 or jit.get("vm_fallbacks") != 0:
        raise RuntimeError(f"forced {tier} sample reported failure/fallback")
    if not wx_is_verified(jit):
        raise RuntimeError(f"forced {tier} sample did not prove W^X on every object")
    objects = jit["objects"]
    functions = jit.get("functions")
    if not isinstance(functions, list) or not functions:
        raise RuntimeError(f"forced {tier} sample omitted function tier facts")
    if tier == "baseline":
        if jit.get("baseline_native_entries", 0) <= 0:
            raise RuntimeError("forced baseline sample had no baseline entry")
        if jit.get("optimizing_native_entries") != 0:
            raise RuntimeError("forced baseline sample entered optimizing code")
        if jit.get("baseline_code_objects", 0) <= 0 or jit.get("optimizing_code_objects") != 0:
            raise RuntimeError("forced baseline sample reported wrong object tier counts")
        if any(obj.get("tier") != "Baseline" for obj in objects):
            raise RuntimeError("forced baseline sample retained a non-baseline object")
        if any(function.get("state") != "BaselineNative" for function in functions):
            raise RuntimeError("forced baseline sample retained a non-baseline function state")
    else:
        if jit.get("optimizing_native_entries", 0) <= 0:
            raise RuntimeError("forced optimizing sample had no optimizing entry")
        if jit.get("baseline_native_entries") != 0:
            raise RuntimeError("forced optimizing sample entered baseline code")
        if jit.get("optimizing_code_objects", 0) <= 0 or jit.get("baseline_code_objects") != 0:
            raise RuntimeError("forced optimizing sample reported wrong object tier counts")
        if any(obj.get("tier") != "Optimizing" for obj in objects):
            raise RuntimeError("forced optimizing sample retained a non-optimizing object")
        if any(function.get("state") != "OptimizedNative" for function in functions):
            raise RuntimeError("forced optimizing sample retained a non-optimized function state")
        if proof_required and (
            jit.get("optimization_certificate_records", 0) <= 0
            or jit.get("checked_i64_rewrites", 0) <= 0
            or jit.get("optimizing_passes", 0) <= 0
        ):
            raise RuntimeError("forced optimizing sample omitted executed proof evidence")


def exact_jit_facts(metrics: dict[str, Any]) -> dict[str, Any] | None:
    jit = metrics.get("jit")
    if jit is None:
        return None
    top_names = (
        "compile_failures",
        "vm_fallbacks",
        "native_entries",
        "baseline_native_entries",
        "optimizing_native_entries",
        "baseline_code_objects",
        "optimizing_code_objects",
        "optimizing_passes",
        "optimization_discovery_passes",
        "optimization_checker_passes",
        "optimization_reconstruction_passes",
        "optimization_cleanup_passes",
        "optimization_validation_passes",
        "optimization_certificate_records",
        "optimization_certificate_bytes_estimate",
        "algebraic_rewrites",
        "gvn_rewrites",
        "checked_i64_rewrites",
        "direct_native_calls",
        "poll_calls",
        "native_invocations",
        "code_cache_peak_objects",
        "code_cache_peak_bytes",
        "metadata_cache_peak_bytes",
        "accounted_allocation_peak_bytes",
        "runtime_value_attempts",
        "runtime_value_successes",
        "segmented_lists",
        "island",
        "peak_native_frame_depth",
        "vm_to_native_transitions",
        "native_to_vm_transitions",
    )
    object_names = (
        "identity",
        "tier",
        "functions",
        "code_bytes",
        "metadata_bytes",
        "optimization_metadata_bytes_estimate",
        "accounted_allocation_bytes",
        "relocations",
        "work_units",
        "optimization_work_units",
        "input_instructions",
        "output_instructions",
        "instruction_growth",
        "cleanup_removed_instructions",
        "iterations",
        "optimizing_passes",
        "discovery_passes",
        "checker_passes",
        "reconstruction_passes",
        "cleanup_passes",
        "validation_passes",
        "certificate_records",
        "certificate_bytes_estimate",
        "algebraic_rewrites",
        "gvn_rewrites",
        "checked_i64_rewrites",
        "native_entries",
        "wx_verified",
    )
    function_names = (
        "id",
        "name",
        "state",
        "calls",
        "attempts",
        "failure",
        "code_object",
        "epoch",
        "native_entries",
    )
    return {
        "top": {name: jit[name] for name in top_names},
        "functions": [
            {name: function[name] for name in function_names}
            for function in jit["functions"]
        ],
        "objects": [
            {name: obj[name] for name in object_names} for obj in jit["objects"]
        ],
    }


def validate_allocation_sample(sample: dict[str, Any]) -> None:
    jit = sample["metrics"]["jit"]
    lists = jit["segmented_lists"]
    if lists["segment_allocations"] < 1 or lists["live_entries"] < 2:
        raise RuntimeError("allocation graph did not report expected list storage")
    if jit["runtime_value_attempts"] < 12:
        raise RuntimeError("allocation graph did not reach expected runtime-value operation count")
    if jit["runtime_value_attempts"] != jit["runtime_value_successes"]:
        raise RuntimeError("allocation graph reported a failed runtime-value operation")
