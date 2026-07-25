"""Independent reference-oracle execution."""

import pathlib
import subprocess
import sys

from brainfuck_protocol.process import run_to_file, verify_output
from brainfuck_protocol.reporting import write_result

def verify_reference(root, work, command, output, expected_length, expected_sha256,
                     timeout, result, oracle):
    try:
        run_to_file(command, root, output, timeout)
        verify_output(output, expected_length, expected_sha256, "reference")
    except subprocess.TimeoutExpired:
        result["status"] = "oracle-timeout"
        oracle["execution"] = {"status": "timed-out", "timeout_seconds": timeout}
        path = write_result(work, result)
        output.unlink(missing_ok=True)
        print(f"oracle timed out; result: {path}")
        return 2
    except RuntimeError as error:
        result["status"] = "oracle-failed"
        oracle["execution"] = {"status": "failed", "error": str(error)}
        path = write_result(work, result)
        output.unlink(missing_ok=True)
        print(f"oracle failed; result: {path}", file=sys.stderr)
        return 1
    oracle["output_verified"] = True
    print(
        f"oracle: independent C interpreter produced {expected_length} bytes, "
        f"SHA-256 {expected_sha256}"
    )
    return None
