"""Repository paths and content-addressed artifact facts."""

from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Any

def repository_root() -> Path:
    return Path(__file__).resolve().parents[4]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def artifact(path: Path, root: Path) -> dict[str, Any]:
    return {
        "path": str(path.relative_to(root)) if path.is_relative_to(root) else str(path),
        "size_bytes": path.stat().st_size,
        "sha256": sha256(path),
    }
