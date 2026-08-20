#!/usr/bin/env python3
"""Black-box headless and pseudo-terminal acceptance for the generic lkjstudio runner."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
from pathlib import Path
import select
import signal
import struct
import subprocess
import sys
import termios
import time


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BINARY = ROOT / "target" / "debug" / "lkjstudio"
APPLICATION = ROOT / "applications" / "lkjstudio" / "lkjstudio.lkja"
CLEANUP = (b"\x1b[?25h", b"\x1b[?2004l", b"\x1b[?1049l")


def headless(binary: Path) -> None:
    request = {
        "version": 3,
        "rows": 24,
        "columns": 80,
        "events": [
            {
                "kind": "key",
                "data": {
                    "code": {"character": ord("o")},
                    "control": False,
                    "alt": True,
                    "shift": False,
                    "repeat": False,
                },
            },
            {
                "kind": "key",
                "data": {
                    "code": {"character": ord("A")},
                    "control": False,
                    "alt": False,
                    "shift": False,
                    "repeat": False,
                },
            },
            {"kind": "paste", "data": [ord("λ"), ord("!")]},
            {"kind": "resize", "data": {"rows": 7, "columns": 19}},
            {"kind": "close"},
        ],
        "outcomes": [
            {
                "class": "succeeded",
                "message": "orientation accepted",
                "content": "",
                "token": "",
            }
        ],
    }
    encoded = json.dumps(request, separators=(",", ":")).encode()
    receipts = []
    for _ in range(2):
        completed = subprocess.run(
            [str(binary), "headless", "--artifact", str(APPLICATION)],
            input=encoded,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=20,
        )
        require(completed.returncode == 0, completed.stderr.decode(errors="replace"))
        receipts.append(json.loads(completed.stdout)["result"])
    require(
        receipts[0]["replay_digest"] == receipts[1]["replay_digest"],
        "headless replay digest changed",
    )
    require(receipts[0]["event_count"] == 5, "headless event count differs")
    require(receipts[0]["action_count"] == 1, "headless action count differs")
    require(receipts[0]["exit_event"] == 5, "headless exit event differs")
    require(
        receipts[0]["final_frame"]["scalars"][-3:] == [ord("A"), ord("λ"), ord("!")],
        "headless edited content differs",
    )


def terminal_case(binary: Path, mode: str) -> None:
    master, slave = os.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    before = termios.tcgetattr(slave)
    process = subprocess.Popen(
        [
            str(binary),
            "--artifact",
            str(APPLICATION),
            "--project",
            str(APPLICATION.parent),
        ],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        close_fds=True,
    )
    output = bytearray()
    try:
        read_until(master, output, b"\x1b[?1049h", process, 10.0)
        if mode == "normal":
            os.write(master, b"A\x11")  # Insert A, then semantic Ctrl-Q exit.
        elif mode == "project":
            os.write(master, b"\x1bo")  # Semantic Alt-O project orientation.
            read_until(master, output, b"workspace", process, 10.0)
            os.write(master, b"\x11")
        elif mode == "signal":
            process.send_signal(signal.SIGTERM)
        else:
            raise AssertionError(f"unknown terminal mode {mode}")
        drain_until_exit(master, output, process, 10.0)
        require(process.returncode == 0, f"{mode} exit was {process.returncode}: {output!r}")
        for sequence in CLEANUP:
            require(sequence in output, f"{mode} omitted cleanup sequence {sequence!r}")
        after = termios.tcgetattr(slave)
        require(after == before, f"{mode} did not restore exact terminal attributes")
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=2)
        os.close(master)
        os.close(slave)


def terminal_eof_case(binary: Path) -> None:
    master, slave = os.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    process = subprocess.Popen(
        [
            str(binary),
            "--artifact",
            str(APPLICATION),
            "--project",
            str(APPLICATION.parent),
        ],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        close_fds=True,
    )
    output = bytearray()
    master_open = True
    try:
        read_until(master, output, b"\x1b[?1049h", process, 10.0)
        os.close(master)
        master_open = False
        process.wait(timeout=10)
        require(
            process.returncode == 3,
            f"EOF exit was {process.returncode}; closed output must be classified as unavailable",
        )
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=2)
        if master_open:
            os.close(master)
        os.close(slave)


def non_terminal_rejection(binary: Path) -> None:
    completed = subprocess.run(
        [
            str(binary),
            "--artifact",
            str(APPLICATION),
            "--project",
            str(APPLICATION.parent),
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=10,
    )
    require(completed.returncode == 3, "non-terminal run returned the wrong exit code")
    envelope = json.loads(completed.stdout)
    require(
        envelope["error"]["code"] == "terminal_unavailable",
        "non-terminal run returned the wrong typed error",
    )


def read_until(
    master: int,
    output: bytearray,
    needle: bytes,
    process: subprocess.Popen[bytes],
    timeout: float,
) -> None:
    deadline = time.monotonic() + timeout
    while needle not in output:
        require(process.poll() is None, f"process exited before terminal acquisition: {output!r}")
        require(time.monotonic() < deadline, f"timed out waiting for {needle!r}: {output!r}")
        readable, _, _ = select.select([master], [], [], 0.1)
        if readable:
            output.extend(os.read(master, 65_536))


def drain_until_exit(
    master: int,
    output: bytearray,
    process: subprocess.Popen[bytes],
    timeout: float,
) -> None:
    deadline = time.monotonic() + timeout
    while process.poll() is None:
        require(time.monotonic() < deadline, f"terminal process did not exit: {output!r}")
        readable, _, _ = select.select([master], [], [], 0.1)
        if readable:
            try:
                output.extend(os.read(master, 65_536))
            except OSError:
                break
    process.wait(timeout=2)
    while True:
        readable, _, _ = select.select([master], [], [], 0)
        if not readable:
            break
        try:
            chunk = os.read(master, 65_536)
        except OSError:
            break
        if not chunk:
            break
        output.extend(chunk)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    arguments = parser.parse_args()
    require(arguments.binary.is_file(), f"binary is absent: {arguments.binary}")
    require(APPLICATION.is_file(), f"application is absent: {APPLICATION}")
    headless(arguments.binary)
    non_terminal_rejection(arguments.binary)
    terminal_case(arguments.binary, "normal")
    terminal_case(arguments.binary, "project")
    terminal_case(arguments.binary, "signal")
    terminal_eof_case(arguments.binary)
    print(
        "lkjstudio acceptance passed: headless, non-terminal, normal PTY, "
        "project-action PTY, signal PTY, EOF PTY"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(f"lkjstudio acceptance failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
