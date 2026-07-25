"""Focused Brainfuck success and failure smoke coverage."""

import pathlib

from brainfuck_protocol.process import (
    assert_failure, assert_success, interpreter_command, run_small,
)

def run_smokes(
    root: pathlib.Path, binary: pathlib.Path, main: pathlib.Path, work: pathlib.Path
) -> None:
    fixtures = root / "meta/benchmarks/brainfuck/fixtures"
    successful = [
        ("comments", "comments.bf", b"", b"A"),
        ("hello", "hello.bf", b"", b"Hello World!\n"),
        ("nested loops", "nested.bf", b"", bytes((17,))),
        ("wrapping cells", "wrapping.bf", b"", b"\xff\x00"),
        ("input byte", "echo.bf", b"Z", b"Z"),
        ("input EOF clears nonzero cell", "eof.bf", b"", b"\x00"),
    ]
    for name, fixture, stdin, expected in successful:
        for fold_runs in (False, True):
            variant = "run-folded" if fold_runs else "direct"
            result = run_small(
                interpreter_command(binary, main, fixtures / fixture, fold_runs),
                root,
                stdin,
            )
            assert_success(f"{name} ({variant})", result, expected)

    failing = [
        ("left underflow", "left-underflow.bf", b"tape pointer underflow"),
        ("unmatched open", "unmatched-open.bf", b"unmatched ["),
        ("unmatched close", "unmatched-close.bf", b"unmatched ]"),
    ]
    for name, fixture, diagnostic in failing:
        for fold_runs in (False, True):
            variant = "run-folded" if fold_runs else "direct"
            result = run_small(
                interpreter_command(binary, main, fixtures / fixture, fold_runs), root
            )
            assert_failure(f"{name} ({variant})", result, diagnostic)

    generated = work / "smoke"
    generated.mkdir(parents=True, exist_ok=True)
    right_overflow = generated / "right-overflow.bf"
    right_overflow.write_bytes(b">" * 30000)
    for fold_runs in (False, True):
        variant = "run-folded" if fold_runs else "direct"
        assert_failure(
            f"right overflow ({variant})",
            run_small(interpreter_command(binary, main, right_overflow, fold_runs), root),
            b"tape pointer overflow",
        )

    oversized = generated / "oversized.bf"
    oversized.write_bytes(b"x" * 250001)
    assert_failure(
        "source size limit",
        run_small(interpreter_command(binary, main, oversized), root),
        b"source exceeds 250000-byte buffer limit",
    )

    repeat = generated / "repeat.bf"
    repeat.write_bytes(b"+.")
    for fold_runs in (False, True):
        variant = "run-folded" if fold_runs else "direct"
        first = run_small(interpreter_command(binary, main, repeat, fold_runs), root)
        second = run_small(interpreter_command(binary, main, repeat, fold_runs), root)
        assert_success(f"first zeroed tape run ({variant})", first, b"\x01")
        assert_success(f"repeated zeroed tape run ({variant})", second, b"\x01")

    assert_failure(
        "missing path",
        run_small([str(binary), "run", str(main), "--"], root),
        b"usage: brainfuck PROGRAM.bf",
    )
    missing = generated / "does-not-exist.bf"
    assert_failure(
        "unreadable source",
        run_small(interpreter_command(binary, main, missing), root),
        b"sys-open-read",
    )
    assert_failure(
        "unknown option",
        run_small(
            [
                str(binary),
                "run",
                str(main),
                "--",
                str(fixtures / "hello.bf"),
                "--unknown",
            ],
            root,
        ),
        b"usage: brainfuck PROGRAM.bf [--fold-runs]",
    )
    print("smoke: direct and run-folded correctness and failure checks passed")
