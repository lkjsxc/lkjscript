"""Semantic validation of one retained AI-authorability result."""

from __future__ import annotations

import hashlib
import pathlib
from typing import Any

from validation.schema import (
    ACCEPTANCE_KEYS, METRIC_KEYS, RESULT_SCHEMA, TASK_SCHEMA, TOP_LEVEL_KEYS,
    canonical_sha256, load_json, require_exact_keys, require_nonnegative_or_null,
    task_path,
)

def validate(path: pathlib.Path) -> dict[str, Any]:
    result = load_json(path)
    require_exact_keys(str(path), result, TOP_LEVEL_KEYS)
    if result["schema"] != RESULT_SCHEMA:
        raise ValueError(f"{path}: unsupported result schema {result['schema']!r}")

    task = load_json(task_path(path, result["taskId"]))
    if task.get("schema") != TASK_SCHEMA:
        raise ValueError(f"{path}: unsupported task schema")
    if task.get("taskId") != result["taskId"]:
        raise ValueError(f"{path}: task identity mismatch")
    if canonical_sha256(task) != result["taskSha256"]:
        raise ValueError(f"{path}: task SHA-256 mismatch")
    prompt = task.get("prompt")
    if not isinstance(prompt, str):
        raise ValueError(f"{path}: task prompt must be a string")
    prompt_bytes = prompt.encode()
    if len(prompt_bytes) != task.get("promptUtf8Bytes"):
        raise ValueError(f"{path}: prompt byte count mismatch")
    if hashlib.sha256(prompt_bytes).hexdigest() != task.get("promptSha256"):
        raise ValueError(f"{path}: prompt SHA-256 mismatch")
    if result["baseCommit"] != task.get("baseCommit"):
        raise ValueError(f"{path}: base commit mismatch")

    metrics = result["metrics"]
    if not isinstance(metrics, dict):
        raise ValueError(f"{path}: metrics must be an object")
    require_exact_keys(f"{path}: metrics", metrics, METRIC_KEYS)
    for name in METRIC_KEYS - {"toolCallsByName"}:
        require_nonnegative_or_null(f"{path}: metrics.{name}", metrics[name])
    by_name = metrics["toolCallsByName"]
    if not isinstance(by_name, dict) or any(
        not isinstance(name, str) or not isinstance(count, int) or count < 0
        for name, count in by_name.items()
    ):
        raise ValueError(f"{path}: toolCallsByName must map strings to nonnegative integers")
    if metrics["toolCalls"] is not None and sum(by_name.values()) != metrics["toolCalls"]:
        raise ValueError(f"{path}: tool call total mismatch")

    acceptance = result["acceptance"]
    if not isinstance(acceptance, dict):
        raise ValueError(f"{path}: acceptance must be an object")
    require_exact_keys(f"{path}: acceptance", acceptance, ACCEPTANCE_KEYS)
    commands = acceptance["commands"]
    if not isinstance(commands, list) or not commands:
        raise ValueError(f"{path}: acceptance.commands must be a nonempty list")
    expected_commands = task.get("acceptanceCommands")
    if [entry.get("command") for entry in commands] != expected_commands:
        raise ValueError(f"{path}: acceptance commands/order differ from task")
    for entry in commands:
        if set(entry) != {"command", "exitCode"}:
            raise ValueError(f"{path}: malformed acceptance command result")
        code = entry["exitCode"]
        if code is not None and (not isinstance(code, int) or isinstance(code, bool) or code < 0):
            raise ValueError(f"{path}: command exitCode must be nonnegative or null")

    changed = result["changedPaths"]
    unrelated = result["unrelatedPaths"]
    if not isinstance(changed, list) or changed != sorted(set(changed)):
        raise ValueError(f"{path}: changedPaths must be sorted and unique")
    if not isinstance(unrelated, list) or unrelated != sorted(set(unrelated)):
        raise ValueError(f"{path}: unrelatedPaths must be sorted and unique")
    expected_paths = sorted(task.get("expectedChangedPaths", []))
    if result["verdict"] == "passed" and changed != expected_paths:
        raise ValueError(f"{path}: passing changed paths differ from expected paths")

    passed_facts = all(
        acceptance[name]
        for name in (
            "structurallyValid",
            "compilerValid",
            "functionallyCorrect",
            "diffChecked",
            "isolationPreserved",
        )
    )
    passed_commands = all(entry["exitCode"] == 0 for entry in commands)
    should_pass = passed_facts and passed_commands and not unrelated
    if (result["verdict"] == "passed") != should_pass:
        raise ValueError(f"{path}: verdict is inconsistent with measured acceptance")
    if result["verdict"] not in {"passed", "failed"}:
        raise ValueError(f"{path}: invalid verdict")
    if not isinstance(result["unmeasured"], list) or not all(
        isinstance(item, str) for item in result["unmeasured"]
    ):
        raise ValueError(f"{path}: unmeasured must be a string list")
    return result
