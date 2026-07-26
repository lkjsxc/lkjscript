"""Schema identities and structural checks for AI-authorability evidence."""

from __future__ import annotations

import hashlib
import json
import pathlib
from typing import Any

TASK_SCHEMA = "lkjscript.ai-authorability-task/v1"
RESULT_SCHEMA = "lkjscript.ai-authorability-result/v1"
TOP_LEVEL_KEYS = {
    "schema", "taskId", "taskSha256", "baseCommit", "interface", "model",
    "harness", "run", "metrics", "acceptance", "changedPaths",
    "unrelatedPaths", "unmeasured", "verdict",
}
METRIC_KEYS = {
    "wallMilliseconds", "inputTokens", "cachedInputTokens", "outputTokens",
    "reasoningTokens", "toolCalls", "toolCallsByName", "failedMutations",
    "compilerInvocations", "repairIterations",
}
ACCEPTANCE_KEYS = {
    "commands", "structurallyValid", "compilerValid", "functionallyCorrect",
    "diffChecked", "isolationPreserved",
}

def load_json(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise ValueError(f"{path}: root must be an object")
    return value


def canonical_sha256(value: dict[str, Any]) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def require_exact_keys(label: str, value: dict[str, Any], expected: set[str]) -> None:
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        unknown = sorted(actual - expected)
        raise ValueError(f"{label}: missing={missing}, unknown={unknown}")


def require_nonnegative_or_null(label: str, value: Any) -> None:
    if value is not None and (not isinstance(value, int) or isinstance(value, bool) or value < 0):
        raise ValueError(f"{label}: expected a nonnegative integer or null")


def task_path(result_path: pathlib.Path, task_id: str) -> pathlib.Path:
    root = result_path.parent.parent
    return root / "tasks" / f"{task_id}.json"
