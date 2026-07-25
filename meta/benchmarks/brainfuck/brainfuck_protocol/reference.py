"""Build the pinned independent C oracle."""

import os
import pathlib
import shlex
import subprocess

from brainfuck_protocol.constants import (
    LICENSE_PATH, LICENSE_SHA256, REFERENCE_PATH, REFERENCE_SHA256, UPSTREAM_ROOT,
)
from brainfuck_protocol.files import fetch_verified, sha256_file
from brainfuck_protocol.process import checked_output

def build_reference(
    root: pathlib.Path, work: pathlib.Path
) -> tuple[pathlib.Path, str]:
    source = root / REFERENCE_PATH
    if sha256_file(source) != REFERENCE_SHA256:
        raise RuntimeError(f"reference source SHA-256 does not match {REFERENCE_SHA256}")
    license_file = work / "reference" / "LICENSE.md"
    fetch_verified(f"{UPSTREAM_ROOT}/{LICENSE_PATH}", license_file, LICENSE_SHA256)
    binary = work / "reference" / "brainfuck"
    binary.parent.mkdir(parents=True, exist_ok=True)
    compiler = os.environ.get("CC", "cc")
    command = [
        compiler,
        "-O3",
        "-std=c11",
        "-Wall",
        "-Wextra",
        "-Werror",
        str(source),
        "-o",
        str(binary),
    ]
    subprocess.run(command, cwd=root, check=True)
    version = checked_output([compiler, "--version"], root).splitlines()[0]
    return binary, f"{shlex.join(command)}; {version}"
