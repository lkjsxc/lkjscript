#!/usr/bin/env python3
"""Evolve a deployment policy through the production semantic workbench."""

import copy
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import time


CLI = pathlib.Path(sys.argv[1]).resolve()
DAEMON = pathlib.Path(sys.argv[2]).resolve()
METRICS_PATH = pathlib.Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else None
ROOT = pathlib.Path(__file__).resolve().parents[2]
JOB_DRIVER = ROOT / "examples" / "job-policy" / "driver.py"

spec = importlib.util.spec_from_file_location("lkjscript_job_policy_fixture", JOB_DRIVER)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load the retained job-policy builder")
job = importlib.util.module_from_spec(spec)
spec.loader.exec_module(job)

state = None
agent_measurements = []
packet_metrics = []


def record_process(purpose, command, input_bytes, completed, started):
    agent_measurements.append({
        "purpose": purpose,
        "command": command,
        "input_bytes": len(input_bytes),
        "stdout_bytes": len(completed.stdout),
        "stderr_bytes": len(completed.stderr),
        "elapsed_nanoseconds": time.monotonic_ns() - started,
        "exit": completed.returncode,
    })


def invoke(arguments, input_bytes, purpose, accepted_exits=(0,)):
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), *arguments],
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    record_process(purpose, arguments[:2], input_bytes, completed, started)
    if completed.returncode not in accepted_exits:
        raise RuntimeError(
            f"workbench failed for {purpose} ({completed.returncode}): "
            f"{completed.stderr.decode()}\n{completed.stdout.decode()}"
        )
    return completed


def response(completed, purpose):
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"non-JSON response for {purpose}: {error}") from error


def expect(value, kind):
    return job.expect(value, kind)


def expect_error(value, code, target=None):
    return job.expect_error(value, code, target)


def alias_map(packet):
    return {
        item["node"]: f"@{item['alias']}"
        for item in packet["payload"]["aliases"]
    }


IDENTIFIER = re.compile(r"^[a-z_][a-z0-9_]*$")


def compact(value, aliases):
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        if value in aliases:
            return aliases[value]
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, list):
        return "[ " + " ".join(compact(item, aliases) for item in value) + " ]"
    if isinstance(value, dict):
        keys = set(value)
        if keys == {"kind"}:
            kind = value["kind"]
            if not isinstance(kind, str) or IDENTIFIER.fullmatch(kind) is None:
                raise RuntimeError(f"invalid tagged-plan kind {kind!r}")
            return f"({kind})"
        if keys == {"kind", "data"}:
            kind = value["kind"]
            if not isinstance(kind, str) or IDENTIFIER.fullmatch(kind) is None:
                raise RuntimeError(f"invalid tagged-plan kind {kind!r}")
            return f"({kind} {compact(value['data'], aliases)})"
        fields = []
        for key, item in value.items():
            if IDENTIFIER.fullmatch(key) is None:
                raise RuntimeError(f"invalid plan field {key!r}")
            fields.append(f"{key} {compact(item, aliases)}")
        return "{ " + " ".join(fields) + " }"
    raise RuntimeError(f"unsupported plan value {type(value).__name__}")


def edit_plan(request, packet=None):
    if request.get("kind") != "apply_transaction":
        raise RuntimeError("edit plan requires an apply_transaction request")
    data = request["data"]
    transaction = data["transaction"]
    aliases = alias_map(packet) if packet is not None else {}
    fields = []
    if packet is not None:
        fields.append(f"packet {json.dumps(packet['digest'])}")
    fields.extend([
        f"workspace {compact(transaction['workspace'], aliases)}",
        f"base_revision {transaction['base_revision']}",
    ])
    if "idempotency_key" in transaction:
        fields.append(
            f"idempotency_key {compact(transaction['idempotency_key'], aliases)}"
        )
    fields.append(f"operations {compact(transaction['operations'], aliases)}")
    fields.append(
        "return_symbols "
        + compact(data.get("response", {}).get("return_symbols", []), aliases)
    )
    return ("plan { " + " ".join(fields) + " }").encode()


def packet_file(packet):
    cache = state / "client-cache"
    cache.mkdir(mode=0o700, exist_ok=True)
    path = cache / f"context-v1-{packet['digest']}.json"
    encoded = json.dumps(packet, separators=(",", ":")).encode()
    if not path.exists():
        path.write_bytes(encoded)
    elif path.read_bytes() != encoded:
        raise RuntimeError("context cache digest collision")
    return path


def agent_create():
    completed = invoke(
        ["agent", "create", "--state", str(state)], b"", "workspace_create"
    )
    return expect(response(completed, "workspace_create"), "workspace_created")


def agent_context(workspace, revision, purpose, targets=None, from_revision=None):
    arguments = [
        "agent", "context", "--state", str(state), "--workspace", workspace,
        "--revision", str(revision), "--purpose", purpose, "--max-nodes", "256",
    ]
    for target in targets or []:
        arguments.extend(["--target", target])
    if from_revision is not None:
        arguments.extend(["--from-revision", str(from_revision)])
    completed = invoke(arguments, b"", f"context_{purpose}_r{revision}")
    packet = response(completed, f"context_{purpose}_r{revision}")
    path = packet_file(packet)
    view = invoke(
        ["agent", "view", "--packet", str(path)],
        b"",
        f"view_{purpose}_r{revision}",
    )
    packet_metrics.append({
        "purpose": purpose,
        "revision": revision,
        "packet_bytes": len(completed.stdout),
        "view_bytes": len(view.stdout),
        "nodes": len(packet["payload"]["nodes"]),
        "truncated": packet["payload"]["omissions"]["node_scope_truncated"],
    })
    return packet, path, view.stdout.decode()


def agent_edit(request, purpose, packet=None):
    transaction = request["data"]["transaction"]
    command = "validate" if transaction["mode"] == "validate_only" else "apply"
    plan = edit_plan(request, packet)
    arguments = ["agent", command, "--state", str(state)]
    if packet is not None:
        arguments.extend(["--packet", str(packet_file(packet))])
    completed = invoke(arguments, plan, purpose)
    return response(completed, purpose), plan


def agent_run(workspace, revision, entry, arguments, purpose, fuel=1_000_000, packet=None):
    aliases = alias_map(packet) if packet is not None else {}
    fields = []
    if packet is not None:
        fields.append(f"packet {json.dumps(packet['digest'])}")
    fields.extend([
        f"workspace {compact(workspace, aliases)}",
        f"revision {revision}",
        f"entry {compact(entry, aliases)}",
        f"arguments {compact(arguments, aliases)}",
        f"policy {compact({'fuel': fuel, 'maximum_frames': 1000}, aliases)}",
    ])
    plan = ("run { " + " ".join(fields) + " }").encode()
    command = ["agent", "run", "--state", str(state)]
    if packet is not None:
        command.extend(["--packet", str(packet_file(packet))])
    return response(invoke(command, plan, purpose), purpose), plan


def same_predicted_receipt(predicted, committed):
    predicted = copy.deepcopy(predicted)
    predicted["published"] = True
    if predicted != committed:
        raise RuntimeError("validate-only and committed receipts disagree")


def update_ids(ids, receipt):
    for symbol, node in receipt["returned_bindings"]:
        ids[int(symbol.removeprefix("draft_"))] = node


def materialize_targets(value, ids):
    if isinstance(value, list):
        return [materialize_targets(item, ids) for item in value]
    if isinstance(value, dict):
        if set(value) == {"kind", "data"} and value["kind"] == "draft":
            symbol = value["data"]
            if isinstance(symbol, str) and symbol.startswith("draft_"):
                number = int(symbol.removeprefix("draft_"))
                if number in ids:
                    return job.existing(ids[number])
        return {key: materialize_targets(item, ids) for key, item in value.items()}
    return value


def rewrite_drafts(value, mapping):
    if isinstance(value, list):
        return [rewrite_drafts(item, mapping) for item in value]
    if isinstance(value, dict):
        return {key: rewrite_drafts(item, mapping) for key, item in value.items()}
    if isinstance(value, str) and value.startswith("draft_"):
        number = int(value.removeprefix("draft_"))
        return job.draft_symbol(mapping.get(number, number))
    return value


def allocation_probe(workspace, revision, purpose, packet=None):
    request = job.apply_request(
        workspace,
        revision,
        "validate_only",
        [{
            "kind": "create_package",
            "data": {"symbol": job.draft_symbol(9000), "name": "allocation-probe"},
        }],
        [9000],
    )
    value, _ = agent_edit(request, purpose, packet)
    return expect(value, "transaction_receipt")


def assert_run(workspace, revision, entry, arguments, expected, purpose, fuel=1_000_000):
    value, _ = agent_run(workspace, revision, entry, arguments, purpose, fuel)
    run = expect(value, "run")
    if run["value"] != expected:
        raise RuntimeError(f"unexpected result for {purpose}: {run['value']}")
    return run


def extension_cases(ids):
    values = copy.deepcopy(job.policy_cases(ids))
    for _, _, _, expected in values:
        if expected["data"]["variant"] == ids[161]:
            expected["data"]["payload"]["data"] += 2
    return values


def run_cases(workspace, revision, entry, cases, prefix):
    results = []
    for name, job_value, limits, expected in cases:
        results.append(assert_run(
            workspace,
            revision,
            entry,
            [job_value, limits],
            expected,
            f"{prefix}_{name}",
        ))
    return results


def extension_operations(ids, owner_block):
    helper = job.function(
        450,
        "rollout_bonus",
        [],
        "i64",
        [job.expression(451, "const_i64", 2)],
        job.result(451),
    )
    operations = [
        helper,
        {
            "kind": "insert_expression",
            "data": {
                "block": owner_block,
                "before": ids[241],
                "expression": job.expression(452, "call", {
                    "function": job.local(450), "arguments": [],
                }),
            },
        },
        {
            "kind": "insert_expression",
            "data": {
                "block": owner_block,
                "before": ids[241],
                "expression": job.expression(453, "add_i64", {
                    "lhs": job.existing_result(ids[239]),
                    "rhs": job.existing_result(ids[240]),
                }),
            },
        },
        {
            "kind": "replace_operation",
            "data": {
                "operation": job.existing(ids[241]),
                "replacement": {
                    "kind": "add_i64",
                    "data": {"lhs": job.result(453), "rhs": job.result(452)},
                },
            },
        },
    ]
    return materialize_targets(operations, ids)


def refactor_operations(ids, owner_block):
    helper = job.function(
        460,
        "combined_rollout_score",
        [
            {"symbol": 461, "name": "base", "ty": "i64"},
            {"symbol": 462, "name": "mode_bonus", "ty": "i64"},
        ],
        "i64",
        [
            job.expression(463, "const_i64", 2),
            job.expression(464, "add_i64", {
                "lhs": job.parameter(461), "rhs": job.parameter(462),
            }),
            job.expression(465, "add_i64", {
                "lhs": job.result(464), "rhs": job.result(463),
            }),
        ],
        job.result(465),
    )
    operations = [
        helper,
        {
            "kind": "insert_expression",
            "data": {
                "block": owner_block,
                "before": ids[241],
                "expression": job.expression(466, "call", {
                    "function": job.local(460),
                    "arguments": [
                        job.existing_result(ids[239]),
                        job.existing_result(ids[240]),
                    ],
                }),
            },
        },
        {
            "kind": "insert_expression",
            "data": {
                "block": owner_block,
                "before": ids[241],
                "expression": job.expression(467, "const_i64", 0),
            },
        },
        {
            "kind": "replace_operand",
            "data": {
                "operation": job.existing(ids[241]),
                "index": 0,
                "value": job.result(466),
            },
        },
        {
            "kind": "replace_operand",
            "data": {
                "operation": job.existing(ids[241]),
                "index": 1,
                "value": job.result(467),
            },
        },
        {"kind": "delete_owned_subtree", "data": {"root": job.existing(ids[452])}},
        {"kind": "delete_owned_subtree", "data": {"root": job.existing(ids[453])}},
        {"kind": "delete_owned_subtree", "data": {"root": job.existing(ids[450])}},
    ]
    return materialize_targets(operations, ids)


def debug_operations(ids):
    function = job.function(
        700,
        "debug_overflow",
        [],
        "i64",
        [
            job.expression(701, "const_i64", 9223372036854775807),
            job.expression(702, "const_i64", 1),
            job.expression(703, "add_i64", {
                "lhs": job.result(701), "rhs": job.result(702),
            }),
        ],
        job.result(703),
    )
    return materialize_targets([
        function,
        {
            "kind": "set_entry_function",
            "data": {"package": job.local(1), "function": job.local(700)},
        },
    ], ids)


def migration_operations(ids):
    limits = {
        "kind": "create_product_type",
        "data": {
            "symbol": job.draft_symbol(510),
            "module": job.local(2),
            "name": "DeploymentLimits",
            "fields": [
                {"symbol": job.draft_symbol(511), "name": "cpu", "ty": "i64"},
                {"symbol": job.draft_symbol(512), "name": "memory", "ty": "i64"},
                {
                    "symbol": job.draft_symbol(513),
                    "name": "rollout_enabled",
                    "ty": "bool",
                },
            ],
        },
    }

    decide_mapping = {number: number + 1000 for number in range(300, 323)}
    decide_mapping.update({110: 510, 111: 511, 112: 512})
    decide = rewrite_drafts(job.decide_function(), decide_mapping)
    decide["data"]["name"] = "decide_deployment"
    body = decide["data"]["body"]
    body["operations"].extend([
        job.expression(1323, "project_field", {
            "value": job.parameter(1302), "field": job.local(513),
        }),
        job.expression(1324, "construct_variant", {"variant": job.local(154)}),
        job.expression(1325, "construct_variant", {
            "variant": job.local(162), "payload": job.result(1324),
        }),
        job.expression(1326, "if", {
            "condition": job.result(1323),
            "result": job.nominal(job.local(160)),
            "then_body": job.yielding([], job.result(1322)),
            "else_body": job.yielding([], job.result(1325)),
        }),
    ])
    body["return_value"] = job.result(1326)

    main_mapping = {number: number + 1000 for number in range(400, 412)}
    main_mapping.update({110: 510, 111: 511, 112: 512, 300: 1300})
    main = rewrite_drafts(job.main_function(), main_mapping)
    main["data"]["name"] = "main_deployment"
    main_body = main["data"]["body"]
    limits_index = next(
        index
        for index, expression in enumerate(main_body["operations"])
        if expression["symbol"] == job.draft_symbol(1410)
    )
    main_body["operations"].insert(
        limits_index, job.expression(1412, "const_bool", True)
    )
    limits_constructor = next(
        expression
        for expression in main_body["operations"]
        if expression["symbol"] == job.draft_symbol(1410)
    )
    limits_constructor["operation"]["data"]["fields"].append(
        job.field(513, job.result(1412))
    )

    operations = [
        limits,
        decide,
        main,
        {
            "kind": "set_entry_function",
            "data": {"package": job.local(1), "function": job.local(1400)},
        },
        {"kind": "delete_owned_subtree", "data": {"root": job.existing(ids[400])}},
        {"kind": "delete_owned_subtree", "data": {"root": job.existing(ids[300])}},
        {"kind": "delete_owned_subtree", "data": {"root": job.existing(ids[110])}},
        {"kind": "delete_owned_subtree", "data": {"root": job.existing(ids[700])}},
    ]
    return materialize_targets(operations, ids)


def deployment_limits(ids, cpu, memory, rollout_enabled):
    return job.product_value(ids[510], [
        (ids[513], job.bool_value(rollout_enabled)),
        (ids[512], job.i64_value(memory)),
        (ids[511], job.i64_value(cpu)),
    ])


def migrated_cases(ids):
    return [
        (
            "linux_check",
            job.job_value(ids, 4, 8, True, 121, 131),
            deployment_limits(ids, 8, 16, True),
            job.accepted_value(ids, 27),
        ),
        (
            "wasm_build",
            job.job_value(ids, 3, 5, True, 122, 132),
            deployment_limits(ids, 8, 16, True),
            job.accepted_value(ids, 17),
        ),
        (
            "disabled_rollout",
            job.job_value(ids, 4, 8, True, 121, 131),
            deployment_limits(ids, 8, 16, False),
            job.rejected_value(ids, 154),
        ),
    ]


def schema_and_orientation_measurements():
    roots = ["apply_transaction", "query_repair_context", "query_semantic_diff", "run"]
    baseline = invoke(
        ["schema", *sum((["--root", root] for root in roots), [])],
        b"",
        "baseline_schema_roots",
    )
    orientation = invoke(["agent", "orient"], b"", "workbench_orientation")
    if b"machine_schema" not in orientation.stdout:
        raise RuntimeError("workbench orientation omits its schema digest")
    return {
        "baseline_root_bytes": len(baseline.stdout),
        "workbench_orientation_bytes": len(orientation.stdout),
        "roots": roots,
    }


def execute():
    global state
    orientation = schema_and_orientation_measurements()
    started = time.monotonic_ns()
    with tempfile.TemporaryDirectory(prefix="lkjscript-agent-maintenance-") as directory:
        state = pathlib.Path(directory)
        os.chmod(state, 0o700)
        job.state = state
        job.request_id = 0
        job.query_id = 0
        job.measurements.clear()
        job.readiness_nanoseconds.clear()
        job.start_daemon()

        workspace = agent_create()["workspace"]
        orient_packet, _, orient_view = agent_context(workspace, 0, "orient")
        if orient_packet["payload"]["summary"]["revision"] != 0:
            raise RuntimeError("initial context revision mismatch")

        operations = job.application_operations()
        maintenance_symbols = sorted(set(
            job.selected_symbols() + [1, 2, 200, 210, 220, 230, 260]
        ))
        raw_prediction_request = job.apply_request(
            workspace, 0, "validate_only", operations, maintenance_symbols
        )
        raw_prediction = expect(
            job.rpc(raw_prediction_request, "baseline_json_validate"),
            "transaction_receipt",
        )
        candidate_prediction, initial_plan = agent_edit(
            raw_prediction_request, "candidate_plan_validate"
        )
        candidate_prediction = expect(candidate_prediction, "transaction_receipt")
        if raw_prediction != candidate_prediction:
            raise RuntimeError("JSON and edit-plan validation disagree")
        creation_request = job.apply_request(
            workspace, 0, "commit", operations, maintenance_symbols
        )
        creation, _ = agent_edit(creation_request, "candidate_plan_commit")
        creation = expect(creation, "transaction_receipt")
        same_predicted_receipt(candidate_prediction, creation)
        ids = {
            int(symbol.removeprefix("draft_")): node
            for symbol, node in creation["returned_bindings"]
        }
        if creation["revision"] != 1 or creation["complete_after"]:
            raise RuntimeError("seed revision must be incomplete")

        repair_packet, _, repair_view = agent_context(
            workspace, 1, "repair", [ids[241]]
        )
        repair_observation = next(
            item
            for item in repair_packet["payload"]["observations"]
            if item["role"] == "repair_context"
        )
        repair = expect(
            expect(repair_observation["outcome"], "success"), "repair_context"
        )
        owner_block = repair["owner_block"]
        if repair["expected_type"] != "i64" or "placeholder(i64)" not in repair_view:
            raise RuntimeError("repair packet or view omits the typed placeholder")

        probe_before = allocation_probe(
            workspace, 1, "allocation_probe_before_invalid", repair_packet
        )
        invalid, _ = agent_edit(
            job.invalid_repair_request(workspace, ids),
            "invalid_repair",
            repair_packet,
        )
        invalid_error = expect_error(invalid, "type_mismatch", ids[241])
        probe_after = allocation_probe(
            workspace, 1, "allocation_probe_after_invalid", repair_packet
        )
        for field in ("revision", "hash", "created_count", "returned_bindings"):
            if probe_before[field] != probe_after[field]:
                raise RuntimeError("invalid repair changed identity allocation")
        incomplete, _ = agent_run(
            workspace, 1, ids[400], [], "incomplete_revision_run"
        )
        expect_error(incomplete, "compile_incomplete")

        valid_validate = copy.deepcopy(job.valid_repair_request(workspace, ids))
        valid_validate["data"]["transaction"]["mode"] = "validate_only"
        predicted, _ = agent_edit(
            valid_validate, "valid_repair_validate", repair_packet
        )
        repaired, _ = agent_edit(
            job.valid_repair_request(workspace, ids), "valid_repair_commit", repair_packet
        )
        predicted = expect(predicted, "transaction_receipt")
        repaired = expect(repaired, "transaction_receipt")
        same_predicted_receipt(predicted, repaired)
        if repaired["revision"] != 2 or repaired["created_count"] != 0:
            raise RuntimeError("identity-preserving repair allocated nodes")
        assert_run(
            workspace, 2, ids[400], [], job.accepted_value(ids, 25), "repaired_main"
        )
        run_cases(workspace, 2, ids[300], job.policy_cases(ids), "repaired")

        extend_packet, _, _ = agent_context(workspace, 2, "extend", [ids[241]])
        extension_request = job.apply_request(
            workspace,
            2,
            "commit",
            extension_operations(ids, owner_block),
            [450, 451, 452, 453],
        )
        extension, _ = agent_edit(extension_request, "behavior_extension", extend_packet)
        extension = expect(extension, "transaction_receipt")
        update_ids(ids, extension)
        if extension["revision"] != 3:
            raise RuntimeError("extension revision mismatch")
        assert_run(
            workspace, 3, ids[400], [], job.accepted_value(ids, 27), "extended_main"
        )
        extended_cases = extension_cases(ids)
        run_cases(workspace, 3, ids[300], extended_cases, "extended")

        refactor_packet, _, _ = agent_context(workspace, 3, "refactor", [ids[241]])
        refactor_request = job.apply_request(
            workspace, 3, "commit", refactor_operations(ids, owner_block), [460, 466, 467]
        )
        refactor_validate = copy.deepcopy(refactor_request)
        refactor_validate["data"]["transaction"]["mode"] = "validate_only"
        predicted_refactor, _ = agent_edit(
            refactor_validate, "refactor_validate", refactor_packet
        )
        refactored, _ = agent_edit(
            refactor_request, "refactor_commit", refactor_packet
        )
        predicted_refactor = expect(predicted_refactor, "transaction_receipt")
        refactored = expect(refactored, "transaction_receipt")
        same_predicted_receipt(predicted_refactor, refactored)
        update_ids(ids, refactored)
        assert_run(
            workspace, 4, ids[400], [], job.accepted_value(ids, 27), "refactored_main"
        )
        run_cases(workspace, 4, ids[300], extended_cases, "refactored")

        rename_packet, _, _ = agent_context(workspace, 4, "refactor", [ids[102]])
        rename_request = job.apply_request(workspace, 4, "commit", [{
            "kind": "rename_node",
            "data": {"node": job.existing(ids[102]), "name": "memory_units"},
        }])
        renamed, _ = agent_edit(rename_request, "presentation_rename", rename_packet)
        renamed = expect(renamed, "transaction_receipt")
        if renamed["revision"] != 5 or renamed["created_count"] != 0:
            raise RuntimeError("rename changed identity allocation")
        assert_run(
            workspace, 5, ids[400], [], job.accepted_value(ids, 27), "renamed_main"
        )

        debug_request = job.apply_request(
            workspace, 5, "commit", debug_operations(ids), [700, 701, 703]
        )
        debug_revision, _ = agent_edit(debug_request, "publish_debug_trap")
        debug_revision = expect(debug_revision, "transaction_receipt")
        update_ids(ids, debug_revision)
        trapped, _ = agent_run(
            workspace, 6, ids[700], [], "debug_overflow_run"
        )
        trap = expect_error(trapped, "runtime_trap", ids[703])
        debug_packet, _, debug_view = agent_context(
            workspace, 6, "debug", [ids[703]]
        )
        if "add_i64" not in debug_view:
            raise RuntimeError("debug context omits the trapping semantic operation")
        fix_request = job.apply_request(workspace, 6, "commit", [{
            "kind": "replace_operation",
            "data": {
                "operation": job.existing(ids[701]),
                "replacement": {"kind": "const_i64", "data": -1},
            },
        }])
        fix_validate = copy.deepcopy(fix_request)
        fix_validate["data"]["transaction"]["mode"] = "validate_only"
        fixed_prediction, _ = agent_edit(
            fix_validate, "debug_fix_validate", debug_packet
        )
        fixed, _ = agent_edit(fix_request, "debug_fix_commit", debug_packet)
        fixed_prediction = expect(fixed_prediction, "transaction_receipt")
        fixed = expect(fixed, "transaction_receipt")
        same_predicted_receipt(fixed_prediction, fixed)
        assert_run(workspace, 7, ids[700], [], job.i64_value(0), "debug_fixed_run")

        delete_packet, _, _ = agent_context(workspace, 7, "delete", [ids[110]])
        blocked_delete_request = job.apply_request(workspace, 7, "commit", [{
            "kind": "delete_owned_subtree",
            "data": {"root": job.existing(ids[110])},
        }])
        blocked_delete, _ = agent_edit(
            blocked_delete_request, "blocked_old_limits_delete", delete_packet
        )
        delete_error = expect_error(blocked_delete, "delete_blocked", ids[110])
        probe_after_delete = allocation_probe(
            workspace, 7, "allocation_probe_after_blocked_delete", delete_packet
        )
        if probe_after_delete["base_revision"] != 7:
            raise RuntimeError("blocked deletion changed the head")

        migration_request = job.apply_request(
            workspace,
            7,
            "commit",
            migration_operations(ids),
            [510, 511, 512, 513, 1300, 1326, 1400],
        )
        migration_validate = copy.deepcopy(migration_request)
        migration_validate["data"]["transaction"]["mode"] = "validate_only"
        migration_prediction, migration_plan = agent_edit(
            migration_validate, "migration_validate", delete_packet
        )
        migrated, _ = agent_edit(
            migration_request, "migration_commit", delete_packet
        )
        migration_prediction = expect(migration_prediction, "transaction_receipt")
        migrated = expect(migrated, "transaction_receipt")
        same_predicted_receipt(migration_prediction, migrated)
        update_ids(ids, migrated)
        if migrated["revision"] != 8:
            raise RuntimeError("migration revision mismatch")
        assert_run(
            workspace, 8, ids[1400], [], job.accepted_value(ids, 27), "migrated_main"
        )
        run_cases(workspace, 8, ids[1300], migrated_cases(ids), "migrated")

        review_packet, review_path, _ = agent_context(
            workspace, 8, "review", from_revision=2
        )
        diff = invoke(
            ["agent", "diff", "--packet", str(review_path)],
            b"",
            "multi_revision_diff",
        )
        if b"deleted product_type" not in diff.stdout or b"created product_type" not in diff.stdout:
            raise RuntimeError("migration review omits replacement declaration facts")
        if review_packet["payload"]["omissions"]["semantic_diff_truncated"]:
            raise RuntimeError("maintenance semantic diff unexpectedly truncated")

        job.stop_daemon("shutdown_before_restart")
        job.start_daemon()
        restarted_packet, _, _ = agent_context(workspace, 8, "orient")
        if restarted_packet["digest"] != agent_context(workspace, 8, "orient")[0]["digest"]:
            raise RuntimeError("restart context packet is nondeterministic")
        old_incomplete, _ = agent_run(
            workspace, 1, ids[400], [], "restart_incomplete"
        )
        expect_error(old_incomplete, "compile_incomplete")
        assert_run(
            workspace, 2, ids[400], [], job.accepted_value(ids, 25), "restart_repaired"
        )
        for revision in (3, 4, 5):
            assert_run(
                workspace,
                revision,
                ids[400],
                [],
                job.accepted_value(ids, 27),
                f"restart_revision_{revision}",
            )
        historical_trap, _ = agent_run(
            workspace, 6, ids[700], [], "restart_historical_trap"
        )
        expect_error(historical_trap, "runtime_trap", ids[703])
        assert_run(
            workspace, 7, ids[700], [], job.i64_value(0), "restart_fixed_debug"
        )
        assert_run(
            workspace, 8, ids[1400], [], job.accepted_value(ids, 27), "restart_current"
        )

        workspace_directory = state / "workspaces" / workspace
        artifact_sizes = {
            str(revision): (
                workspace_directory / "revisions" / f"{revision:020d}.lkjscript"
            ).stat().st_size
            for revision in range(1, 9)
        }
        cache_bytes = sum(
            path.stat().st_size
            for path in (state / "client-cache").iterdir()
            if path.is_file()
        )
        job.stop_daemon("final_shutdown")

        baseline_measurement = next(
            item for item in job.measurements
            if item["purpose"] == "baseline_json_validate"
        )
        candidate_measurement = next(
            item for item in agent_measurements
            if item["purpose"] == "candidate_plan_validate"
        )
        summary = {
            "application": "release_deployment_policy",
            "revisions": {
                "incomplete": 1,
                "repaired": 2,
                "extended": 3,
                "refactored": 4,
                "renamed": 5,
                "debug_trap": 6,
                "debug_fixed": 7,
                "migrated": 8,
            },
            "oracles": {
                "incomplete_blocked": True,
                "invalid_repair_atomic": invalid_error["code"] == "type_mismatch",
                "identity_preserving_repair": repaired["created_count"] == 0,
                "pre_extension_score": 25,
                "extended_and_refactored_score": 27,
                "rename_identity": ids[102],
                "debug_trap": {"code": trap["code"], "target": trap["target"]},
                "blocked_delete": delete_error["code"],
                "migration": "Limits -> DeploymentLimits",
                "restart": True,
                "historical_runs": True,
            },
            "interface_comparison": {
                "task": "validate identical initial application creation",
                "baseline_json_accepted": True,
                "candidate_plan_accepted": True,
                "receipts_identical": raw_prediction == candidate_prediction,
                "baseline_request_bytes": baseline_measurement["json_request_bytes"],
                "baseline_response_bytes": baseline_measurement["json_response_bytes"],
                "candidate_plan_bytes": len(initial_plan),
                "candidate_response_bytes": candidate_measurement["stdout_bytes"],
                "migration_plan_bytes": len(migration_plan),
            },
            "observation": {
                **orientation,
                "initial_view_bytes": len(orient_view.encode()),
                "packet_metrics": packet_metrics,
                "derived_cache_bytes": cache_bytes,
            },
            "interaction": {
                "agent_cli_processes": len(agent_measurements),
                "raw_baseline_requests": sum(
                    1 for item in job.measurements if item["counted"]
                ),
                "agent_input_bytes": sum(item["input_bytes"] for item in agent_measurements),
                "agent_stdout_bytes": sum(item["stdout_bytes"] for item in agent_measurements),
                "semantic_rejections": 2,
                "runtime_failures": 3,
            },
            "storage": {
                "artifact_bytes": artifact_sizes,
                "head_bytes": (workspace_directory / "HEAD").stat().st_size,
            },
            "timing": {
                "wall_nanoseconds": time.monotonic_ns() - started,
                "cold_readiness_nanoseconds": job.readiness_nanoseconds[0],
                "restart_readiness_nanoseconds": job.readiness_nanoseconds[1],
            },
            "provider_telemetry": "unavailable",
            "shutdown": "acknowledged",
        }
        if METRICS_PATH is not None:
            METRICS_PATH.write_text(json.dumps({
                "summary": summary,
                "agent_measurements": agent_measurements,
                "raw_measurements": job.measurements,
            }, separators=(",", ":")) + "\n")
        return summary


def main():
    print(json.dumps(execute(), separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    finally:
        if job.daemon is not None and job.daemon.poll() is None:
            job.daemon.terminate()
            try:
                job.daemon.wait(timeout=5)
            except subprocess.TimeoutExpired:
                job.daemon.kill()
                job.daemon.wait()
