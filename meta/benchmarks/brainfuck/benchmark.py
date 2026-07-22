#!/usr/bin/env python3
"""Correctness and end-to-end benchmark harness for the lkjscript BF interpreter."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import platform
import shlex
import statistics
import subprocess
import sys
import time
import urllib.request
from typing import BinaryIO, Sequence

UPSTREAM_COMMIT = "153924714ae5e569ec39dcf0c0a5b5ae33600cc6"
UPSTREAM_ROOT = f"https://raw.githubusercontent.com/pablojorge/brainfuck/{UPSTREAM_COMMIT}"
INPUT_PATH = "programs/mandelbrot.bf"
INPUT_SHA256 = "f0f048e90855450fb06f2bea21f914f0d24e6b6c15fd050c68176ff794c6229e"
REFERENCE_PATH = "meta/benchmarks/brainfuck/reference.c"
REFERENCE_SHA256 = "af6250f93ef18b35e35788958e6c1feed1a20155011e7208546940661dbedf1d"
OUTPUT_LENGTH = 6240
OUTPUT_SHA256 = "83a0aac65090b3b5e85c22337afac39d8ac17bfd88675f044b33bd55ca0c351b"
LICENSE_PATH = "LICENSE.md"
LICENSE_SHA256 = "68ffa8b51537b1fc1ca38b4ad6bb0c2c7230262d3309d1ef55a3f25de9360d2d"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "validate and measure Brainfuck Mandelbrot interpreted by lkjscript; "
            "reported time is end-to-end process wall time"
        )
    )
    parser.add_argument(
        "--mode", choices=("smoke", "correctness", "benchmark"), default="benchmark"
    )
    parser.add_argument("--warmups", type=int, default=2)
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--diagnostic-timeout", type=float, default=10.0)
    parser.add_argument("--timeout", type=float, default=1800.0)
    parser.add_argument(
        "--fold-runs",
        action="store_true",
        help="measure the optional identical +, -, >, and < run-folding mode",
    )
    parser.add_argument(
        "--no-build", action="store_true", help="reuse an existing release binary"
    )
    args = parser.parse_args()
    if args.warmups < 0 or args.runs < 1:
        parser.error("warmups must be nonnegative and runs must be positive")
    if args.diagnostic_timeout <= 0 or args.timeout <= 0:
        parser.error("timeouts must be positive")
    return args


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def fetch_verified(url: str, destination: pathlib.Path, expected_sha256: str) -> None:
    if destination.exists():
        actual = sha256_file(destination)
        if actual != expected_sha256:
            raise RuntimeError(
                f"cached {destination} has SHA-256 {actual}, expected {expected_sha256}"
            )
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".tmp")
    try:
        with urllib.request.urlopen(url, timeout=60) as response, temporary.open(
            "wb"
        ) as output:
            while block := response.read(1024 * 1024):
                output.write(block)
        actual = sha256_file(temporary)
        if actual != expected_sha256:
            raise RuntimeError(
                f"downloaded {url} has SHA-256 {actual}, expected {expected_sha256}"
            )
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)


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


def first_matching_line(path: pathlib.Path, prefix: str) -> str:
    try:
        for line in path.read_text(errors="replace").splitlines():
            if line.startswith(prefix):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return "unknown"


def workload_hash(source_dir: pathlib.Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(source_dir.glob("*.lkjscript")):
        digest.update(path.name.encode())
        digest.update(b"\x00")
        digest.update(path.read_bytes())
        digest.update(b"\x00")
    return digest.hexdigest()


def machine_metadata(
    root: pathlib.Path, binary: pathlib.Path
) -> dict[str, object]:
    status = checked_output(["git", "status", "--short"], root)
    memory_kib = first_matching_line(pathlib.Path("/proc/meminfo"), "MemTotal")
    cpu = first_matching_line(pathlib.Path("/proc/cpuinfo"), "model name")
    return {
        "repository_commit": checked_output(["git", "rev-parse", "HEAD"], root),
        "tree_state": "clean" if not status else "dirty",
        "git_status_short": status.splitlines(),
        "interpreter_source_sha256": workload_hash(
            root / "src/examples/brainfuck"
        ),
        "release_binary_sha256": sha256_file(binary),
        "harness_sha256": sha256_file(pathlib.Path(__file__).resolve()),
        "reference_source_sha256": sha256_file(root / REFERENCE_PATH),
        "cpu": cpu,
        "ram": memory_kib,
        "operating_system": platform.platform(),
        "kernel": platform.release(),
        "machine": platform.machine(),
        "rustc": checked_output(["rustc", "--version"], root),
        "cargo": checked_output(["cargo", "--version"], root),
        "python": platform.python_version(),
    }


def write_result(work: pathlib.Path, result: dict[str, object]) -> pathlib.Path:
    results = work / "results"
    results.mkdir(parents=True, exist_ok=True)
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    path = results / f"brainfuck-mandelbrot-{stamp}.json"
    encoded = json.dumps(result, indent=2, sort_keys=True) + "\n"
    path.write_text(encoded)
    (results / "latest.json").write_text(encoded)
    return path


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


def main() -> int:
    args = parse_args()
    root = pathlib.Path(__file__).resolve().parents[3]
    work = root / "target/brainfuck-bench"
    binary = root / "target/release/lkjscript"
    source = root / "src/examples/brainfuck/main.lkjscript"
    work.mkdir(parents=True, exist_ok=True)

    if not args.no_build:
        subprocess.run(
            ["cargo", "build", "--workspace", "--release", "--locked"],
            cwd=root,
            check=True,
        )
    if not binary.is_file():
        raise RuntimeError(f"release binary not found: {binary}")

    run_smokes(root, binary, source, work)
    if args.mode == "smoke":
        return 0

    input_file = work / "inputs/mandelbrot.bf"
    fetch_verified(f"{UPSTREAM_ROOT}/{INPUT_PATH}", input_file, INPUT_SHA256)
    reference, reference_build = build_reference(root, work)
    reference_output = work / "oracle-output.bin"
    reference_command = [str(reference), str(input_file)]
    expected_length = OUTPUT_LENGTH
    expected_sha256 = OUTPUT_SHA256
    command = interpreter_command(binary, source, input_file, args.fold_runs)
    metadata = machine_metadata(root, binary)
    oracle: dict[str, object] = {
        "kind": "independent repository C interpreter",
        "source_path": REFERENCE_PATH,
        "source_sha256": REFERENCE_SHA256,
        "build": reference_build,
        "command": shlex.join(reference_command),
        "output_length": expected_length,
        "output_sha256": expected_sha256,
        "output_verified": False,
        "byte_equal": False,
    }
    base_result: dict[str, object] = {
        "status": "running",
        "metric": "end-to-end process wall time (compile + initialize + interpret + output)",
        "mode": "release",
        "release_build_performed": not args.no_build,
        "interpreter_variant": "run-folded" if args.fold_runs else "direct",
        "optional_run_folding": args.fold_runs,
        "command": shlex.join(command),
        "harness_command": shlex.join([sys.executable, *sys.argv]),
        "timeout_seconds": args.timeout,
        "diagnostic": {"status": "not-run"},
        "warmups": args.warmups,
        "measured_runs": args.runs,
        "upstream": {
            "repository": "https://github.com/pablojorge/brainfuck",
            "commit": UPSTREAM_COMMIT,
            "input_path": INPUT_PATH,
            "input_url": f"{UPSTREAM_ROOT}/{INPUT_PATH}",
            "input_sha256": INPUT_SHA256,
            "attribution": "Mandelbrot Brainfuck program by Erik Bosman",
            "license_path": LICENSE_PATH,
            "license_sha256": LICENSE_SHA256,
        },
        "oracle": oracle,
        "environment": metadata,
    }

    try:
        run_to_file(reference_command, root, reference_output, args.timeout)
        verify_output(reference_output, expected_length, expected_sha256, "reference")
    except subprocess.TimeoutExpired:
        base_result["status"] = "oracle-timeout"
        oracle["execution"] = {
            "status": "timed-out",
            "timeout_seconds": args.timeout,
        }
        path = write_result(work, base_result)
        reference_output.unlink(missing_ok=True)
        print(f"oracle timed out; result: {path}")
        return 2
    except RuntimeError as error:
        base_result["status"] = "oracle-failed"
        oracle["execution"] = {"status": "failed", "error": str(error)}
        path = write_result(work, base_result)
        reference_output.unlink(missing_ok=True)
        print(f"oracle failed; result: {path}", file=sys.stderr)
        return 1
    oracle["output_verified"] = True
    print(
        f"oracle: independent C interpreter produced {expected_length} bytes, "
        f"SHA-256 {expected_sha256}"
    )

    diagnostic_output = work / "diagnostic-output.bin"
    try:
        diagnostic_elapsed, _ = run_to_file(
            command, root, diagnostic_output, args.diagnostic_timeout
        )
        verify_output(
            diagnostic_output, expected_length, expected_sha256, "diagnostic"
        )
        base_result["diagnostic"] = {
            "status": "completed",
            "elapsed_seconds": diagnostic_elapsed,
        }
        print(f"diagnostic: completed in {diagnostic_elapsed:.6f} s")
    except subprocess.TimeoutExpired:
        base_result["diagnostic"] = {
            "status": "timed-out-as-bounded",
            "timeout_seconds": args.diagnostic_timeout,
        }
        print(f"diagnostic: timed out at {args.diagnostic_timeout:.3f} s (expected bound)")
    except RuntimeError as error:
        base_result["status"] = "diagnostic-failed"
        base_result["diagnostic"] = {"status": "failed", "error": str(error)}
        path = write_result(work, base_result)
        reference_output.unlink(missing_ok=True)
        print(f"diagnostic failed; result: {path}", file=sys.stderr)
        return 1
    finally:
        diagnostic_output.unlink(missing_ok=True)

    correctness_output = work / "correctness-output.bin"
    try:
        correctness_elapsed, _ = run_to_file(
            command, root, correctness_output, args.timeout
        )
        verify_output(
            correctness_output, expected_length, expected_sha256, "full correctness"
        )
    except subprocess.TimeoutExpired:
        base_result["status"] = "full-run-timeout"
        base_result["correctness"] = {
            "status": "timed-out",
            "timeout_seconds": args.timeout,
        }
        path = write_result(work, base_result)
        correctness_output.unlink(missing_ok=True)
        reference_output.unlink(missing_ok=True)
        print(f"full correctness: did not complete within {args.timeout:.3f} s")
        print(f"result: {path}")
        return 2
    except RuntimeError as error:
        base_result["status"] = "full-correctness-failed"
        base_result["correctness"] = {"status": "failed", "error": str(error)}
        path = write_result(work, base_result)
        correctness_output.unlink(missing_ok=True)
        reference_output.unlink(missing_ok=True)
        print(f"full correctness failed; result: {path}", file=sys.stderr)
        return 1
    finally:
        correctness_output.unlink(missing_ok=True)
    oracle["byte_equal"] = True
    base_result["correctness"] = {
        "status": "passed",
        "elapsed_seconds": correctness_elapsed,
    }
    print(f"full correctness: byte-equal in {correctness_elapsed:.6f} s")

    if args.mode == "correctness":
        base_result["status"] = "correctness-passed"
        path = write_result(work, base_result)
        reference_output.unlink(missing_ok=True)
        print(f"result: {path}")
        return 0

    sample_output = work / "sample-output.bin"
    samples: list[float] = []
    active_sample = "warmup 1"
    try:
        for index in range(args.warmups):
            active_sample = f"warmup {index + 1}"
            elapsed, _ = run_to_file(command, root, sample_output, args.timeout)
            verify_output(sample_output, expected_length, expected_sha256, active_sample)
            print(f"warmup {index + 1}/{args.warmups}: {elapsed:.6f} s")
        for index in range(args.runs):
            active_sample = f"measured {index + 1}"
            elapsed, _ = run_to_file(command, root, sample_output, args.timeout)
            verify_output(sample_output, expected_length, expected_sha256, active_sample)
            samples.append(elapsed)
            print(f"measured {index + 1}/{args.runs}: {elapsed:.6f} s")
    except subprocess.TimeoutExpired:
        base_result["status"] = "measurement-timeout"
        base_result["measurement_failure"] = {
            "status": "timed-out",
            "sample": active_sample,
            "timeout_seconds": args.timeout,
        }
        base_result["samples_seconds"] = samples
        path = write_result(work, base_result)
        print(f"measurement timed out at {args.timeout:.3f} s; result: {path}")
        return 2
    except RuntimeError as error:
        base_result["status"] = "measurement-failed"
        base_result["measurement_failure"] = {
            "status": "failed",
            "sample": active_sample,
            "error": str(error),
        }
        base_result["samples_seconds"] = samples
        path = write_result(work, base_result)
        print(f"measurement failed; result: {path}", file=sys.stderr)
        return 1
    finally:
        sample_output.unlink(missing_ok=True)
        reference_output.unlink(missing_ok=True)

    median = statistics.median(samples)
    mad = statistics.median(abs(sample - median) for sample in samples)
    stats = {
        "minimum_seconds": min(samples),
        "median_seconds": median,
        "maximum_seconds": max(samples),
        "median_absolute_deviation_seconds": mad,
    }
    base_result.update(
        {
            "status": "passed",
            "samples_seconds": samples,
            "statistics": stats,
        }
    )
    path = write_result(work, base_result)
    print(
        "end-to-end process wall time: "
        f"min {stats['minimum_seconds']:.6f} s, "
        f"median {stats['median_seconds']:.6f} s, "
        f"max {stats['maximum_seconds']:.6f} s, MAD "
        f"{stats['median_absolute_deviation_seconds']:.6f} s"
    )
    print(f"result: {path}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        OSError,
        RuntimeError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
    ) as error:
        print(f"benchmark: {error}", file=sys.stderr)
        raise SystemExit(1)
