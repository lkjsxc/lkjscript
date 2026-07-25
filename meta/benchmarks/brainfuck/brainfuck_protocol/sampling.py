"""Warmup, sampling, statistics, and final reporting."""

import statistics
import subprocess
import sys

from brainfuck_protocol.process import run_to_file, verify_output
from brainfuck_protocol.reporting import write_result

def measure(root, work, command, args, result, reference_output, expected_length, expected_sha256):
    output = work / "sample-output.bin"
    samples: list[float] = []
    active_sample = "warmup 1"
    try:
        for index in range(args.warmups):
            active_sample = f"warmup {index + 1}"
            elapsed, _ = run_to_file(command, root, output, args.timeout)
            verify_output(output, expected_length, expected_sha256, active_sample)
            print(f"warmup {index + 1}/{args.warmups}: {elapsed:.6f} s")
        for index in range(args.runs):
            active_sample = f"measured {index + 1}"
            elapsed, _ = run_to_file(command, root, output, args.timeout)
            verify_output(output, expected_length, expected_sha256, active_sample)
            samples.append(elapsed)
            print(f"measured {index + 1}/{args.runs}: {elapsed:.6f} s")
    except subprocess.TimeoutExpired:
        result["status"] = "measurement-timeout"
        result["measurement_failure"] = {
            "status": "timed-out", "sample": active_sample,
            "timeout_seconds": args.timeout,
        }
        result["samples_seconds"] = samples
        path = write_result(work, result)
        print(f"measurement timed out at {args.timeout:.3f} s; result: {path}")
        return 2
    except RuntimeError as error:
        result["status"] = "measurement-failed"
        result["measurement_failure"] = {
            "status": "failed", "sample": active_sample, "error": str(error),
        }
        result["samples_seconds"] = samples
        path = write_result(work, result)
        print(f"measurement failed; result: {path}", file=sys.stderr)
        return 1
    finally:
        output.unlink(missing_ok=True)
        reference_output.unlink(missing_ok=True)
    median = statistics.median(samples)
    mad = statistics.median(abs(sample - median) for sample in samples)
    stats = {
        "minimum_seconds": min(samples), "median_seconds": median,
        "maximum_seconds": max(samples),
        "median_absolute_deviation_seconds": mad,
    }
    result.update({"status": "passed", "samples_seconds": samples, "statistics": stats})
    path = write_result(work, result)
    print(
        "end-to-end process wall time: "
        f"min {stats['minimum_seconds']:.6f} s, "
        f"median {stats['median_seconds']:.6f} s, "
        f"max {stats['maximum_seconds']:.6f} s, MAD "
        f"{stats['median_absolute_deviation_seconds']:.6f} s"
    )
    print(f"result: {path}")
    return 0
