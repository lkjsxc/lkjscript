#!/usr/bin/env python3
"""Black-box acceptance for the checked lkjedit package and generic runner."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
from pathlib import Path
import select
import signal
import stat
import struct
import subprocess
import sys
import tempfile
import termios
import time
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BINARY = ROOT / "target" / "release" / "lkjedit"
APPLICATION = Path(
    os.environ.get(
        "LKJEDIT_TEST_ARTIFACT",
        ROOT / "applications" / "lkjedit" / "lkjedit.lkja",
    )
)
MOUSE_ENABLE = (b"\x1b[?1000h", b"\x1b[?1002h", b"\x1b[?1006h")
CLEANUP = (
    b"\x1b[?1006l",
    b"\x1b[?1002l",
    b"\x1b[?1000l",
    b"\x1b[?1004l",
    b"\x1b[?25h",
    b"\x1b[?2004l",
    b"\x1b[?1049l",
)


def key(character: str, *, control: bool = False, alt: bool = False) -> dict[str, Any]:
    return event(
        "key",
        {
            "code": {"character": ord(character)},
            "control": control,
            "alt": alt,
            "shift": False,
            "repeat": False,
        },
    )


def special(code: str, *, control: bool = False) -> dict[str, Any]:
    return event(
        "key",
        {
            "code": code,
            "control": control,
            "alt": False,
            "shift": False,
            "repeat": False,
        },
    )


def event(kind: str, data: Any | None = None) -> dict[str, Any]:
    value: dict[str, Any] = {"kind": "event", "data": {"kind": kind}}
    if data is not None:
        value["data"]["data"] = data
    return value


def outcome(
    job_id: int,
    outcome_class: str,
    message: str,
    *,
    content: str = "",
    token: str = "",
) -> dict[str, Any]:
    return {
        "kind": "outcome",
        "data": {
            "job_id": job_id,
            "class": outcome_class,
            "message": message,
            "content": content,
            "token": token,
        },
    }


def command(value: str) -> list[dict[str, Any]]:
    return [key(":"), *(key(character) for character in value), special("enter")]


def paste(value: str) -> dict[str, Any]:
    return event("paste", [ord(character) for character in value])


def mouse(kind: str, row: int, column: int, button: str = "primary") -> dict[str, Any]:
    return event(
        "mouse",
        {
            "button": button,
            "kind": kind,
            "row": row,
            "column": column,
            "control": False,
            "alt": False,
            "shift": False,
        },
    )


def run_headless(binary: Path, transitions: list[dict[str, Any]]) -> dict[str, Any]:
    request = {
        "version": 4,
        "rows": 40,
        "columns": 120,
        "transitions": transitions,
    }
    completed = subprocess.run(
        [str(binary), "headless", "--artifact", str(APPLICATION)],
        input=json.dumps(request, separators=(",", ":")).encode(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=180,
    )
    require(completed.returncode == 0, completed.stderr.decode(errors="replace"))
    envelope = json.loads(completed.stdout)
    require(envelope["version"] == 1, "headless envelope version differs")
    result = envelope["result"]
    require(result["version"] == 4, "headless replay version differs")
    return result


def frame_text(receipt: dict[str, Any]) -> str:
    return "".join(chr(value) for value in receipt["final_frame"]["scalars"])


def frame_rows(receipt: dict[str, Any]) -> list[str]:
    """Return logical rows with renderer padding removed, preserving empty rows."""
    return [row.rstrip() for row in frame_text(receipt).splitlines()]


def action_kinds(receipt: dict[str, Any]) -> list[str]:
    trace = receipt["action_trace"]
    require(
        [item["job_id"] for item in trace] == list(range(1, len(trace) + 1)),
        "job identities are not monotonic and nonreused",
    )
    for item in trace:
        require(len(item["payload_digest"]) == 64, "action digest is not exact")
    return [item["kind"] for item in trace]


def deterministic_editor_case(binary: Path) -> None:
    transitions = [
        key("i"),
        paste("one\ntwo\nthree\nfour\nλe\u0301界\n"),
        special("escape"),
        key("g"),
        key("g"),
        key("2"),
        key("d"),
        key("d"),
        key("u"),
        key("/"),
        key("t"),
        key("h"),
        key("r"),
        key("e"),
        key("e"),
        special("enter"),
        key("n"),
        key("N"),
        *command("q!"),
    ]
    first = run_headless(binary, transitions)
    second = run_headless(binary, transitions)
    require(first["replay_digest"] == second["replay_digest"], "replay changed")
    require(first["final_frame_digest"] == second["final_frame_digest"], "frame changed")
    require(first["exit_transition"] == len(transitions), "editor did not exit exactly")
    require(first["action_count"] == 0, "local editing emitted a host action")
    rows = frame_rows(first)
    require(
        rows[1:6] == ["one", "two", "three", "four", "λe\u0301界"],
        "counted undo did not restore exact logical rows",
    )
    require(first["final_frame"]["cursor_shape"] == "block", "Normal cursor differs")


def command_surface_case(binary: Path) -> None:
    opened = run_headless(
        binary,
        [
            *command("e alpha.txt"),
            outcome(1, "succeeded", "opened", content="alpha\n", token="origin-a"),
            event("close"),
        ],
    )
    require(action_kinds(opened) == ["filesystem_read"], ":e did not request one read")

    written = run_headless(
        binary,
        [
            key("i"),
            key("x"),
            special("escape"),
            *command("w new.txt"),
            outcome(1, "succeeded", "saved", token="origin-new"),
            event("close"),
        ],
    )
    require(action_kinds(written) == ["filesystem_save"], ":w PATH did not request one save")

    for value in ["wq", "x"]:
        exited = run_headless(
            binary,
            [
                event("open", {"path": "a.txt", "directory": False, "project": False}),
                outcome(1, "succeeded", "opened", content="a\n", token="origin-a"),
                key("i"),
                key("x"),
                special("escape"),
                *command(value),
                outcome(2, "succeeded", "saved", token="origin-b"),
            ],
        )
        require(
            action_kinds(exited) == ["filesystem_read", "filesystem_save"],
            f":{value} action route differs",
        )
        require(exited["exit_transition"] is not None, f":{value} did not exit after save")

    buffers = run_headless(binary, [*command("buffers"), event("close")])
    require("Buffers" in frame_rows(buffers)[0], ":buffers did not open an ordinary tab")
    require(
        buffers["final_frame"]["status"] == "ordinary tab opened",
        "dogfood status change is absent",
    )


def buffer_view_and_unicode_case(binary: Path) -> None:
    grapheme = run_headless(
        binary,
        [
            event("open", {"path": "unicode.txt", "directory": False, "project": False}),
            outcome(1, "succeeded", "opened unicode.txt", content="e\u0301界x\n", token="origin-u"),
            key("x"),
            *command("q!"),
        ],
    )
    require(action_kinds(grapheme) == ["filesystem_read"], "Unicode open action differs")
    require(frame_rows(grapheme)[1] == "界x", "Normal x split an extended grapheme cluster")

    protected = run_headless(
        binary,
        [
            key("i"),
            key("x"),
            special("escape"),
            *command("help"),
            mouse("press", 0, 3),
            mouse("release", 0, 3),
            *command("tabclose"),
            event("close"),
        ],
    )
    require(
        frame_rows(protected)[0].startswith("[* [No Name]] [ Help]"),
        "closing the final dirty view removed its tab",
    )
    require(
        protected["final_frame"]["status"] == "No write since last change; use :q! to discard",
        "final dirty view did not expose an application-owned decision",
    )

    shared = run_headless(
        binary,
        [
            key("i"),
            key("x"),
            special("escape"),
            *command("vsplit"),
            *command("tabclose"),
            event("close"),
        ],
    )
    require("│" not in frame_rows(shared)[1], "closing one shared-buffer view did not collapse")
    require(frame_rows(shared)[1] == "x", "remaining view lost shared buffer content")
    require(shared["final_frame"]["status"] == "tab closed", "shared view close was blocked")


def layout_and_mouse_case(binary: Path) -> None:
    vertical = run_headless(binary, [*command("vsplit"), event("close")])
    vertical_rows = frame_rows(vertical)
    require(vertical_rows[1].index("│") == 59, "vertical split geometry differs")
    horizontal = run_headless(binary, [*command("split"), event("close")])
    require(frame_rows(horizontal)[20].startswith("[* [No Name]]"), "horizontal split differs")
    collapsed = run_headless(
        binary,
        [*command("vsplit"), *command("tabclose"), event("close")],
    )
    require("│" not in frame_rows(collapsed)[1], "empty split did not collapse")

    selected = run_headless(
        binary,
        [
            *command("tabnew"),
            *command("tabnew"),
            mouse("press", 0, 14),
            mouse("release", 0, 14),
            event("close"),
        ],
    )
    require(
        frame_rows(selected)[0].startswith("[ [No Name]] [* View] [ View]"),
        "tab click did not use exact rendered title width",
    )

    reordered = run_headless(
        binary,
        [
            *command("tabnew"),
            *command("help"),
            mouse("press", 0, 25),
            mouse("drag", 0, 3),
            mouse("release", 0, 3),
            event("close"),
        ],
    )
    require(
        frame_rows(reordered)[0].startswith("[* Help] [ [No Name]] [ View]"),
        "tab drag did not reach the exact insertion slot",
    )

    cross_tile_base = [*command("help"), *command("vsplit"), *command("tabnew")]
    strip_move = run_headless(
        binary,
        [
            *cross_tile_base,
            mouse("press", 0, 72),
            mouse("drag", 0, 3),
            mouse("release", 0, 3),
            event("close"),
        ],
    )
    strip_row = frame_rows(strip_move)[0]
    require(strip_row.startswith("[* View] [ [No Name]] [ Help]"), "cross-tile strip move differs")
    require(strip_row.endswith("│[* Help]"), "source tile was not repaired after strip move")

    center_move = run_headless(
        binary,
        [
            *cross_tile_base,
            mouse("press", 0, 72),
            mouse("drag", 10, 20),
            mouse("release", 10, 20),
            event("close"),
        ],
    )
    center_row = frame_rows(center_move)[0]
    require("[* View]" in center_row.split("│")[0], "center drop did not join target stack")
    require(center_row.split("│")[1] == "[* Help]", "center drop changed the wrong source tab")

    edge_destinations = {
        "left": (10, 1),
        "right": (10, 56),
        "top": (2, 20),
        "bottom": (36, 20),
    }
    for name, (row, column) in edge_destinations.items():
        dropped = run_headless(
            binary,
            [
                *cross_tile_base,
                mouse("press", 0, 72),
                mouse("drag", row, column),
                mouse("release", row, column),
                event("close"),
            ],
        )
        rows = frame_rows(dropped)
        if name in ("left", "right"):
            require(rows[0].count("│") == 2, f"{name} edge drop did not create a third tile")
            require("[* View]" in rows[0], f"{name} edge drop lost the dragged tab")
        else:
            require(rows[0].count("│") == 1, f"{name} edge drop changed the wrong axis")
            require(rows[20].startswith("[* View]") == (name == "bottom"), f"{name} order differs")

    cancelled = run_headless(
        binary,
        [
            *cross_tile_base,
            mouse("press", 0, 72),
            mouse("drag", 39, 119),
            mouse("release", 39, 119),
            event("close"),
        ],
    )
    require(
        frame_rows(cancelled)[0].startswith("[ [No Name]] [* Help]")
        and frame_rows(cancelled)[0].endswith("│[ Help] [* View]"),
        "invalid drop did not restore the canonical layout",
    )

    for destination, expected in ((75, 75), (1, 8), (119, 111)):
        resized = run_headless(
            binary,
            [
                *command("vsplit"),
                mouse("press", 10, 59),
                mouse("drag", 10, destination),
                mouse("release", 10, destination),
                event("close"),
            ],
        )
        require(frame_rows(resized)[1].index("│") == expected, "splitter clamp differs")

    resize_during_drag = run_headless(
        binary,
        [
            *command("tabnew"),
            *command("help"),
            mouse("press", 0, 25),
            mouse("drag", 0, 3),
            event("resize", {"rows": 17, "columns": 61}),
            mouse("release", 0, 3),
            event("close"),
        ],
    )
    require(frame_rows(resize_during_drag)[0].startswith("[* Help]"), "resize lost drag identity")
    require(resize_during_drag["final_frame"]["rows"] == 17, "resize rows differ")
    require(resize_during_drag["final_frame"]["columns"] == 61, "resize columns differ")


def explorer_and_search_case(binary: Path) -> None:
    explorer = run_headless(
        binary,
        [
            event("open", {"path": "", "directory": True, "project": False}),
            outcome(1, "succeeded", "listed root", content="D nested\nF note.txt\n"),
            mouse("press", 2, 2),
            mouse("release", 2, 2),
            special("enter"),
            outcome(2, "succeeded", "opened note.txt", content="hello\n", token="origin-1"),
            *command("q"),
        ],
    )
    require(
        action_kinds(explorer) == ["filesystem_list", "filesystem_read"],
        "explorer did not route list and open through typed actions",
    )
    require("hello" in frame_rows(explorer), "explorer-opened content differs")

    tiled_explorer = run_headless(
        binary,
        [
            *command("split"),
            *command("explore ."),
            outcome(1, "succeeded", "listed root", content="D nested\nF note.txt\n"),
            mouse("press", 22, 2),
            mouse("release", 22, 2),
            special("enter"),
            outcome(2, "succeeded", "opened note.txt", content="tiled hello\n", token="origin-3"),
            event("close"),
        ],
    )
    require(
        action_kinds(tiled_explorer) == ["filesystem_list", "filesystem_read"],
        "tile-local explorer mouse routing differs",
    )
    require("tiled hello" in frame_rows(tiled_explorer), "tiled explorer opened the wrong row")

    search = run_headless(
        binary,
        [
            *command("search needle"),
            key("j"),
            mouse("scroll_down", 2, 2, "none"),
            *command("tabmoveleft"),
            outcome(1, "succeeded", "search complete", content="a.txt\nb.txt\n"),
            special("down"),
            special("enter"),
            outcome(2, "succeeded", "opened b.txt", content="needle\n", token="origin-2"),
            *command("q"),
        ],
    )
    require(
        action_kinds(search) == ["filesystem_search", "filesystem_read"],
        "root search did not remain typed",
    )
    require(search["changed_count"] >= 8, "local input stalled during search")

    abandoned = run_headless(
        binary,
        [
            *command("search abandoned"),
            *command("tabclose"),
            outcome(1, "succeeded", "late search", content="late.txt\n"),
            event("close"),
        ],
    )
    require(action_kinds(abandoned) == ["filesystem_search"], "abandoned search rerouted")


def save_authority_case(binary: Path) -> None:
    conflict = run_headless(
        binary,
        [
            event("open", {"path": "note.txt", "directory": False, "project": False}),
            outcome(1, "succeeded", "opened note.txt", content="base\r\n", token="origin-base"),
            key("i"),
            key("L"),
            special("escape"),
            *command("w"),
            outcome(2, "conflict", "external change", token="origin-current"),
            *command("overwrite"),
            outcome(3, "succeeded", "saved note.txt", token="origin-next"),
            *command("q"),
        ],
    )
    require(
        action_kinds(conflict)
        == ["filesystem_read", "filesystem_save", "filesystem_save"],
        "explicit overwrite action sequence differs",
    )
    require(conflict["final_frame"]["status"] == "NORMAL", "save did not return to Normal")

    unknown = run_headless(
        binary,
        [
            event("open", {"path": "note.txt", "directory": False, "project": False}),
            outcome(1, "succeeded", "opened note.txt", content="base\n", token="origin-base"),
            key("i"),
            key("?"),
            special("escape"),
            *command("w"),
            outcome(2, "unknown", "save visibility unknown", token="reconcile-1"),
            *command("reconcile"),
            outcome(3, "succeeded", "save is present", token="origin-present"),
            *command("q"),
        ],
    )
    require(
        action_kinds(unknown)
        == ["filesystem_read", "filesystem_save", "filesystem_reconcile"],
        "unknown save did not require independent reconciliation",
    )

    for outcome_class, message, token in (
        ("unchanged", "save is absent", "origin-current"),
        ("conflict", "save conflicts with current bytes", ""),
    ):
        unresolved_prefix = [
            event("open", {"path": "note.txt", "directory": False, "project": False}),
            outcome(1, "succeeded", "opened note.txt", content="base\n", token="origin-base"),
            key("i"),
            key("?"),
            special("escape"),
            *command("w"),
            outcome(2, "unknown", "save visibility unknown", token="reconcile-2"),
            *command("reconcile"),
            outcome(3, outcome_class, message, token=token),
        ]
        blocked = run_headless(binary, [*unresolved_prefix, *command("q")])
        require(
            blocked.get("exit_transition") is None,
            f"{outcome_class} reconciliation allowed dirty exit",
        )
        require(
            blocked["final_frame"]["status"] == "No write since last change; use :q!",
            f"{outcome_class} reconciliation cleared dirty state",
        )
        retried = run_headless(
            binary,
            [
                *unresolved_prefix,
                *command("w"),
                outcome(4, "succeeded", "new explicit save succeeded", token="origin-next"),
                *command("q"),
            ],
        )
        require(
            action_kinds(retried)
            == [
                "filesystem_read",
                "filesystem_save",
                "filesystem_reconcile",
                "filesystem_save",
            ],
            f"{outcome_class} reconciliation did not require a fresh explicit save",
        )


def semantic_project_case(binary: Path) -> None:
    receipt = run_headless(
        binary,
        [
            event("open", {"path": "", "directory": False, "project": True}),
            key("j"),
            mouse("scroll_down", 2, 2, "none"),
            outcome(1, "succeeded", "orientation revision 171", content="workspace revision 171"),
            key("p", alt=True),
            key("j"),
            outcome(2, "succeeded", "proposal generated", content="base-bound proposal"),
            key("v", alt=True),
            key("j"),
            outcome(3, "succeeded", "proposal validated", content="no publication"),
            key("x", alt=True),
            key("j"),
            outcome(4, "succeeded", "proposal applied", content="revision 174"),
            key("h", alt=True),
            key("j"),
            outcome(5, "succeeded", "history opened", content="revision 174"),
            key("k", alt=True),
            key("j"),
            mouse("scroll_down", 2, 2, "none"),
            outcome(6, "succeeded", "target tests passed", content="12 passed, 0 failed"),
            event("close"),
        ],
    )
    require(
        action_kinds(receipt)
        == [
            "project_orient",
            "project_proposal",
            "project_validate",
            "project_apply",
            "project_history",
            "project_target_test",
        ],
        "semantic tabs did not use typed project actions",
    )
    require(receipt["changed_count"] >= 15, "project work blocked local navigation")


def spawn_terminal(binary: Path, arguments: list[str]) -> tuple[int, int, Any, list[Any]]:
    master, slave = os.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    before = termios.tcgetattr(slave)
    process = subprocess.Popen(
        [str(binary), *arguments],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        close_fds=True,
    )
    return master, slave, process, before


def terminal_case(binary: Path, mode: str) -> None:
    with tempfile.TemporaryDirectory(prefix="lkjedit-acceptance-") as temporary:
        root = Path(temporary)
        note = root / "note.txt"
        note.write_bytes(b"alpha\r\nbeta")
        note.chmod(0o640)
        master, slave, process, before = spawn_terminal(binary, ["--root", str(root), "note.txt"])
        output = bytearray()
        try:
            read_until(master, output, b"alpha", process, 15.0)
            if mode == "edit":
                start = len(output)
                os.write(master, b"iX")
                read_until(master, output, b"INSERT", process, 15.0, start=start)

                start = len(output)
                os.write(master, b"\x1b")
                read_until(master, output, b"NORMAL", process, 15.0, start=start)

                start = len(output)
                os.write(master, b":w\r")
                read_until(master, output, b"saved", process, 15.0, start=start)
                require(note.read_bytes() == b"Xalpha\r\nbeta", "edit/save bytes differ")

                os.write(master, b":q\r")
            elif mode == "mouse":
                start = len(output)
                os.write(master, b":tabnew\r")
                read_until(master, output, b"tab opened", process, 15.0, start=start)

                start = len(output)
                os.write(master, b":vsplit\r")
                read_until(master, output, b"split", process, 15.0, start=start)

                start = len(output)
                os.write(master, b"\x1b[<0;3;1M\x1b[<32;22;1M\x1b[<0;22;1m")
                read_until(master, output, b"tab drop", process, 15.0, start=start)
                os.write(master, b":q!\r")
            elif mode == "signal":
                process.send_signal(signal.SIGTERM)
            else:
                raise AssertionError(f"unknown terminal mode {mode}")
            drain_until_exit(master, output, process, 15.0, mode=mode)
            require(process.returncode == 0, f"{mode} exit {process.returncode}: {output!r}")
            for sequence in MOUSE_ENABLE:
                require(sequence in output, f"{mode} omitted mouse acquisition {sequence!r}")
            for sequence in CLEANUP:
                require(sequence in output, f"{mode} omitted cleanup {sequence!r}")
            require(termios.tcgetattr(slave) == before, f"{mode} did not restore terminal")
            if mode == "edit":
                require(note.read_bytes() == b"Xalpha\r\nbeta", "edit/save bytes differ")
                require(stat.S_IMODE(note.stat().st_mode) == 0o640, "save changed permission mode")
        finally:
            terminate(process)
            os.close(master)
            os.close(slave)


def terminal_eof_case(binary: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="lkjedit-eof-") as temporary:
        master, slave, process, _ = spawn_terminal(binary, [temporary])
        output = bytearray()
        master_open = True
        try:
            read_until(master, output, b"\x1b[?1049h", process, 15.0)
            os.close(master)
            master_open = False
            process.wait(timeout=15)
            require(process.returncode == 3, f"EOF exit was {process.returncode}")
        finally:
            terminate(process)
            if master_open:
                os.close(master)
            os.close(slave)


def non_terminal_and_grammar_case(binary: Path) -> None:
    completed = subprocess.run(
        [str(binary)],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=15,
    )
    require(completed.returncode == 3, "non-terminal launch returned wrong status")
    envelope = json.loads(completed.stdout)
    require(envelope["error"]["code"] == "terminal_unavailable", "wrong non-terminal error")

    rejected = subprocess.run(
        [str(binary), "--artifact", str(APPLICATION)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=15,
    )
    require(rejected.returncode == 2, "ordinary --artifact was accepted")


def read_until(
    master: int,
    output: bytearray,
    needle: bytes,
    process: subprocess.Popen[bytes],
    timeout: float,
    *,
    start: int = 0,
) -> None:
    deadline = time.monotonic() + timeout
    while needle not in output[start:]:
        require(process.poll() is None, f"process exited before {needle!r}: {output!r}")
        require(time.monotonic() < deadline, f"timed out waiting for {needle!r}")
        readable, _, _ = select.select([master], [], [], 0.1)
        if readable:
            output.extend(os.read(master, 65_536))


def drain_until_exit(
    master: int,
    output: bytearray,
    process: subprocess.Popen[bytes],
    timeout: float,
    *,
    mode: str,
) -> None:
    deadline = time.monotonic() + timeout
    while process.poll() is None:
        require(time.monotonic() < deadline, f"{mode} terminal process did not exit")
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


def terminate(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=2)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    parser.add_argument("--headless-only", action="store_true")
    arguments = parser.parse_args()
    require(arguments.binary.is_file(), f"binary is absent: {arguments.binary}")
    require(APPLICATION.is_file(), f"application is absent: {APPLICATION}")

    deterministic_editor_case(arguments.binary)
    command_surface_case(arguments.binary)
    buffer_view_and_unicode_case(arguments.binary)
    layout_and_mouse_case(arguments.binary)
    explorer_and_search_case(arguments.binary)
    save_authority_case(arguments.binary)
    semantic_project_case(arguments.binary)
    if not arguments.headless_only:
        non_terminal_and_grammar_case(arguments.binary)
        terminal_case(arguments.binary, "edit")
        terminal_case(arguments.binary, "mouse")
        terminal_case(arguments.binary, "signal")
        terminal_eof_case(arguments.binary)
    scope = "headless workflow groups"
    if not arguments.headless_only:
        scope = "headless, grammar, filesystem, and 4 PTY workflow groups"
    print(
        "lkjedit acceptance passed: editor, Unicode, buffer/view, layout, mouse, explorer, "
        f"search, save/reconcile, semantic-job, and {scope}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(f"lkjedit acceptance failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
