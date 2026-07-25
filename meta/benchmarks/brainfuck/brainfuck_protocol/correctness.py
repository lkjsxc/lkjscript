"""Bounded diagnostic and complete correctness phases."""

import subprocess
import sys

from brainfuck_protocol.process import run_to_file, verify_output
from brainfuck_protocol.reporting import write_result

def run_diagnostic(root, work, command, args, result, reference_output, expected_length, expected_sha256):
    output = work / "diagnostic-output.bin"
    try:
        elapsed, _ = run_to_file(command, root, output, args.diagnostic_timeout)
        verify_output(output, expected_length, expected_sha256, "diagnostic")
        result["diagnostic"] = {"status": "completed", "elapsed_seconds": elapsed}
        print(f"diagnostic: completed in {elapsed:.6f} s")
    except subprocess.TimeoutExpired:
        result["diagnostic"] = {
            "status": "timed-out-as-bounded",
            "timeout_seconds": args.diagnostic_timeout,
        }
        print(f"diagnostic: timed out at {args.diagnostic_timeout:.3f} s (expected bound)")
    except RuntimeError as error:
        result["status"] = "diagnostic-failed"
        result["diagnostic"] = {"status": "failed", "error": str(error)}
        path = write_result(work, result)
        reference_output.unlink(missing_ok=True)
        print(f"diagnostic failed; result: {path}", file=sys.stderr)
        return 1
    finally:
        output.unlink(missing_ok=True)
    return None

def run_correctness(root, work, command, args, result, oracle, reference_output, expected_length, expected_sha256):
    output = work / "correctness-output.bin"
    try:
        elapsed, _ = run_to_file(command, root, output, args.timeout)
        verify_output(output, expected_length, expected_sha256, "full correctness")
    except subprocess.TimeoutExpired:
        result["status"] = "full-run-timeout"
        result["correctness"] = {"status": "timed-out", "timeout_seconds": args.timeout}
        path = write_result(work, result)
        output.unlink(missing_ok=True)
        reference_output.unlink(missing_ok=True)
        print(f"full correctness: did not complete within {args.timeout:.3f} s")
        print(f"result: {path}")
        return 2
    except RuntimeError as error:
        result["status"] = "full-correctness-failed"
        result["correctness"] = {"status": "failed", "error": str(error)}
        path = write_result(work, result)
        output.unlink(missing_ok=True)
        reference_output.unlink(missing_ok=True)
        print(f"full correctness failed; result: {path}", file=sys.stderr)
        return 1
    finally:
        output.unlink(missing_ok=True)
    oracle["byte_equal"] = True
    result["correctness"] = {"status": "passed", "elapsed_seconds": elapsed}
    print(f"full correctness: byte-equal in {elapsed:.6f} s")
    if args.mode == "correctness":
        result["status"] = "correctness-passed"
        path = write_result(work, result)
        reference_output.unlink(missing_ok=True)
        print(f"result: {path}")
        return 0
    return None
