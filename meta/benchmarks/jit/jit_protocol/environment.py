"""Measured toolchain and host environment facts."""

import subprocess
from pathlib import Path

def command_version(command: list[str]) -> str:
    completed = subprocess.run(
        command, check=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
    )
    return completed.stdout.strip()


def git_output(root: Path, arguments: list[str]) -> str:
    completed = subprocess.run(
        ["git", *arguments], cwd=root, check=True, stdout=subprocess.PIPE, text=True
    )
    return completed.stdout.strip()


def cpu_model() -> str:
    try:
        for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return "not measured"


def memory_kib() -> int | None:
    try:
        for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemTotal:"):
                return int(line.split()[1])
    except (OSError, ValueError):
        pass
    return None
