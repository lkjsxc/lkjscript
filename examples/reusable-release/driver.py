#!/usr/bin/env python3
"""Prove exact reusable-release composition through public lkjscript commands."""

import base64
import copy
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time


ROOT = pathlib.Path(__file__).resolve().parents[2]
CLI = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/release/lkjscript"
session = None
request_id = 0
state = None
rpc_measurements = []
command_measurements = []


def symbol(number):
    return f"draft_{number}"


def local(number):
    return {"kind": "draft", "data": symbol(number)}


def nominal(number):
    return {"nominal": local(number)}


def parameter(number):
    return {"kind": "function_parameter", "data": local(number)}


def result(number):
    return {
        "kind": "operation_result",
        "data": {"operation": local(number), "output": 0},
    }


def expression(number, kind, data=None):
    operation = {"kind": kind}
    if data is not None:
        operation["data"] = data
    return {"symbol": symbol(number), "operation": operation}


def function(number, name, parameters, result_type, operations=None, return_value=None):
    body = None
    if operations is not None:
        body = {"operations": operations, "return_value": return_value}
    return {
        "kind": "create_function",
        "data": {
            "symbol": symbol(number),
            "module": local(2),
            "name": name,
            "parameters": [
                {**item, "symbol": symbol(item["symbol"])} for item in parameters
            ],
            "result": result_type,
            "body": body,
        },
    }


def call(number, target, arguments):
    return expression(
        number,
        "call",
        {"function": local(target), "arguments": arguments},
    )


def bytes_value(value):
    encoded = base64.urlsafe_b64encode(bytes(value)).rstrip(b"=").decode("ascii")
    return {"kind": "bytes", "data": encoded}


def i64_value(value):
    return {"kind": "i64", "data": value}


def target(release, item):
    return {"release": release, "item": item}


def product_value(release, ty, field, payload):
    return {
        "kind": "product",
        "data": {
            "ty": target(release, ty),
            "fields": [
                {
                    "field": target(release, field),
                    "value": bytes_value(payload),
                }
            ],
        },
    }


def run_process(arguments, input_bytes=b"", expected=0):
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), *arguments],
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        check=False,
    )
    command_measurements.append(
        {
            "command": " ".join(arguments[:2]),
            "input_bytes": len(input_bytes),
            "output_bytes": len(completed.stdout),
            "diagnostic_bytes": len(completed.stderr),
            "elapsed_nanoseconds": time.monotonic_ns() - started,
            "exit": completed.returncode,
        }
    )
    if completed.returncode != expected:
        raise RuntimeError(
            f"command {arguments} returned {completed.returncode}, expected {expected}: "
            f"stdout={completed.stdout!r} stderr={completed.stderr!r}"
        )
    return completed


def json_command(arguments, value=None, expected=0):
    encoded = b"" if value is None else json.dumps(value, separators=(",", ":")).encode()
    completed = run_process(arguments, encoded, expected)
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"command {arguments} returned invalid JSON") from error


def expect_failure(arguments, value, code):
    encoded = json.dumps(value, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), *arguments],
        input=encoded,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        check=False,
    )
    command_measurements.append(
        {
            "command": " ".join(arguments[:2]),
            "input_bytes": len(encoded),
            "output_bytes": len(completed.stdout),
            "diagnostic_bytes": len(completed.stderr),
            "elapsed_nanoseconds": time.monotonic_ns() - started,
            "exit": completed.returncode,
        }
    )
    if completed.returncode == 0:
        raise RuntimeError(f"command {arguments} unexpectedly succeeded")
    failure = json.loads(completed.stdout)
    observed = failure.get("error", {}).get("code")
    if observed != code:
        raise RuntimeError(f"expected {code}, received {failure}")
    return failure


def start_session():
    global session
    session = subprocess.Popen(
        [str(CLI), "--state", str(state), "session"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    manifest = expect(
        expect(
            rpc(
                {
                    "kind": "describe_schema",
                    "data": {"projection": {"kind": "manifest"}},
                },
                "schema_manifest",
            ),
            "describe_schema",
        ),
        "manifest",
    )
    if manifest["protocol_version"] != 10:
        raise RuntimeError("unexpected protocol version")


def stop_session():
    global session
    if session is None:
        return
    session.stdin.close()
    if session.wait(timeout=5) != 0:
        raise RuntimeError("line session failed during shutdown")
    diagnostic = session.stderr.read()
    if diagnostic:
        raise RuntimeError(f"line session wrote diagnostics: {diagnostic!r}")
    session = None


def rpc(request, purpose):
    global request_id
    request_id += 1
    envelope = {"version": 10, "request_id": request_id, "request": request}
    encoded = json.dumps(envelope, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    session.stdin.write(encoded + b"\n")
    session.stdin.flush()
    response_bytes = session.stdout.readline()
    rpc_measurements.append(
        {
            "purpose": purpose,
            "request_bytes": len(encoded),
            "response_bytes": len(response_bytes),
            "elapsed_nanoseconds": time.monotonic_ns() - started,
        }
    )
    if not response_bytes:
        raise RuntimeError(f"line session ended during {purpose}")
    response = json.loads(response_bytes)
    if response.get("version") != 10 or response.get("request_id") != request_id:
        raise RuntimeError(f"RPC correlation failed during {purpose}")
    return response["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response.get("data")


def create_workspace(operations, returned, purpose):
    workspace = expect(rpc({"kind": "create_workspace"}, f"{purpose}_workspace"), "workspace_created")[
        "workspace"
    ]
    request = {
        "kind": "apply_transaction",
        "data": {
            "transaction": {
                "workspace": workspace,
                "base_revision": 0,
                "mode": "commit",
                "operations": operations,
            },
            "response": {"return_symbols": [symbol(number) for number in returned]},
        },
    }
    receipt = expect(rpc(request, f"{purpose}_author"), "transaction_receipt")
    ids = {
        int(name.removeprefix("draft_")): node
        for name, node in receipt["returned_bindings"]
    }
    if set(ids) != set(returned):
        raise RuntimeError(f"{purpose} returned incomplete bindings")
    return workspace, receipt["revision"], ids


def producer_operations(reverse=False, noise=False, doubled=False):
    prefix = []
    if noise:
        prefix = [
            {
                "kind": "create_package",
                "data": {"symbol": symbol(900), "name": "discarded-noise"},
            },
            {
                "kind": "create_module",
                "data": {
                    "symbol": symbol(901),
                    "package": local(900),
                    "name": "unused",
                },
            },
        ]
    private = function(
        6,
        "private_identity",
        [{"symbol": 7, "name": "input", "ty": "bytes"}],
        "bytes",
        [],
        parameter(7),
    )
    normalize_operations = [call(81, 6, [parameter(9)])]
    normalize_result = result(81)
    if doubled:
        normalize_operations.append(
            expression(82, "bytes_concat", {"lhs": result(81), "rhs": result(81)})
        )
        normalize_result = result(82)
    normalize = function(
        8,
        "normalize",
        [{"symbol": 9, "name": "input", "ty": "bytes"}],
        "bytes",
        normalize_operations,
        normalize_result,
    )
    inspect = function(
        10,
        "inspect_length",
        [{"symbol": 11, "name": "input", "ty": "bytes"}],
        "i64",
        [expression(101, "bytes_len", {"value": parameter(11)})],
        result(101),
    )
    echo = function(
        12,
        "echo_frame",
        [{"symbol": 13, "name": "frame", "ty": nominal(4)}],
        nominal(4),
        [],
        parameter(13),
    )
    functions = [private, normalize, inspect, echo]
    if reverse:
        functions.reverse()
    return prefix + [
        {
            "kind": "create_package",
            "data": {"symbol": symbol(1), "name": "shared-codec"},
        },
        {
            "kind": "create_module",
            "data": {"symbol": symbol(2), "package": local(1), "name": "codec"},
        },
        {
            "kind": "create_product_type",
            "data": {
                "symbol": symbol(4),
                "module": local(2),
                "name": "Frame",
                "fields": [
                    {
                        "symbol": symbol(5),
                        "name": "payload",
                        "ty": "bytes",
                    }
                ],
            },
        },
        *functions,
        {
            "kind": "set_entry_function",
            "data": {"package": local(1), "function": local(8)},
        },
    ]


def producer_request(workspace, revision, ids, version, doubled=False):
    expected = b"abcabc" if doubled else b"abc"
    return {
        "version": 1,
        "workspace": workspace,
        "revision": revision,
        "root": ids[1],
        "coordinate": "examples/shared-codec",
        "user_version": version,
        "exports": [
            {"name": "frame", "target": ids[4]},
            {"name": "normalize", "target": ids[8]},
            {"name": "inspect_length", "target": ids[10]},
            {"name": "echo_frame", "target": ids[12]},
        ],
        "dependencies": [],
        "imports": [],
        "tests": [
            {
                "name": "normalize_bytes",
                "target": ids[8],
                "arguments": [bytes_value(b"abc")],
                "expected": {"kind": "value", "data": bytes_value(expected)},
                "policy": {"fuel": 1000, "maximum_frames": 32},
            },
            {
                "name": "inspect_bytes",
                "target": ids[10],
                "arguments": [bytes_value(b"abc")],
                "expected": {"kind": "value", "data": i64_value(3)},
                "policy": {"fuel": 1000, "maximum_frames": 32},
            },
        ],
    }


def normalizer_operations():
    return [
        {
            "kind": "create_package",
            "data": {"symbol": symbol(1), "name": "consumer-normalizer"},
        },
        {
            "kind": "create_module",
            "data": {"symbol": symbol(2), "package": local(1), "name": "main"},
        },
        {
            "kind": "create_product_type",
            "data": {
                "symbol": symbol(4),
                "module": local(2),
                "name": "SharedFrame",
                "fields": [
                    {"symbol": symbol(5), "name": "payload", "ty": "bytes"}
                ],
            },
        },
        function(6, "shared_normalize", [{"symbol": 7, "name": "input", "ty": "bytes"}], "bytes"),
        function(8, "shared_echo", [{"symbol": 9, "name": "frame", "ty": nominal(4)}], nominal(4)),
        function(
            10,
            "entry",
            [{"symbol": 11, "name": "input", "ty": "bytes"}],
            "bytes",
            [call(101, 6, [parameter(11)])],
            result(101),
        ),
        function(
            12,
            "echo_entry",
            [{"symbol": 13, "name": "frame", "ty": nominal(4)}],
            nominal(4),
            [call(121, 8, [parameter(13)])],
            result(121),
        ),
        {
            "kind": "set_entry_function",
            "data": {"package": local(1), "function": local(10)},
        },
    ]


def inspector_operations():
    return [
        {
            "kind": "create_package",
            "data": {"symbol": symbol(1), "name": "consumer-inspector"},
        },
        {
            "kind": "create_module",
            "data": {"symbol": symbol(2), "package": local(1), "name": "main"},
        },
        function(4, "shared_inspect", [{"symbol": 5, "name": "input", "ty": "bytes"}], "i64"),
        function(
            6,
            "entry",
            [{"symbol": 7, "name": "input", "ty": "bytes"}],
            "i64",
            [call(61, 4, [parameter(7)])],
            result(61),
        ),
        {
            "kind": "set_entry_function",
            "data": {"package": local(1), "function": local(6)},
        },
    ]


def coexistence_operations():
    return [
        {
            "kind": "create_package",
            "data": {"symbol": symbol(1), "name": "release-version-coexistence"},
        },
        {
            "kind": "create_module",
            "data": {"symbol": symbol(2), "package": local(1), "name": "main"},
        },
        {
            "kind": "create_product_type",
            "data": {
                "symbol": symbol(4),
                "module": local(2),
                "name": "FrameR1",
                "fields": [{"symbol": symbol(5), "name": "payload", "ty": "bytes"}],
            },
        },
        {
            "kind": "create_product_type",
            "data": {
                "symbol": symbol(6),
                "module": local(2),
                "name": "FrameR2",
                "fields": [{"symbol": symbol(7), "name": "payload", "ty": "bytes"}],
            },
        },
        function(8, "normalize_r1", [{"symbol": 9, "name": "input", "ty": "bytes"}], "bytes"),
        function(10, "normalize_r2", [{"symbol": 11, "name": "input", "ty": "bytes"}], "bytes"),
        function(
            12,
            "compose",
            [{"symbol": 13, "name": "input", "ty": "bytes"}],
            "bytes",
            [call(121, 8, [parameter(13)]), call(122, 10, [result(121)])],
            result(122),
        ),
        function(
            14,
            "choose_r1",
            [
                {"symbol": 15, "name": "r1", "ty": nominal(4)},
                {"symbol": 16, "name": "r2", "ty": nominal(6)},
            ],
            nominal(4),
            [],
            parameter(15),
        ),
        {
            "kind": "set_entry_function",
            "data": {"package": local(1), "function": local(12)},
        },
    ]


def diamond_operations():
    return [
        {
            "kind": "create_package",
            "data": {"symbol": symbol(1), "name": "release-diamond"},
        },
        {
            "kind": "create_module",
            "data": {"symbol": symbol(2), "package": local(1), "name": "main"},
        },
        function(4, "left_entry", [{"symbol": 5, "name": "input", "ty": "bytes"}], "bytes"),
        function(6, "right_entry", [{"symbol": 7, "name": "input", "ty": "bytes"}], "i64"),
        function(
            8,
            "entry",
            [{"symbol": 9, "name": "input", "ty": "bytes"}],
            "bytes",
            [call(81, 4, [parameter(9)]), call(82, 6, [result(81)])],
            result(81),
        ),
        {
            "kind": "set_entry_function",
            "data": {"package": local(1), "function": local(8)},
        },
    ]


def build_release(path, request, dependencies):
    arguments = ["release", "build", "--state", str(state)]
    for dependency in dependencies:
        arguments.extend(["--dependency", str(dependency)])
    arguments.extend(["--output", str(path)])
    return json_command(arguments, request)


def release_exports(receipt):
    return {item["name"]: item for item in receipt["inspection"]["exports"]}


def build_application(path, request, releases):
    arguments = ["app", "build"]
    for release in releases:
        arguments.extend(["--release", str(release)])
    arguments.extend(["--output", str(path)])
    return json_command(arguments, request)


def application_request(root_release, entry_item, profile, arguments, expected):
    entry = target(root_release, entry_item)
    return {
        "version": 4,
        "root_release": root_release,
        "entry": entry,
        "profile": {"kind": profile},
        "policy": {"fuel": 10000, "maximum_frames": 64},
        "tests": [
            {
                "name": "public_entry",
                "target": entry,
                "arguments": arguments,
                "expected": {"kind": "value", "data": expected},
                "policy": {"fuel": 10000, "maximum_frames": 64},
            }
        ],
    }


def invocation(arguments):
    return {"version": 4, "arguments": arguments}


def release_test(path, dependencies):
    arguments = ["release", "test", "--artifact", str(path)]
    for dependency in dependencies:
        arguments.extend(["--dependency", str(dependency)])
    report = json_command(arguments)
    if report["report"]["total"] != report["report"]["passed"]:
        raise RuntimeError(f"release tests failed: {report}")


def workflow():
    global state
    with tempfile.TemporaryDirectory(prefix="lkjscript-reusable-release-") as directory:
        root = pathlib.Path(directory)
        state = root / "state"
        state.mkdir()
        os.chmod(state, 0o700)
        start_session()

        producer_symbols = [1, 4, 5, 6, 8, 10, 12]
        p1_workspace, p1_revision, p1 = create_workspace(
            producer_operations(), producer_symbols, "producer_r1"
        )
        p1_request = producer_request(p1_workspace, p1_revision, p1, "1.0.0")
        r1_path = root / "shared-codec-r1.lkjr"

        p1b_workspace, p1b_revision, p1b = create_workspace(
            producer_operations(reverse=True, noise=True), producer_symbols, "producer_r1_rebuilt"
        )
        p1b_request = producer_request(p1b_workspace, p1b_revision, p1b, "1.0.0")
        r1_rebuilt_path = root / "shared-codec-r1-rebuilt.lkjr"

        p2_workspace, p2_revision, p2 = create_workspace(
            producer_operations(doubled=True), producer_symbols, "producer_r2"
        )
        p2_request = producer_request(p2_workspace, p2_revision, p2, "2.0.0", doubled=True)
        r2_path = root / "shared-codec-r2.lkjr"

        a_workspace, a_revision, a = create_workspace(
            normalizer_operations(), [1, 4, 5, 6, 8, 10, 12], "consumer_normalizer"
        )
        b_workspace, b_revision, b = create_workspace(
            inspector_operations(), [1, 4, 6], "consumer_inspector"
        )
        c_workspace, c_revision, c = create_workspace(
            coexistence_operations(), [1, 4, 5, 6, 7, 8, 10, 12, 14], "coexistence"
        )
        d_workspace, d_revision, d = create_workspace(
            diamond_operations(), [1, 4, 6, 8], "diamond"
        )
        stop_session()

        r1 = build_release(r1_path, p1_request, [])
        r1_rebuilt = build_release(r1_rebuilt_path, p1b_request, [])
        if r1_path.read_bytes() != r1_rebuilt_path.read_bytes():
            raise RuntimeError("equivalent producer histories did not produce canonical equal releases")
        if r1["inspection"]["release"] != r1_rebuilt["inspection"]["release"]:
            raise RuntimeError("equivalent producer histories produced different exact identities")
        r2 = build_release(r2_path, p2_request, [])
        r1_id = r1["inspection"]["release"]
        r2_id = r2["inspection"]["release"]
        if r1_id == r2_id or r1["inspection"]["coordinate"] != r2["inspection"]["coordinate"]:
            raise RuntimeError("R1/R2 coordinate coexistence is malformed")

        a_request = {
            "version": 1,
            "workspace": a_workspace,
            "revision": a_revision,
            "root": a[1],
            "coordinate": "examples/consumer-normalizer",
            "user_version": "1.0.0",
            "exports": [
                {"name": "entry", "target": a[10]},
                {"name": "echo_frame", "target": a[12]},
            ],
            "dependencies": [{"slot": "shared", "release": r1_id}],
            "imports": [
                {"local": a[4], "dependency_slot": "shared", "export": "frame"},
                {"local": a[6], "dependency_slot": "shared", "export": "normalize"},
                {"local": a[8], "dependency_slot": "shared", "export": "echo_frame"},
            ],
            "tests": [
                {
                    "name": "normalizes",
                    "target": a[10],
                    "arguments": [bytes_value(b"abc")],
                    "expected": {"kind": "value", "data": bytes_value(b"abc")},
                    "policy": {"fuel": 1000, "maximum_frames": 32},
                }
            ],
        }
        private_request = copy.deepcopy(a_request)
        private_request["imports"][1]["export"] = "private_identity"
        private_args = [
            "release",
            "build",
            "--state",
            str(state),
            "--dependency",
            str(r1_path),
            "--validate-only",
        ]
        expect_failure(private_args, private_request, "node_not_found")
        a_path = root / "consumer-normalizer.lkjr"
        a_release = build_release(a_path, a_request, [r1_path])
        a_id = a_release["inspection"]["release"]

        b_request = {
            "version": 1,
            "workspace": b_workspace,
            "revision": b_revision,
            "root": b[1],
            "coordinate": "examples/consumer-inspector",
            "user_version": "1.0.0",
            "exports": [{"name": "entry", "target": b[6]}],
            "dependencies": [{"slot": "shared", "release": r1_id}],
            "imports": [
                {"local": b[4], "dependency_slot": "shared", "export": "inspect_length"}
            ],
            "tests": [
                {
                    "name": "inspects",
                    "target": b[6],
                    "arguments": [bytes_value(b"abc")],
                    "expected": {"kind": "value", "data": i64_value(3)},
                    "policy": {"fuel": 1000, "maximum_frames": 32},
                }
            ],
        }
        b_path = root / "consumer-inspector.lkjr"
        b_release = build_release(b_path, b_request, [r1_path])
        b_id = b_release["inspection"]["release"]

        c_request = {
            "version": 1,
            "workspace": c_workspace,
            "revision": c_revision,
            "root": c[1],
            "coordinate": "examples/release-version-coexistence",
            "user_version": "1.0.0",
            "exports": [
                {"name": "choose_r1", "target": c[14]},
                {"name": "compose", "target": c[12]},
            ],
            "dependencies": [
                {"slot": "r1", "release": r1_id},
                {"slot": "r2", "release": r2_id},
            ],
            "imports": [
                {"local": c[4], "dependency_slot": "r1", "export": "frame"},
                {"local": c[6], "dependency_slot": "r2", "export": "frame"},
                {"local": c[8], "dependency_slot": "r1", "export": "normalize"},
                {"local": c[10], "dependency_slot": "r2", "export": "normalize"},
            ],
            "tests": [
                {
                    "name": "both_versions",
                    "target": c[12],
                    "arguments": [bytes_value(b"abc")],
                    "expected": {"kind": "value", "data": bytes_value(b"abcabc")},
                    "policy": {"fuel": 1000, "maximum_frames": 32},
                }
            ],
        }
        c_path = root / "release-version-coexistence.lkjr"
        c_release = build_release(c_path, c_request, [r2_path, r1_path])
        c_id = c_release["inspection"]["release"]

        d_request = {
            "version": 1,
            "workspace": d_workspace,
            "revision": d_revision,
            "root": d[1],
            "coordinate": "examples/release-diamond",
            "user_version": "1.0.0",
            "exports": [{"name": "entry", "target": d[8]}],
            "dependencies": [
                {"slot": "left", "release": a_id},
                {"slot": "right", "release": b_id},
            ],
            "imports": [
                {"local": d[4], "dependency_slot": "left", "export": "entry"},
                {"local": d[6], "dependency_slot": "right", "export": "entry"},
            ],
            "tests": [
                {
                    "name": "shared_diamond",
                    "target": d[8],
                    "arguments": [bytes_value(b"abc")],
                    "expected": {"kind": "value", "data": bytes_value(b"abc")},
                    "policy": {"fuel": 1000, "maximum_frames": 32},
                }
            ],
        }
        d_path = root / "release-diamond.lkjr"
        d_release = build_release(d_path, d_request, [b_path, r1_path, a_path])
        d_id = d_release["inspection"]["release"]

        r1_exports = release_exports(r1)
        r2_exports = release_exports(r2)
        r1_frame = r1_exports["frame"]
        r2_frame = r2_exports["frame"]
        r1_value = product_value(
            r1_id,
            r1_frame["target"],
            r1_frame["signature"]["data"]["fields"][0]["target"],
            b"one",
        )
        r2_value = product_value(
            r2_id,
            r2_frame["target"],
            r2_frame["signature"]["data"]["fields"][0]["target"],
            b"two",
        )

        a_entry = release_exports(a_release)["entry"]["target"]
        b_entry = release_exports(b_release)["entry"]["target"]
        c_entry = release_exports(c_release)["choose_r1"]["target"]
        d_entry = release_exports(d_release)["entry"]["target"]
        application_specs = {
            "consumer-normalizer": (
                a_id,
                a_entry,
                "bytes_stream",
                [bytes_value(b"abc")],
                bytes_value(b"abc"),
                [a_path, r1_path],
            ),
            "consumer-inspector": (
                b_id,
                b_entry,
                "typed",
                [bytes_value(b"abc")],
                i64_value(3),
                [b_path, r1_path],
            ),
            "release-version-coexistence": (
                c_id,
                c_entry,
                "typed",
                [r1_value, r2_value],
                r1_value,
                [c_path, r1_path, r2_path],
            ),
            "release-diamond": (
                d_id,
                d_entry,
                "bytes_stream",
                [bytes_value(b"abc")],
                bytes_value(b"abc"),
                [d_path, a_path, b_path, r1_path],
            ),
        }
        applications = {}
        requests = {}
        for name, spec in application_specs.items():
            root_id, entry_item, profile, arguments, expected, releases = spec
            request = application_request(root_id, entry_item, profile, arguments, expected)
            path = root / f"{name}.lkja"
            receipt = build_application(path, request, list(reversed(releases)))
            applications[name] = (path, receipt, releases)
            requests[name] = request

        diamond_path, diamond_receipt, diamond_releases = applications["release-diamond"]
        if (
            len(diamond_receipt["inspection"]["releases"]) != 4
            or diamond_receipt["inspection"]["graph_edges"] != 4
            or diamond_receipt["inspection"]["graph_depth"] != 3
            or sum(
                item["release"] == r1_id
                for item in diamond_receipt["inspection"]["releases"]
            )
            != 1
        ):
            raise RuntimeError("diamond did not retain one exact shared R1")
        permuted_path = root / "release-diamond-permuted.lkja"
        build_application(permuted_path, requests["release-diamond"], diamond_releases)
        if diamond_path.read_bytes() != permuted_path.read_bytes():
            raise RuntimeError("application graph input permutation changed canonical bytes")

        bad_r1 = root / "shared-codec-r1-corrupt.lkjr"
        corrupt = bytearray(r1_path.read_bytes())
        corrupt[len(corrupt) // 2] ^= 1
        bad_r1.write_bytes(corrupt)
        corrupt_args = [
            "app",
            "build",
            "--release",
            str(a_path),
            "--release",
            str(bad_r1),
            "--validate-only",
        ]
        expect_failure(corrupt_args, requests["consumer-normalizer"], "artifact_corrupt")
        missing_args = ["app", "build", "--release", str(a_path), "--validate-only"]
        expect_failure(missing_args, requests["consumer-normalizer"], "artifact_corrupt")
        extra_args = [
            "app",
            "build",
            "--release",
            str(a_path),
            "--release",
            str(r1_path),
            "--release",
            str(r2_path),
            "--validate-only",
        ]
        expect_failure(extra_args, requests["consumer-normalizer"], "artifact_corrupt")

        shutil.rmtree(state)

        dependency_sets = {
            r1_path: [],
            r2_path: [],
            a_path: [r1_path],
            b_path: [r1_path],
            c_path: [r1_path, r2_path],
            d_path: [a_path, b_path, r1_path],
        }
        for release_path, dependencies in dependency_sets.items():
            json_command(["release", "validate", "--artifact", str(release_path)])
            json_command(["release", "inspect", "--artifact", str(release_path)])
            release_test(release_path, dependencies)

        for name, (path, receipt, releases) in applications.items():
            validation = json_command(["app", "validate", "--artifact", str(path)])
            inspection = json_command(["app", "inspect", "--artifact", str(path)])
            tests = json_command(["app", "test", "--artifact", str(path)])
            if validation["digest"] != inspection["digest"] or tests["report"]["passed"] != tests["report"]["total"]:
                raise RuntimeError(f"offline validation/test failed for {name}")
            rebuilt = root / f"{name}-offline-rebuilt.lkja"
            rebuilt_receipt = build_application(rebuilt, requests[name], releases)
            if rebuilt.read_bytes() != path.read_bytes() or rebuilt_receipt["inspection"]["digest"] != receipt["inspection"]["digest"]:
                raise RuntimeError(f"offline rebuild changed {name}")

        normalizer_path = applications["consumer-normalizer"][0]
        if run_process(["app", "stream", "--artifact", str(normalizer_path)], b"abc").stdout != b"abc":
            raise RuntimeError("offline normalizer stream result disagrees")
        inspector_path = applications["consumer-inspector"][0]
        inspector_run = json_command(
            ["app", "run", "--artifact", str(inspector_path)], invocation([bytes_value(b"abc")])
        )
        if inspector_run["result"]["value"] != i64_value(3):
            raise RuntimeError("offline inspector typed result disagrees")
        coexist_path = applications["release-version-coexistence"][0]
        coexist_run = json_command(
            ["app", "run", "--artifact", str(coexist_path)], invocation([r1_value, r2_value])
        )
        if coexist_run["result"]["value"] != r1_value:
            raise RuntimeError("coexistence typed result lost R1 nominal identity")
        expect_failure(
            ["app", "run", "--artifact", str(coexist_path)],
            invocation([r2_value, r2_value]),
            "run_argument_mismatch",
        )
        if run_process(["app", "stream", "--artifact", str(diamond_path)], b"abc").stdout != b"abc":
            raise RuntimeError("offline diamond stream result disagrees")

        report = {
            "campaign": "reusable-semantic-release",
            "release_contract": 1,
            "application_contract": 4,
            "canonical_rebuild_across_workspace_histories": True,
            "private_access_rejected": True,
            "workspaces_removed": not state.exists(),
            "offline_rebuild_validate_inspect_test_run": True,
            "r1_r2_nominal_substitution_rejected": True,
            "ambient_resolver_or_network": "absent",
            "releases": {
                "shared-codec-r1": {"id": r1_id, "bytes": r1_path.stat().st_size},
                "shared-codec-r2": {"id": r2_id, "bytes": r2_path.stat().st_size},
                "consumer-normalizer": {"id": a_id, "bytes": a_path.stat().st_size},
                "consumer-inspector": {"id": b_id, "bytes": b_path.stat().st_size},
                "release-version-coexistence": {"id": c_id, "bytes": c_path.stat().st_size},
                "release-diamond": {"id": d_id, "bytes": d_path.stat().st_size},
            },
            "applications": {
                name: {
                    "bytes": path.stat().st_size,
                    "digest": receipt["inspection"]["digest"],
                    "graph_nodes": len(receipt["inspection"]["releases"]),
                    "graph_edges": receipt["inspection"]["graph_edges"],
                    "graph_depth": receipt["inspection"]["graph_depth"],
                }
                for name, (path, receipt, _) in applications.items()
            },
            "diamond": {
                "exact_r1_occurrences": 1,
                "graph_nodes": 4,
                "graph_edges": 4,
                "graph_depth": 3,
            },
            "interaction": {
                "rpc_calls": len(rpc_measurements),
                "engine_opens": 1
                + sum(item["command"] == "release build" for item in command_measurements),
                "processes": len(command_measurements) + 1,
                "action_bytes": sum(item["request_bytes"] for item in rpc_measurements)
                + sum(item["input_bytes"] for item in command_measurements),
                "observation_bytes": sum(item["response_bytes"] for item in rpc_measurements)
                + sum(item["output_bytes"] for item in command_measurements),
                "diagnostic_bytes": sum(item["diagnostic_bytes"] for item in command_measurements),
                "elapsed_nanoseconds": sum(
                    item["elapsed_nanoseconds"] for item in rpc_measurements
                )
                + sum(item["elapsed_nanoseconds"] for item in command_measurements),
            },
        }
        print(json.dumps(report, sort_keys=True))


def main():
    try:
        workflow()
    finally:
        if session is not None and session.poll() is None:
            session.kill()


if __name__ == "__main__":
    main()
