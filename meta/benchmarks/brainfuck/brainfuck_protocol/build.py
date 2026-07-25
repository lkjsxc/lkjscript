"""Release interpreter build validation."""

import pathlib
import subprocess

def prepare_release(root: pathlib.Path, no_build: bool) -> pathlib.Path:
    binary = root / "target/release/lkjscript"
    if not no_build:
        subprocess.run(
            ["cargo", "build", "--workspace", "--release", "--locked"],
            cwd=root, check=True,
        )
    if not binary.is_file():
        raise RuntimeError(f"release binary not found: {binary}")
    return binary
