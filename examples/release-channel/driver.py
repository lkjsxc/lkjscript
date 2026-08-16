#!/usr/bin/env python3
"""Replay the sealed release-channel authoring task through the public JSON control plane."""

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
CLI = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/release/lkjscript"
DAEMON = pathlib.Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else ROOT / "target/release/lkjscriptd"
METRICS_PATH = pathlib.Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else None
PROPOSAL_PATH = pathlib.Path(__file__).with_name("proposal.json")
request_id = 0
query_id = 0
daemon = None
state = None
measurements = []
readiness_nanoseconds = []


def rpc(request, purpose, counted=True):
    global request_id
    request_id += 1
    envelope = {"version": 5, "request_id": request_id, "request": request}
    encoded = json.dumps(envelope, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), "--state", str(state), "rpc"],
        input=encoded,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    elapsed = time.monotonic_ns() - started
    if completed.returncode != 0:
        raise RuntimeError(
            f"CLI failed for {purpose} ({completed.returncode}): {completed.stderr.decode()}"
        )
    if completed.stderr:
        raise RuntimeError(f"CLI wrote stderr for {purpose}: {completed.stderr.decode()}")
    response_envelope = json.loads(completed.stdout)
    if response_envelope.get("version") != 5 or response_envelope.get("request_id") != request_id:
        raise RuntimeError(f"response envelope mismatch for {purpose}")
    measurements.append({
        "purpose": purpose,
        "counted": counted,
        "elapsed_nanoseconds": elapsed,
        "json_request_bytes": len(encoded),
        "json_response_bytes": len(completed.stdout),
        "request": envelope,
        "response": response_envelope,
    })
    return response_envelope["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response.get("data")


def expect_error(response, code, target=None):
    error = expect(response, "error")
    if error.get("code") != code:
        raise RuntimeError(f"expected error {code}, received {error}")
    if target is not None and error.get("target") != target:
        raise RuntimeError(f"expected error target {target}, received {error}")
    return error


def existing(node):
    return {"kind": "existing", "data": node}


def existing_result(node):
    return {"kind": "operation_result", "data": {"operation": existing(node), "output": 0}}


def function_parameter(node):
    return {"kind": "function_parameter", "data": existing(node)}


def start_daemon():
    global daemon
    started = time.monotonic_ns()
    daemon = subprocess.Popen(
        [str(DAEMON), "--state", str(state), "--foreground"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    endpoint = state / "lkjscript.sock"
    deadline = time.monotonic() + 5
    while not endpoint.exists():
        if daemon.poll() is not None:
            raise RuntimeError(f"daemon exited early: {daemon.stderr.read().decode()}")
        if time.monotonic() >= deadline:
            raise RuntimeError("daemon readiness timeout")
        time.sleep(0.001)
    readiness_nanoseconds.append(time.monotonic_ns() - started)


def stop_daemon(purpose):
    global daemon
    if daemon is None or daemon.poll() is not None:
        raise RuntimeError(f"daemon is absent before typed shutdown {purpose}")
    expect(rpc({"kind": "shutdown"}, purpose, counted=False), "acknowledged")
    if daemon.wait(timeout=5) != 0:
        raise RuntimeError("daemon shutdown failed")
    daemon_error = daemon.stderr.read()
    if daemon_error:
        raise RuntimeError(f"daemon wrote stderr: {daemon_error.decode()}")
    daemon = None


def query_batch(workspace, revision, queries, purpose):
    global query_id
    request_queries = []
    expected_ids = []
    for query_value in queries:
        query_id += 1
        expected_ids.append(query_id)
        request_queries.append({"id": query_id, "query": query_value})
    result = expect(rpc({
        "kind": "query_batch",
        "data": {"workspace": workspace, "revision": revision, "queries": request_queries},
    }, purpose), "query_batch_result")
    if [item["id"] for item in result["results"]] != expected_ids:
        raise RuntimeError(f"query correlation mismatch for {purpose}")
    return [expect(item["outcome"], "success") for item in result["results"]]


def query(workspace, revision, query_value, purpose):
    return query_batch(workspace, revision, [query_value], purpose)[0]


def apply_request(workspace, base_revision, mode, operations, return_symbols=None):
    return {
        "kind": "apply_transaction",
        "data": {
            "transaction": {
                "workspace": workspace,
                "base_revision": base_revision,
                "mode": mode,
                "operations": operations,
            },
            "response": {"return_symbols": return_symbols or []},
        },
    }


def run_request(workspace, revision, entry, arguments, purpose, fuel=100_000):
    return rpc({
        "kind": "run",
        "data": {
            "workspace": workspace,
            "revision": revision,
            "entry": entry,
            "arguments": arguments,
            "policy": {"fuel": fuel, "maximum_frames": 1_000},
        },
    }, purpose)


def i64(value):
    return {"kind": "i64", "data": value}


def boolean(value):
    return {"kind": "bool", "data": value}


def product(type_id, fields):
    return {
        "kind": "product",
        "data": {
            "ty": type_id,
            "fields": [{"field": field_id, "value": value} for field_id, value in fields],
        },
    }


def variant(type_id, variant_id, payload=None):
    data = {"ty": type_id, "variant": variant_id}
    if payload is not None:
        data["payload"] = payload
    return {"kind": "sum", "data": data}


def client_value(ids, major, minor, channel_symbol, transport_symbol, trusted):
    version = product(ids[3], [(ids[4], i64(major)), (ids[5], i64(minor))])
    return product(ids[14], [
        (ids[15], version),
        (ids[16], variant(ids[6], ids[channel_symbol])),
        (ids[17], variant(ids[10], ids[transport_symbol])),
        (ids[18], boolean(trusted)),
    ])


def policy_value(ids, minimum_major=1, minimum_minor=5, rollout_steps=4):
    return product(ids[19], [
        (ids[20], i64(minimum_major)),
        (ids[21], i64(minimum_minor)),
        (ids[22], i64(rollout_steps)),
    ])


def served(ids, weight):
    return variant(ids[28], ids[29], i64(weight))


def blocked(ids, reason_symbol):
    return variant(ids[28], ids[30], variant(ids[23], ids[reason_symbol]))


def run_case(workspace, revision, ids, name, client, policy, expected, fuel=100_000):
    result = expect(run_request(
        workspace, revision, ids[78], [client, policy], name, fuel=fuel
    ), "run")
    if result["value"] != expected:
        raise RuntimeError(f"unexpected run value for {name}: {result['value']}")
    return result


def normal_cases(ids):
    policy = policy_value(ids)
    return [
        ("stable_native", client_value(ids, 1, 5, 7, 11, False), policy, served(ids, 7)),
        ("beta_web", client_value(ids, 1, 5, 8, 12, False), policy, served(ids, 8)),
        ("trusted_nightly_native", client_value(ids, 1, 5, 9, 11, True), policy, served(ids, 9)),
        ("old_major", client_value(ids, 0, 99, 7, 11, False), policy, blocked(ids, 24)),
        ("old_minor", client_value(ids, 1, 4, 7, 11, False), policy, blocked(ids, 25)),
        ("unsupported_transport", client_value(ids, 1, 5, 7, 13, False), policy, blocked(ids, 26)),
        ("untrusted_nightly", client_value(ids, 1, 5, 9, 11, False), policy, blocked(ids, 27)),
    ]


def run_normal_cases(workspace, revision, ids, prefix):
    results = {}
    timings = []
    for name, client, policy, expected in normal_cases(ids):
        result = run_case(workspace, revision, ids, f"{prefix}_{name}", client, policy, expected)
        results[name] = result["value"]
        timings.append(result)
    return results, timings


def nominal_query(declaration):
    return {"kind": "nominal_type", "data": {"declaration": declaration, "page": {"limit": 32}}}


def member_name(result, member_id):
    for item in result["members"]["items"]:
        data = item.get("data", {})
        if data.get("field") == member_id or data.get("variant") == member_id:
            return data["name"]
    raise RuntimeError(f"member {member_id} missing from nominal type result")


def count_explicit_symbols(value):
    symbols = set()
    def visit(item):
        if isinstance(item, dict):
            for key in ("symbol", "index_symbol", "carried_symbol", "payload_symbol"):
                if key in item:
                    symbols.add(item[key])
            for child in item.values():
                visit(child)
        elif isinstance(item, list):
            for child in item:
                visit(child)
    visit(value)
    return len(symbols)


def measurement_summary():
    counted = [item for item in measurements if item["counted"]]
    discovery = [
        item for item in counted
        if item["request"]["request"].get("kind") == "describe_schema"
    ]
    workflow = [item for item in counted if item not in discovery]

    def totals(items):
        return {
            "json_request_bytes": sum(item["json_request_bytes"] for item in items),
            "json_response_bytes": sum(item["json_response_bytes"] for item in items),
            "cli_launches": len(items),
            "daemon_round_trips": len(items),
            "connections": len(items),
            "cli_daemon_wall_nanoseconds": sum(item["elapsed_nanoseconds"] for item in items),
        }

    summary = totals(workflow)
    summary.update({
        "discovery": totals(discovery),
        "total_excluding_lifecycle": totals(counted),
        "lifecycle_cli_launches": len(measurements) - len(counted),
        "daemon_processes": len(readiness_nanoseconds),
        "boundary_errors": 0,
    })
    return summary


def proposal_metrics():
    rows = []
    for item in measurements:
        request = item["request"]["request"]
        if request.get("kind") != "apply_transaction":
            continue
        operations = request["data"]["transaction"]["operations"]
        response = item["response"]["response"]
        returned = 0
        if response.get("kind") == "transaction_receipt":
            returned = len(response["data"]["returned_bindings"])
        rows.append({
            "purpose": item["purpose"],
            "mode": request["data"]["transaction"]["mode"],
            "operation_count": len(operations),
            "explicit_symbols": count_explicit_symbols(operations),
            "selected_bindings": len(request["data"]["response"]["return_symbols"]),
            "returned_bindings": returned,
            "json_request_bytes": item["json_request_bytes"],
            "outcome": response["data"].get("code", "accepted") if response.get("kind") == "error" else "accepted",
        })
    return rows


def historical_revision_view(workspace, revision, ids, purpose):
    nodes = [ids[key] for key in (1, 19, 22, 38, 40, 78)]
    outcomes = query_batch(workspace, revision, [
        {"kind": "workspace_summary"},
        *[
            {"kind": "node", "data": {"node": node, "expand": False}}
            for node in nodes
        ],
        nominal_query(ids[19]),
    ], purpose)
    workspace_view = expect(outcomes[0], "workspace_summary")
    if workspace_view["revision"] != revision:
        raise RuntimeError(f"workspace revision mismatch for historical revision {revision}")
    for node, outcome in zip(nodes, outcomes[1:-1]):
        view = expect(outcome, "node")
        if view["summary"]["node"] != node:
            raise RuntimeError(f"identity mismatch for {node} at revision {revision}")
    return workspace_view, expect(outcomes[-1], "nominal_type")


def execute():
    global state
    proposal = json.loads(PROPOSAL_PATH.read_text())
    operations = proposal["operations"]
    return_symbols = proposal["return_symbols"]
    if count_explicit_symbols(operations) != 111 or len(return_symbols) != 38:
        raise RuntimeError("sealed proposal symbol/binding counts changed")
    with tempfile.TemporaryDirectory(prefix="lkjscript-release-channel-") as directory:
        state = pathlib.Path(directory)
        os.chmod(state, 0o700)
        start_daemon()

        manifest = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "manifest"},
        }}, "schema_manifest"), "describe_schema"), "manifest")
        digest = manifest["digest"]
        roots = [
            "create_workspace", "apply_transaction", "query_workspace_summary", "query_node",
            "query_blockers", "query_body", "query_incoming_uses", "query_repair_context",
            "query_semantic_diff", "query_nominal_type", "run", "shutdown",
        ]
        task_contract = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "roots", "data": {"roots": roots}},
        }}, "task_contract_roots"), "describe_schema"), "roots")
        full_contract = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "full"},
        }}, "full_contract"), "describe_schema"), "full")
        unchanged = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "full"}, "known_digest": digest,
        }}, "known_digest_unchanged"), "describe_schema"), "unchanged")
        if task_contract["digest"] != digest or full_contract["digest"] != digest or unchanged["digest"] != digest:
            raise RuntimeError("machine contract digest mismatch")

        created = expect(rpc({"kind": "create_workspace"}, "workspace_creation"), "workspace_created")
        workspace = created["workspace"]
        creation = expect(rpc(apply_request(
            workspace, 0, "commit", operations, return_symbols
        ), "release_policy_incomplete_creation"), "transaction_receipt")
        ids = {
            int(symbol.removeprefix("draft_")): node
            for symbol, node in creation["returned_bindings"]
        }
        expected_symbols = {int(symbol.removeprefix("draft_")) for symbol in return_symbols}
        if set(ids) != expected_symbols or creation["revision"] != 1 or creation["complete_after"]:
            raise RuntimeError("initial incomplete revision/bindings mismatch")

        context = expect(query(workspace, 1, {"kind": "repair_context", "data": {
            "target": {"kind": "hole", "data": ids[40]},
            "budget": {
                "body_before": 4, "body_after": 4, "visible_values": 8,
                "incoming_uses": 8, "include_incompatible": True,
            },
        }}, "rollout_weight_repair_context"), "repair_context")
        if context["operation"] != ids[40] or context["expected_type"] != "i64":
            raise RuntimeError("repair context target/type mismatch")
        compatible = [item for item in context["visible_values"]["items"] if item["compatible"]]
        if len(compatible) != 1 or compatible[0]["ty"] != "i64":
            raise RuntimeError("repair context must expose exactly the rollout steps parameter")
        parameter_id = compatible[0]["producer"]
        before_location = (context["owner_block"], context["owner_function"], context["ordinal"])
        before_uses = context["incoming_uses"]["items"]
        if (
            len(before_uses) != 1
            or before_uses[0]["target"] != {
                "kind": "operation_result",
                "data": {"operation": ids[40], "output": 0},
            }
        ):
            raise RuntimeError("placeholder must have exactly one output-zero use site")

        probe_operation = [{"kind": "create_package", "data": {
            "symbol": "allocation_probe", "name": "allocation_probe",
        }}]
        probe_before = expect(rpc(apply_request(
            workspace, 1, "validate_only", probe_operation, ["allocation_probe"]
        ), "allocation_probe_before_invalid"), "transaction_receipt")
        invalid_operation = [{"kind": "refine_hole", "data": {
            "hole": existing(ids[40]),
            "replacement": {"kind": "construct_variant", "data": {
                "variant": existing(ids[29]),
                "payload": function_parameter(parameter_id),
            }},
        }}]
        invalid = expect_error(rpc(apply_request(
            workspace, 1, "commit", invalid_operation
        ), "invalid_rollout_weight_repair"), "type_mismatch", ids[40])
        probe_after = expect(rpc(apply_request(
            workspace, 1, "validate_only", probe_operation, ["allocation_probe"]
        ), "allocation_probe_after_invalid"), "transaction_receipt")
        for field in ("revision", "hash", "created_count", "returned_bindings"):
            if probe_before[field] != probe_after[field]:
                raise RuntimeError("invalid repair changed publication or identity allocation")
        summary_one = expect(query(workspace, 1, {"kind": "workspace_summary"},
                                   "summary_after_invalid_repair"), "workspace_summary")
        if summary_one["revision"] != 1 or summary_one["complete"]:
            raise RuntimeError("invalid repair published or completed revision one")

        valid_operation = [{"kind": "refine_hole", "data": {
            "hole": existing(ids[40]),
            "replacement": {"kind": "call", "data": {
                "function": existing(ids[31]),
                "arguments": [function_parameter(parameter_id)],
            }},
        }}]
        repaired = expect(rpc(apply_request(
            workspace, 1, "commit", valid_operation
        ), "valid_rollout_weight_repair"), "transaction_receipt")
        if repaired["revision"] != 2 or repaired["created_count"] != 0 or not repaired["complete_after"]:
            raise RuntimeError("valid repair publication/identity mismatch")

        post = query_batch(workspace, 2, [
            {"kind": "node", "data": {"node": ids[40], "expand": True}},
            {"kind": "body", "data": {"block": before_location[0], "page": {"limit": 16}}},
            {"kind": "incoming_uses", "data": {
                "value": {"kind": "operation_result", "data": {"operation": ids[40], "output": 0}},
                "page": {"limit": 16},
            }},
            {"kind": "workspace_summary"},
            {"kind": "node", "data": {"node": before_location[0], "expand": False}},
        ], "post_repair_identity")
        repaired_node = expect(post[0], "node")
        repaired_body = expect(post[1], "body")
        repaired_uses = expect(post[2], "incoming_uses")
        summary_two = expect(post[3], "workspace_summary")
        owner_block = expect(post[4], "node")
        owner_region = expect(query(
            workspace, 2,
            {"kind": "node", "data": {
                "node": owner_block["summary"]["owner"], "expand": False,
            }},
            "post_repair_owner_region",
        ), "node")
        if (
            repaired_node["summary"]["node"] != ids[40]
            or repaired_node["summary"]["owner"] != before_location[0]
            or owner_block["summary"]["node"] != before_location[0]
            or owner_region["summary"]["owner"] != before_location[1]
            or before_location[1] != ids[38]
        ):
            raise RuntimeError("repair changed placeholder identity or transitive owner")
        body_item = next((item for item in repaired_body["items"] if item["operation"] == ids[40]), None)
        if body_item is None or body_item["ordinal"] != before_location[2] or body_item["code"] != "call":
            raise RuntimeError("repair changed body position or did not install call")
        if repaired_uses["items"] != before_uses:
            raise RuntimeError("repair changed placeholder output use sites")

        diff = expect(query(workspace, 2, {"kind": "semantic_diff", "data": {
            "from": 1, "page": {"limit": 16},
        }}, "repair_semantic_diff"), "semantic_diff")
        diff_items = diff["page"]["items"]
        target_kinds = sorted(item["kind"]["kind"] for item in diff_items if item["node"] == ids[40])
        if (
            diff["change_count"] != 3 or diff["page"].get("total") != 3
            or "next" in diff["page"]
            or target_kinds != ["operand_changed", "operation_refined"]
            or any(item["kind"]["kind"] in ("created", "deleted") for item in diff_items)
        ):
            raise RuntimeError(f"unexpected identity-preserving repair diff: {diff}")

        run_results, run_timings = run_normal_cases(workspace, 2, ids, "revision_two")
        huge_policy = policy_value(ids, rollout_steps=1_000_000)
        lazy = run_case(
            workspace, 2, ids, "lazy_unsupported_transport",
            client_value(ids, 1, 5, 7, 13, False), huge_policy, blocked(ids, 26), fuel=500,
        )
        exhausted = expect_error(run_request(
            workspace, 2, ids[78],
            [client_value(ids, 1, 5, 7, 11, False), huge_policy],
            "selected_expensive_rollout", fuel=500,
        ), "execution_fuel_exhausted")

        old_policy = expect(query(workspace, 2, nominal_query(ids[19]),
                                  "revision_two_policy_name"), "nominal_type")
        if member_name(old_policy, ids[22]) != "rollout_steps":
            raise RuntimeError("revision two lost rollout_steps name")
        renamed = expect(rpc(apply_request(workspace, 2, "commit", [{
            "kind": "rename_node", "data": {"node": existing(ids[22]), "name": "steps"},
        }]), "rename_rollout_steps"), "transaction_receipt")
        if renamed["revision"] != 3 or renamed["created_count"] != 0:
            raise RuntimeError("rename allocated identity or published wrong revision")
        rename_results = query_batch(workspace, 3, [
            {"kind": "semantic_diff", "data": {"from": 2, "page": {"limit": 8}}},
            nominal_query(ids[19]),
        ], "rename_diff_and_name")
        rename_diff = expect(rename_results[0], "semantic_diff")
        new_policy = expect(rename_results[1], "nominal_type")
        if (
            rename_diff["change_count"] != 1
            or rename_diff["page"].get("total") != 1
            or rename_diff["page"]["items"][0]["node"] != ids[22]
            or rename_diff["page"]["items"][0]["kind"] != {
                "kind": "renamed", "data": {"before": "rollout_steps", "after": "steps"},
            }
            or member_name(new_policy, ids[22]) != "steps"
        ):
            raise RuntimeError("rename history/diff mismatch")

        stop_daemon("shutdown_before_restart")
        start_daemon()
        historical_views = [
            historical_revision_view(
                workspace, revision, ids, f"restart_revision_{revision}_history"
            )
            for revision in (1, 2, 3)
        ]
        completeness = [view[0]["complete"] for view in historical_views]
        names = [member_name(view[1], ids[22]) for view in historical_views]
        if completeness != [False, True, True]:
            raise RuntimeError(f"historical completeness changed across restart: {completeness}")
        if names != ["rollout_steps", "rollout_steps", "steps"]:
            raise RuntimeError(f"historical names changed across restart: {names}")

        expect_error(run_request(
            workspace, 1, ids[78], normal_cases(ids)[0][1:3], "restart_incomplete_revision"
        ), "compile_incomplete")
        beta_case = normal_cases(ids)[1]
        restart_two = run_case(
            workspace, 2, ids, "restart_repaired_beta_web", beta_case[1], beta_case[2], beta_case[3]
        )
        restart_three = run_case(
            workspace, 3, ids, "restart_renamed_beta_web", beta_case[1], beta_case[2], beta_case[3]
        )
        if restart_two["value"] != restart_three["value"]:
            raise RuntimeError("revisions two and three execute differently after restart")

        workspace_directory = state / "workspaces" / workspace
        artifact_sizes = {
            str(revision): (workspace_directory / "revisions" / f"{revision:020d}.lkjscript").stat().st_size
            for revision in (1, 2, 3)
        }
        head_size = (workspace_directory / "HEAD").stat().st_size
        stop_daemon("final_shutdown")

        contract_rows = {item["purpose"]: item for item in measurements}
        proposals = proposal_metrics()
        counted = measurement_summary()
        summary = {
            "workspace": workspace,
            "revisions": {"incomplete": 1, "repaired": 2, "renamed": 3},
            "ids": {
                "package": ids[1], "policy_type": ids[19], "rollout_steps": ids[22],
                "rollout_weight": ids[38], "placeholder": ids[40], "entry": ids[78],
            },
            "hashes": {"incomplete": creation["hash"], "repaired": repaired["hash"], "renamed": renamed["hash"]},
            "run_results": run_results,
            "repair": {
                "rejected_code": invalid["code"], "allocator_rollback": True,
                "identity_preserved": True, "owner_preserved": True, "body_position_preserved": True,
                "output_zero_preserved": True, "use_sites_preserved": True,
                "change": "operation_refined",
            },
            "history": {"names": names, "rename_identity_preserved": True, "rename_diff_exact": True},
            "laziness": {
                "unselected_expensive_branch": lazy["value"],
                "selected_expensive_branch_error": exhausted["code"],
                "fuel": 500,
            },
            "restart": {
                "revisions_queried": [1, 2, 3], "revision_one_incomplete": True,
                "revisions_two_three_equal": True, "identities_persisted": True,
            },
            "contracts": {
                "digest": digest, "task_roots": len(roots),
                "task_definitions": len(task_contract["definitions"]),
                "manifest_response_bytes": contract_rows["schema_manifest"]["json_response_bytes"],
                "task_response_bytes": contract_rows["task_contract_roots"]["json_response_bytes"],
                "full_response_bytes": contract_rows["full_contract"]["json_response_bytes"],
                "unchanged_response_bytes": contract_rows["known_digest_unchanged"]["json_response_bytes"],
            },
            "proposals": {
                "fixture_bytes": PROPOSAL_PATH.stat().st_size,
                "initial_compact_payload_bytes": len(json.dumps(proposal, separators=(",", ":")).encode()),
                "rows": proposals,
            },
            "counts": {
                "initial_operations": len(operations), "explicit_draft_symbols": 111,
                "selected_bindings": 38, "created_nodes": creation["created_count"],
                "canonical_nodes": summary_two["node_count"], "rejected_proposals": 1,
            },
            "interaction": counted,
            "artifacts": {"revision_bytes": artifact_sizes, "head_bytes": head_size},
            "timings": {
                "cold_readiness_nanoseconds": readiness_nanoseconds[0],
                "restart_readiness_nanoseconds": readiness_nanoseconds[1],
                "normal_compile_nanoseconds": sum(item["compile_nanoseconds"] for item in run_timings),
                "normal_execute_nanoseconds": sum(item["execute_nanoseconds"] for item in run_timings),
                "restart_revision_two_compile_nanoseconds": restart_two["compile_nanoseconds"],
                "restart_revision_three_compile_nanoseconds": restart_three["compile_nanoseconds"],
            },
            "provider_telemetry": {"available": False},
            "shutdown": "acknowledged",
        }
        if counted["lifecycle_cli_launches"] != 2 or counted["daemon_processes"] != 2:
            raise RuntimeError("workflow process counts changed")
        if METRICS_PATH is not None:
            METRICS_PATH.write_text(json.dumps({
                "summary": summary, "measurements": measurements,
            }, separators=(",", ":")) + "\n")
        return summary


def main():
    global daemon
    try:
        summary = execute()
    finally:
        if daemon is not None and daemon.poll() is None:
            daemon.kill()
            daemon.wait(timeout=5)
            daemon = None
    print(json.dumps(summary, separators=(",", ":")))


if __name__ == "__main__":
    main()
