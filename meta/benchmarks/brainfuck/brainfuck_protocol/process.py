"""Bounded process execution and output verification."""

from __future__ import annotations

import pathlib
import shlex
import subprocess
import time
from typing import Sequence

from brainfuck_protocol.files import sha256_file

def checked_output(command: Sequence[str], cwd: pathlib.Path) -> str:
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def run_small(
    command: Sequence[str], cwd: pathlib.Path, stdin: bytes = b"", timeout: float = 30
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        cwd=cwd,
        input=stdin,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )


def assert_success(
    name: str, result: subprocess.CompletedProcess[bytes], expected: bytes
) -> None:
    if result.returncode != 0 or result.stdout != expected or result.stderr:
        raise RuntimeError(
            f"{name} failed: return={result.returncode}, "
            f"stdout={result.stdout!r}, stderr={result.stderr!r}, expected={expected!r}"
        )


def assert_failure(
    name: str, result: subprocess.CompletedProcess[bytes], diagnostic: bytes
) -> None:
    combined = result.stdout + result.stderr
    if result.returncode == 0 or diagnostic not in combined:
        raise RuntimeError(
            f"{name} did not fail as required: return={result.returncode}, "
            f"stdout={result.stdout!r}, stderr={result.stderr!r}"
        )


def interpreter_command(
    binary: pathlib.Path,
    main: pathlib.Path,
    program: pathlib.Path,
    fold_runs: bool = False,
) -> list[str]:
    command = [str(binary), "run", str(main), "--", str(program)]
    if fold_runs:
        command.append("--fold-runs")
    return command


def run_to_file(
    command: Sequence[str],
    cwd: pathlib.Path,
    output_path: pathlib.Path,
    timeout: float,
) -> tuple[float, bytes]:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    started = time.perf_counter()
    with output_path.open("wb") as output:
        result = subprocess.run(
            command,
            cwd=cwd,
            stdin=subprocess.DEVNULL,
            stdout=output,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    elapsed = time.perf_counter() - started
    if result.returncode != 0:
        raise RuntimeError(
            f"command failed with {result.returncode}: {shlex.join(command)}\n"
            f"stderr: {result.stderr.decode(errors='replace')}"
        )
    if result.stderr:
        raise RuntimeError(
            f"successful command wrote stderr: {result.stderr.decode(errors='replace')}"
        )
    return elapsed, result.stderr


def verify_output(
    path: pathlib.Path, expected_length: int, expected_sha256: str, label: str
) -> None:
    length = path.stat().st_size
    digest = sha256_file(path)
    if length != expected_length or digest != expected_sha256:
        raise RuntimeError(
            f"{label} output mismatch: length={length}, SHA-256={digest}; "
            f"expected length={expected_length}, SHA-256={expected_sha256}"
        )
