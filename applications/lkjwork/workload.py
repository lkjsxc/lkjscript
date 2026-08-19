#!/usr/bin/env python3
"""Run one frozen deterministic lkjwork product corpus through public commands.

The reported core mutation count includes task creation, notes, and explicit
field/lifecycle mutations. Dependency edges are supplied by create_task, so
they do not require extra requests. Attachment workflows are reported
separately because each includes semantic mutation, host evidence, and resume.
"""

import argparse
import json
import pathlib
import statistics
import subprocess
import tempfile
import time


CONTRACT_VERSION = 1
SEED = "lkjwork-corpus-v1"

PROFILES = {
    "functional": {
        "tasks": 25,
        "core_mutations": 75,
        "dependencies": 30,
        "notes": 50,
        "attachments": 5,
        "queries": 100,
    },
    "representative": {
        "tasks": 500,
        "core_mutations": 2500,
        "dependencies": 1000,
        "notes": 1000,
        "attachments": 100,
        "queries": 2000,
    },
    "stress": {
        "tasks": 2000,
        "core_mutations": 10000,
        "dependencies": 4000,
        "notes": 4000,
        "attachments": 200,
        "queries": 4000,
    },
}


def percentile(values, numerator, denominator):
    ordered = sorted(values)
    index = (len(ordered) * numerator + denominator - 1) // denominator - 1
    return ordered[max(0, min(index, len(ordered) - 1))]


def timing_summary(values):
    if not values:
        return {"samples": 0}
    return {
        "samples": len(values),
        "minimum_ns": min(values),
        "median_ns": int(statistics.median(values)),
        "p95_ns": percentile(values, 95, 100),
        "maximum_ns": max(values),
    }


class Session:
    def __init__(self, binary, project):
        self.project = str(project)
        self.next_id = 1
        self.timings = {}
        self.request_bytes = 0
        self.response_bytes = 0
        self.process = subprocess.Popen(
            [str(binary), "session"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=False,
            bufsize=0,
        )

    def call(self, category, arguments, expected_exit=0):
        request = {
            "contract_version": CONTRACT_VERSION,
            "request_id": self.next_id,
            "project": self.project,
            "arguments": arguments,
        }
        self.next_id += 1
        encoded = json.dumps(request, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        started = time.perf_counter_ns()
        self.process.stdin.write(encoded)
        self.process.stdin.flush()
        response_bytes = self.process.stdout.readline()
        elapsed = time.perf_counter_ns() - started
        if not response_bytes:
            error = self.process.stderr.read().decode(errors="replace")
            raise RuntimeError(f"lkjwork session ended before response: {error}")
        response = json.loads(response_bytes)
        if response.get("request_id") != request["request_id"]:
            raise RuntimeError("lkjwork session response correlation mismatch")
        if response.get("exit") != expected_exit:
            raise RuntimeError(
                f"lkjwork {arguments!r} returned {response.get('exit')}, "
                f"expected {expected_exit}: {response!r}"
            )
        self.request_bytes += len(encoded)
        self.response_bytes += len(response_bytes)
        self.timings.setdefault(category, []).append(elapsed)
        return response["response"]

    def close(self):
        request = {
            "contract_version": CONTRACT_VERSION,
            "request_id": self.next_id,
            "arguments": ["shutdown"],
        }
        encoded = json.dumps(request, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        self.process.stdin.write(encoded)
        self.process.stdin.flush()
        response = json.loads(self.process.stdout.readline())
        if response.get("request_id") != self.next_id:
            raise RuntimeError("lkjwork shutdown response correlation mismatch")
        self.process.stdin.close()
        return_code = self.process.wait(timeout=30)
        stderr = self.process.stderr.read().decode(errors="replace")
        if return_code != 0 or stderr:
            raise RuntimeError(
                f"lkjwork session shutdown failed: exit={return_code} stderr={stderr!r}"
            )


def run_one_shot(binary, arguments):
    invocation = [str(binary), "--json", *arguments]
    argument_bytes = sum(len(argument.encode()) for argument in invocation[1:])
    started = time.perf_counter_ns()
    completed = subprocess.run(
        invocation,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    elapsed = time.perf_counter_ns() - started
    if completed.returncode != 0:
        raise RuntimeError(
            f"lkjwork {arguments!r} failed: "
            f"stdout={completed.stdout!r} stderr={completed.stderr!r}"
        )
    return json.loads(completed.stdout), elapsed, argument_bytes, len(completed.stdout)


def dependency_plan(task_count, edge_count):
    planned = {task: [] for task in range(1, task_count + 1)}
    remaining = edge_count
    distance = 1
    while remaining:
        progressed = False
        for task in range(2, task_count + 1):
            prerequisite = task - distance
            if prerequisite >= 1 and prerequisite not in planned[task]:
                planned[task].append(prerequisite)
                remaining -= 1
                progressed = True
                if not remaining:
                    break
        if not progressed:
            raise RuntimeError("requested dependency count exceeds the frozen DAG capacity")
        distance += 1
    return planned


def authority_sizes(project):
    categories = {
        "record_bytes": 0,
        "checkpoint_record_bytes": 0,
        "attempt_bytes": 0,
        "outcome_bytes": 0,
        "application_bytes": 0,
        "current_state_bytes": 0,
        "head_bytes": 0,
        "locator_bytes": 0,
        "blob_bytes": 0,
        "other_bytes": 0,
    }
    file_count = 0
    total = 0
    root = project / ".lkjwork"
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        size = path.stat().st_size
        total += size
        file_count += 1
        relative = path.relative_to(root)
        parts = relative.parts
        if "records" in parts:
            categories["record_bytes"] += size
            revision = int(relative.name.split("-", 1)[0])
            if revision % 64 == 0:
                categories["checkpoint_record_bytes"] += size
        elif "attempts" in parts:
            categories["attempt_bytes"] += size
        elif "outcomes" in parts:
            categories["outcome_bytes"] += size
        elif relative.name == "application.lkja":
            categories["application_bytes"] += size
        elif relative.name == "CURRENT":
            categories["current_state_bytes"] += size
        elif relative.name == "HEAD":
            categories["head_bytes"] += size
        elif relative.name == "locator":
            categories["locator_bytes"] += size
        elif parts and parts[0] == "blobs":
            categories["blob_bytes"] += size
        else:
            categories["other_bytes"] += size
    categories["total_bytes"] = total
    categories["file_count"] = file_count
    return categories


def execute(binary, profile, project):
    counts = PROFILES[profile]
    initialized, init_ns, init_argument_bytes, init_response_bytes = run_one_shot(
        binary, ["init", str(project), "--name", f"{profile}-corpus"]
    )
    session = Session(binary, project)
    dependency_edges = dependency_plan(counts["tasks"], counts["dependencies"])
    observed_mutations = 0
    observed_edges = 0
    observed_notes = 0
    observed_attachments = 0
    observed_queries = 0
    final_revision = initialized["revision"]
    try:
        for task in range(1, counts["tasks"] + 1):
            arguments = [
                "add",
                f"Task {task:04d}",
                "--description",
                f"Deterministic {profile} task {task}",
                "--priority",
                str(((task * 17) % 101) - 50),
                "--label",
                f"group-{task % 7}",
                "--actor",
                "corpus",
            ]
            for prerequisite in dependency_edges[task]:
                arguments.extend(["--depends", f"#{prerequisite}"])
            response = session.call("mutation", arguments)
            result = response["result"]
            if result["publication"] != "completed" or not result["published"]:
                raise RuntimeError(f"task creation did not publish: {response!r}")
            if result["value"]["task"] != task:
                raise RuntimeError("task frontier disagrees with deterministic corpus")
            final_revision = response["revision"]
            observed_mutations += 1
            observed_edges += len(dependency_edges[task])

        for note in range(1, counts["notes"] + 1):
            task = (note - 1) % counts["tasks"] + 1
            response = session.call(
                "mutation",
                [
                    "note",
                    f"#{task}",
                    "add",
                    f"Corpus note {note:05d}",
                    "--actor",
                    "corpus",
                ],
            )
            if response["result"]["publication"] != "completed":
                raise RuntimeError("note mutation did not complete")
            final_revision = response["revision"]
            observed_mutations += 1
            observed_notes += 1

        extra = counts["core_mutations"] - observed_mutations
        if extra < 0:
            raise RuntimeError("frozen core mutation count is smaller than task and note facts")
        for change in range(extra):
            task = change % counts["tasks"] + 1
            cycle = change // counts["tasks"] + 1
            priority = cycle * 10000 + task
            response = session.call(
                "mutation", ["priority", f"#{task}", str(priority)]
            )
            if response["result"]["publication"] != "completed":
                raise RuntimeError("priority mutation did not complete")
            final_revision = response["revision"]
            observed_mutations += 1

        source = project.parent / "corpus-attachment.bin"
        for attachment in range(1, counts["attachments"] + 1):
            source.write_bytes(
                f"{SEED}:{profile}:attachment:{attachment:05d}\n".encode()
            )
            task = (attachment - 1) % counts["tasks"] + 1
            response = session.call(
                "attachment",
                [
                    "attach",
                    f"#{task}",
                    str(source),
                    "--name",
                    f"evidence-{attachment:05d}.txt",
                    "--actor",
                    "corpus",
                ],
            )
            host = response["result"]["host"]
            if len(host) != 1 or host[0]["operation"] != "put_blob":
                raise RuntimeError("attachment did not use one exact blob put")
            final_revision = response["revision"]
            observed_attachments += 1

        query_shapes = [
            ["show", "#1"],
            ["list", "--limit", "20", "--order", "priority"],
            ["next", "--limit", "10"],
            ["summary"],
            [
                "context",
                "--maximum-tasks",
                "10",
                "--maximum-notes",
                "20",
                "--maximum-dependencies",
                "30",
                "--maximum-text-bytes",
                "32768",
            ],
            ["export", "--limit", "20"],
            ["history", "--limit", "20"],
        ]
        query_digests = {}
        for query in range(counts["queries"]):
            arguments = query_shapes[query % len(query_shapes)]
            response = session.call(f"query_{arguments[0]}", arguments)
            if response["revision"] != final_revision:
                raise RuntimeError("pure query selected an unexpected revision")
            result = response["result"]
            if result["published"]:
                raise RuntimeError("pure query reported publication")
            key = tuple(arguments)
            digest = result["result_digest"]
            if key in query_digests and query_digests[key] != digest:
                raise RuntimeError("repeated exact query changed its result digest")
            query_digests[key] = digest
            observed_queries += 1

        doctor = session.call("deep_audit", ["doctor", "--deep"])
        if not doctor["result"]["deep_audited"]:
            raise RuntimeError("deep audit did not report complete replay")
        if doctor["revision"] != final_revision:
            raise RuntimeError("deep audit selected an unexpected revision")
    finally:
        session.close()

    one_shot_timings = {}
    one_shot_argument_bytes = 0
    one_shot_response_bytes = 0
    one_shot_processes = 0
    for arguments in query_shapes:
        invocation = ["--project", str(project), *arguments]
        warmup, _, _, _ = run_one_shot(binary, invocation)
        one_shot_processes += 1
        if warmup["revision"] != final_revision or warmup["result"]["published"]:
            raise RuntimeError("one-shot query warm-up changed or selected the wrong authority")
        samples = []
        for _ in range(5):
            response, elapsed, argument_bytes, response_bytes = run_one_shot(binary, invocation)
            one_shot_processes += 1
            if response["revision"] != final_revision or response["result"]["published"]:
                raise RuntimeError("one-shot query changed or selected the wrong authority")
            samples.append(elapsed)
            one_shot_argument_bytes += argument_bytes
            one_shot_response_bytes += response_bytes
        one_shot_timings[f"one_shot_{arguments[0]}"] = samples

    extra = counts["core_mutations"] - counts["tasks"] - counts["notes"]
    if extra:
        final_task_one_priority = ((extra - 1) // counts["tasks"] + 1) * 10000 + 1
    else:
        final_task_one_priority = (17 % 101) - 50
    unchanged_arguments = [
        "--project",
        str(project),
        "edit",
        "#1",
        "--priority",
        str(final_task_one_priority),
    ]
    warmup, _, _, _ = run_one_shot(binary, unchanged_arguments)
    one_shot_processes += 1
    if (
        warmup["revision"] != final_revision
        or warmup["result"]["published"]
        or warmup["result"]["publication"] != "unchanged"
    ):
        raise RuntimeError("one-shot unchanged mutation did not preserve authority")
    samples = []
    for _ in range(5):
        response, elapsed, argument_bytes, response_bytes = run_one_shot(
            binary, unchanged_arguments
        )
        one_shot_processes += 1
        if (
            response["revision"] != final_revision
            or response["result"]["published"]
            or response["result"]["publication"] != "unchanged"
        ):
            raise RuntimeError("one-shot unchanged mutation published authority")
        samples.append(elapsed)
        one_shot_argument_bytes += argument_bytes
        one_shot_response_bytes += response_bytes
    one_shot_timings["one_shot_mutation_unchanged"] = samples

    if {
        "core_mutations": observed_mutations,
        "dependencies": observed_edges,
        "notes": observed_notes,
        "attachments": observed_attachments,
        "queries": observed_queries,
    } != {
        key: counts[key]
        for key in [
            "core_mutations",
            "dependencies",
            "notes",
            "attachments",
            "queries",
        ]
    }:
        raise RuntimeError("observed corpus counts disagree with frozen profile")

    inspection = doctor["result"]
    return {
        "schema": "lkjwork-corpus-evidence-v1",
        "seed": SEED,
        "profile": profile,
        "application_digest": initialized["result"]["application"],
        "instance": initialized["instance"],
        "final_revision": final_revision,
        "final_state_digest": inspection["state_digest"],
        "counts": {
            "tasks": counts["tasks"],
            "core_mutation_requests": observed_mutations,
            "dependency_edges": observed_edges,
            "notes": observed_notes,
            "attachment_workflows": observed_attachments,
            "pure_queries": observed_queries,
        },
        "history": {
            "records": inspection["history_records"],
            "bytes": inspection["history_bytes"],
            "checkpoint_revision": inspection["checkpoint_revision"],
            "normal_replay_records": inspection["normal_replay_records"],
            "current_state_cache": inspection["current_state_cache"],
            "deep_audited": inspection["deep_audited"],
        },
        "storage": authority_sizes(project),
        "transport": {
            "corpus_processes": 2,
            "one_shot_measurement_processes": one_shot_processes,
            "total_processes": 2 + one_shot_processes,
            "init_argument_utf8_bytes": init_argument_bytes,
            "init_response_bytes": init_response_bytes,
            "session_request_bytes": session.request_bytes,
            "session_response_bytes": session.response_bytes,
            "measured_one_shot_argument_utf8_bytes": one_shot_argument_bytes,
            "measured_one_shot_response_bytes": one_shot_response_bytes,
        },
        "latency": {
            "init": timing_summary([init_ns]),
            **{
                category: timing_summary(values)
                for category, values in sorted(
                    {**session.timings, **one_shot_timings}.items()
                )
            },
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=pathlib.Path)
    parser.add_argument(
        "--profile", choices=sorted(PROFILES), default="functional"
    )
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--project", type=pathlib.Path)
    arguments = parser.parse_args()
    binary = arguments.binary.resolve()
    if arguments.project:
        project = arguments.project.resolve()
        project.parent.mkdir(parents=True, exist_ok=True)
        evidence = execute(binary, arguments.profile, project)
    else:
        with tempfile.TemporaryDirectory(prefix="lkjwork-corpus-") as directory:
            evidence = execute(binary, arguments.profile, pathlib.Path(directory) / "project")
    encoded = json.dumps(evidence, sort_keys=True, separators=(",", ":")) + "\n"
    if arguments.output:
        arguments.output.parent.mkdir(parents=True, exist_ok=True)
        arguments.output.write_text(encoded)
    print(encoded, end="")


if __name__ == "__main__":
    main()
