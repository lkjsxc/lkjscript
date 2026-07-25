"""Brainfuck protocol command-line validation."""

import argparse

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
