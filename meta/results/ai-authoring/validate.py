#!/usr/bin/env python3
"""Validate and summarize retained lkjscript AI-authorability results."""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

from validation.result import validate

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", nargs="+", type=pathlib.Path)
    return parser.parse_args()


def main() -> int:
    rows = []
    for path in parse_args().results:
        result = validate(path)
        metrics = result["metrics"]
        rows.append(
            (
                result["taskId"],
                result["interface"],
                result["model"],
                result["verdict"],
                metrics["wallMilliseconds"],
                metrics["inputTokens"],
                metrics["outputTokens"],
                metrics["toolCalls"],
                metrics["compilerInvocations"],
                metrics["repairIterations"],
            )
        )
    print("task\tinterface\tmodel\tverdict\twall_ms\tinput_tokens\toutput_tokens\ttools\tcompiler\trepairs")
    for row in rows:
        print("\t".join("unmeasured" if item is None else str(item) for item in row))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"ai-authorability result invalid: {error}", file=sys.stderr)
        raise SystemExit(1)
