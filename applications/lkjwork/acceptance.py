#!/usr/bin/env python3
"""Run the complete deterministic lkjwork public product acceptance story."""

import argparse
import hashlib
import json
import pathlib
import shutil
import stat
import subprocess
import tempfile


DOMAIN_CONFLICT = 10


def run(command, *, cwd, expected=0):
    completed = subprocess.run(
        [str(item) for item in command],
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != expected:
        raise RuntimeError(
            f"command {command!r} returned {completed.returncode}, expected {expected}; "
            f"stdout={completed.stdout!r}; stderr={completed.stderr!r}"
        )
    return completed


def machine(binary, cwd, arguments, expected=0):
    completed = run([binary, "--json", *arguments], cwd=cwd, expected=expected)
    if completed.stderr:
        raise RuntimeError(f"machine output contaminated stderr: {completed.stderr!r}")
    lines = completed.stdout.splitlines()
    if len(lines) != 1:
        raise RuntimeError("one-shot machine output must contain exactly one frame")
    return json.loads(lines[0])


def project_machine(binary, cwd, project, arguments, expected=0):
    return machine(
        binary,
        cwd,
        ["--project", str(project), *arguments],
        expected,
    )


def result_value(response):
    return response["result"]["value"]


def authority_digest(project):
    root = project / ".lkjwork"
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode):
            raise RuntimeError(f"authority contains a symlink: {path}")
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix().encode()
        content = path.read_bytes()
        digest.update(len(relative).to_bytes(8, "little"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "little"))
        digest.update(content)
    return digest.hexdigest()


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def execute(binary):
    with tempfile.TemporaryDirectory(prefix="lkjwork-acceptance-") as directory:
        root = pathlib.Path(directory)
        installed = root / "installed" / "lkjwork"
        installed.parent.mkdir(mode=0o700)
        shutil.copy2(binary, installed)
        installed.chmod(0o700)

        project = root / "project"
        backup = root / "backup"
        restored = root / "restored"
        corrupt = root / "corrupt"
        fake = root / "fake"
        evidence = root / "query-evidence.txt"
        evidence.write_text("deterministic query evidence\n", encoding="utf-8")

        version = machine(installed, root, ["version"])
        initialized = machine(
            installed,
            root,
            ["init", str(project), "--name", "lkjscript-next"],
        )
        require(initialized["revision"] == 0, "initial revision is not zero")
        require(
            initialized["result"]["application"]
            == version["result"]["application_digest"],
            "initialized and embedded applications differ",
        )

        tasks = [
            ("Query contract", "60", "runtime"),
            ("Text values", "50", "language"),
            ("Sequence values", "40", "language"),
            ("Storage checkpoints", "30", "storage"),
            ("Product client", "20", "product"),
            ("Verification", "10", "test"),
        ]
        for expected_id, (title, priority, label) in enumerate(tasks, 1):
            created = project_machine(
                installed,
                root,
                project,
                [
                    "add",
                    title,
                    "--priority",
                    priority,
                    "--label",
                    label,
                    "--actor",
                    "campaign",
                ],
            )
            require(
                result_value(created)["task"] == expected_id,
                "task frontier is not deterministic",
            )

        for task, prerequisite in [(2, 1), (3, 1), (5, 2), (5, 3), (5, 4), (6, 5)]:
            project_machine(
                installed,
                root,
                project,
                ["depend", f"#{task}", "--on", f"#{prerequisite}"],
            )

        before_cycle = project_machine(installed, root, project, ["summary"])
        cycle = project_machine(
            installed,
            root,
            project,
            ["depend", "#1", "--on", "#6"],
            DOMAIN_CONFLICT,
        )
        require(cycle["revision"] == before_cycle["revision"], "cycle consumed a revision")
        require(result_value(cycle)["code"] == "dependency_cycle", "cycle was not typed")

        initial_list = project_machine(
            installed,
            root,
            project,
            ["list", "--limit", "20", "--order", "priority"],
        )
        initial_context = project_machine(
            installed,
            root,
            project,
            [
                "context",
                "--maximum-tasks",
                "5",
                "--maximum-notes",
                "5",
                "--maximum-dependencies",
                "10",
                "--maximum-text-bytes",
                "4096",
            ],
        )
        require(not initial_list["result"]["published"], "list published authority")
        require(not initial_context["result"]["published"], "context published authority")

        blocked = project_machine(
            installed, root, project, ["start", "#5"], DOMAIN_CONFLICT
        )
        require(result_value(blocked)["code"] == "task_blocked", "blocked start was not typed")
        project_machine(installed, root, project, ["start", "#1"])
        project_machine(
            installed,
            root,
            project,
            [
                "note",
                "#1",
                "add",
                "Pure query boundary reproduced.",
                "--actor",
                "agent",
            ],
        )
        attached = project_machine(
            installed,
            root,
            project,
            [
                "attach",
                "#1",
                str(evidence),
                "--name",
                "query-evidence.txt",
                "--actor",
                "agent",
            ],
        )
        require(
            [item["operation"] for item in attached["result"]["host"]] == ["put_blob"],
            "production attachment did not perform exactly one put",
        )
        project_machine(installed, root, project, ["finish", "#1"])

        base_response = project_machine(installed, root, project, ["summary"])
        base = str(base_response["revision"])
        first = project_machine(
            installed,
            root,
            project,
            [
                "--base-revision",
                base,
                "--idempotency-key",
                "acceptance-label-1",
                "label",
                "#2",
                "add",
                "ready",
            ],
        )
        project_machine(installed, root, project, ["priority", "#4", "35"])
        replay = project_machine(
            installed,
            root,
            project,
            [
                "--base-revision",
                base,
                "--idempotency-key",
                "acceptance-label-1",
                "label",
                "#2",
                "add",
                "ready",
            ],
        )
        require(first["result"]["replayed"] is False, "first mutation was a replay")
        require(replay["result"]["replayed"] is True, "exact mutation was not replayed")
        require(replay["revision"] == first["revision"], "replay selected a new revision")

        before_queries = authority_digest(project)
        queries = [
            ["show", "#1"],
            ["list", "--limit", "20", "--order", "priority"],
            ["next", "--limit", "5"],
            ["summary"],
            ["context", "--maximum-tasks", "5", "--maximum-notes", "5"],
            ["export", "--limit", "20"],
            ["history", "--limit", "20"],
        ]
        query_results = [
            project_machine(installed, root, project, arguments) for arguments in queries
        ]
        require(
            all(response["result"]["published"] is False for response in query_results),
            "a read-only product operation reported publication",
        )
        require(authority_digest(project) == before_queries, "a pure query changed authority bytes")
        known_digest = query_results[1]["result"]["result_digest"]
        unchanged = project_machine(
            installed,
            root,
            project,
            [
                "--known-result-digest",
                known_digest,
                "list",
                "--limit",
                "20",
                "--order",
                "priority",
            ],
        )
        require(unchanged["result"]["unchanged"] is True, "known digest did not compact output")
        require(authority_digest(project) == before_queries, "digest query changed authority bytes")

        ready_ids = [
            task["id"] for task in result_value(query_results[2])["tasks"]
        ]
        require(ready_ids[:3] == [2, 3, 4], "readiness or priority order is incorrect")
        for task in [2, 3, 4]:
            project_machine(installed, root, project, ["start", f"#{task}"])
            project_machine(installed, root, project, ["finish", f"#{task}"])
        next_after_prerequisites = project_machine(
            installed, root, project, ["next", "--limit", "5"]
        )
        require(
            [task["id"] for task in result_value(next_after_prerequisites)["tasks"]][0] == 5,
            "completing prerequisites did not derive dependent readiness",
        )
        project_machine(installed, root, project, ["start", "#5"])
        project_machine(installed, root, project, ["finish", "#5"])
        project_machine(
            installed,
            root,
            project,
            [
                "note",
                "#6",
                "add",
                "Public lkjwork campaign acceptance completed.",
                "--actor",
                "codex",
            ],
        )
        project_machine(installed, root, project, ["start", "#6"])
        dogfood = project_machine(installed, root, project, ["finish", "#6"])

        machine(
            installed,
            root,
            ["init", str(fake), "--name", "fault-project", "--deterministic-fake"],
        )
        project_machine(installed, root, fake, ["add", "Attachment reconciliation"])
        unknown = project_machine(
            installed,
            root,
            fake,
            [
                "attach",
                "#1",
                str(evidence),
                "--fake-put",
                "unknown",
                "--fake-inspect",
                "absent",
            ],
            DOMAIN_CONFLICT,
        )
        host = unknown["result"]["host"]
        require(
            [(item["operation"], item["class"]) for item in host]
            == [
                ("put_blob", "outcome_unknown"),
                ("inspect_blob", "reconciliation_absent"),
            ],
            "unknown attachment was retried or misclassified",
        )

        deep = project_machine(installed, root, project, ["doctor", "--deep"])
        backup_receipt = project_machine(
            installed, root, project, ["backup", "--to", str(backup)]
        )
        machine(installed, root, ["restore", str(backup), "--to", str(restored)])
        original_export = project_machine(installed, root, project, ["export", "--limit", "20"])
        restored_export = project_machine(installed, root, restored, ["export", "--limit", "20"])
        require(
            result_value(original_export) == result_value(restored_export),
            "restored semantic export differs",
        )
        require(
            original_export["result"]["state_digest"]
            == restored_export["result"]["state_digest"],
            "restored state digest differs",
        )

        project_machine(installed, root, project, ["backup", "--to", str(corrupt)])
        records = sorted((corrupt / ".lkjwork" / "instance-store").glob("*/records/*.lkis"))
        require(bool(records), "backup has no instance records to corrupt")
        bytes_value = bytearray(records[0].read_bytes())
        bytes_value[-1] ^= 1
        records[0].write_bytes(bytes_value)
        corrupt_result = project_machine(
            installed, root, corrupt, ["doctor", "--deep"], expected=3
        )
        require(
            corrupt_result["error"]["code"] == "instance_validation",
            "doctor did not reject corrupt authority",
        )

        isolated = project_machine(installed, root, project, ["show", "#6"])
        require(result_value(isolated)["task"]["phase"] == "done", "installed binary lost state")
        backup_summary = dict(backup_receipt["result"])
        backup_summary.pop("destination", None)
        return {
            "schema": "lkjwork-acceptance-evidence-v1",
            "application_digest": version["result"]["application_digest"],
            "release": version["result"]["release"],
            "instance": initialized["instance"],
            "final_revision": dogfood["revision"],
            "final_state_digest": deep["result"]["state_digest"],
            "dogfood_task": 6,
            "dogfood_result": result_value(dogfood),
            "query_result_digest": known_digest,
            "query_authority_digest": before_queries,
            "attachment_host_operations": attached["result"]["host"],
            "unknown_attachment_host_operations": host,
            "backup": backup_summary,
            "deep_audit": deep["result"],
            "restore_state_digest": restored_export["result"]["state_digest"],
            "corruption_error": corrupt_result["error"],
            "installed_binary_operated_outside_repository": True,
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    arguments = parser.parse_args()
    repository = pathlib.Path(__file__).resolve().parents[2]
    if arguments.binary is None:
        run(
            ["cargo", "build", "--workspace", "--release", "--locked"],
            cwd=repository,
        )
        binary = repository / "target" / "release" / "lkjwork"
    else:
        binary = arguments.binary.resolve()
    evidence = execute(binary)
    encoded = json.dumps(evidence, sort_keys=True, separators=(",", ":")) + "\n"
    if arguments.output:
        arguments.output.parent.mkdir(parents=True, exist_ok=True)
        arguments.output.write_text(encoded, encoding="utf-8")
    print(encoded, end="")


if __name__ == "__main__":
    main()
