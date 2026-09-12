//! Iteration observations are a required child of the existing offline-package owner.
use super::*;
use serde_json::{Value, json};

fn request(
    address: std::net::SocketAddr,
    mode: &str,
    expected: Option<Value>,
    failure: Option<&str>,
) -> Result<Value, DevError> {
    let input = json!({"mode":mode,"prefix":0,"values":[]});
    let response =
        crate::http_probe::request(address, "GET", "/", &serde_json::to_vec(&input)?, &[])?;
    let code = response
        .headers
        .get("x-lkjscript-failure-code")
        .map(String::as_str);
    require(
        response.status == if expected.is_some() { 200 } else { 500 } && code == failure,
        &format!(
            "iteration HTTP status or failure differs: mode={mode} status={} code={code:?}",
            response.status
        ),
    )?;
    let result = if let Some(expected) = expected {
        let actual: Value = serde_json::from_slice(&response.body)?;
        require(
            actual == expected,
            &format!("iteration complete output differs: {actual} != {expected}"),
        )?;
        actual
    } else {
        Value::Null
    };
    Ok(
        json!({"request":input,"status":response.status,"result":result,"failure":code,"elapsed_nanoseconds":response.elapsed_nanoseconds}),
    )
}

pub(super) fn public(context: &mut Context, source: &Path) -> Result<(), DevError> {
    let root = context.root.join("iteration-standalone");
    fs::create_dir(&root)?;
    fs::copy(
        source.join("application.lkja"),
        root.join("application.lkja"),
    )?;
    let mut descriptor: Value = serde_json::from_slice(&process::read_bounded(
        &source.join("service.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    context.cli(
        None,
        &[
            "data",
            "initialize",
            "--root",
            &root.join("data").display().to_string(),
        ],
        true,
    )?;
    let mut rounds = Vec::new();
    let cells = [
        (1_i64, 0_i64),
        (1, 1),
        (1, 4097),
        (1, 8193),
        (2, 33),
        (0, 3),
        (-1, 3),
        (1, -1),
        (1_i64 << 62, i64::MAX),
    ];
    for (index, (stride, threshold)) in cells.into_iter().enumerate() {
        descriptor["configuration"]["stride"] = json!({"kind":"i64","value":stride});
        descriptor["configuration"]["threshold"] = json!({"kind":"i64","value":threshold});
        descriptor["configuration"]["tick"] = json!({"kind":"i64","value":0});
        fs::write(
            root.join("service.deployment.json"),
            evidence::encode_json(&descriptor)?,
        )?;
        let expected = if stride > 0 && threshold >= 0 && index != 8 {
            let m = (threshold + stride - 1) / stride;
            Some(json!({"position":m * stride,"total":stride * m * (m - 1) / 2}))
        } else {
            None
        };
        let failure = if index == 8 {
            Some("normalized_integer_overflow")
        } else if expected.is_none() {
            Some("normalized_integer_division")
        } else {
            None
        };
        let mut round = crate::stateful_http::recursive_round(
            &context.binary,
            &root,
            &context.evidence,
            &format!("iteration-config-{index}"),
            |address| {
                Ok(json!([request(
                    address,
                    "iterate",
                    expected.clone(),
                    failure
                )?]))
            },
        )?;
        round["stride"] = json!(stride);
        round["threshold"] = json!(threshold);
        rounds.push(round);
    }
    let data = crate::stateful_http::recursive_round(
        &context.binary,
        &root,
        &context.evidence,
        "iteration-data",
        |address| {
            Ok(json!([
                request(
                    address,
                    "iterate-transaction",
                    Some(json!({"position":3,"total":3})),
                    None
                )?,
                request(address, "read", Some(json!(3)), None)?,
                request(
                    address,
                    "iterate-rollback",
                    None,
                    Some("normalized_integer_division")
                )?,
                request(address, "read", Some(json!(3)), None)?,
                request(
                    address,
                    "iterate-partial",
                    None,
                    Some("normalized_integer_division")
                )?,
                request(address, "read", Some(json!(1)), None)?,
            ]))
        },
    )?;
    rounds.push(data);
    rounds.push(crate::stateful_http::recursive_round(
        &context.binary,
        &root,
        &context.evidence,
        "iteration-restart",
        |address| Ok(json!([request(address, "read", Some(json!(1)), None)?])),
    )?);
    let proof = json!({"rounds":rounds,"freshly_authored":true,"default_policy":true,"producer_available":false,"effects_replayed":false,"cleanup_complete":true});
    fs::write(
        context.evidence.join("task-iteration.json"),
        evidence::encode_json(&proof)?,
    )?;
    context.receipt.effects.iteration = proof;
    Ok(())
}

pub(super) fn validate(parent: &Receipt, root: &Path) -> Result<(), DevError> {
    let proof = &parent.effects.iteration;
    let actual: Value = serde_json::from_slice(&process::read_bounded(
        &root.join("task-iteration.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        *proof == actual
            && proof["freshly_authored"] == true
            && proof["default_policy"] == true
            && proof["producer_available"] == false
            && proof["effects_replayed"] == false
            && proof["cleanup_complete"] == true,
        "fresh default-policy public task iteration is absent or substituted",
    )?;
    let rounds = proof["rounds"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("iteration rounds missing"))?;
    require(
        rounds.len() == 11,
        "iteration stopping, overflow, persistence, or recovery rounds missing",
    )?;
    for (index, (stride, threshold)) in [
        (1_i64, 0_i64),
        (1, 1),
        (1, 4097),
        (1, 8193),
        (2, 33),
        (0, 3),
        (-1, 3),
        (1, -1),
        (1_i64 << 62, i64::MAX),
    ]
    .into_iter()
    .enumerate()
    {
        let round = &rounds[index];
        let rows = round["observations"]
            .as_array()
            .ok_or_else(|| DevError::corrupt("iteration observations absent"))?;
        require(
            round["stride"] == stride
                && round["threshold"] == threshold
                && rows.len() == 1
                && rows[0]["request"] == json!({"mode":"iterate","prefix":0,"values":[]}),
            "iteration configuration or request changed",
        )?;
        if index < 5 {
            let m = (threshold + stride - 1) / stride;
            require(
                rows[0]["status"] == 200
                    && rows[0]["failure"].is_null()
                    && rows[0]["result"]
                        == json!({"position":m * stride,"total":stride * m * (m - 1) / 2}),
                "iteration complete arithmetic result differs",
            )?;
        } else {
            require(
                rows[0]["status"] == 500
                    && rows[0]["result"].is_null()
                    && rows[0]["failure"]
                        == if index == 8 {
                            "normalized_integer_overflow"
                        } else {
                            "normalized_integer_division"
                        },
                "invalid or overflowing iteration returned a value",
            )?;
        }
    }
    let data = &rounds[9]["observations"];
    require(
        data.as_array().is_some_and(|r| r.len() == 6)
            && data[0]["result"] == json!({"position":3,"total":3})
            && data[1]["result"] == 3
            && data[2]["status"] == 500
            && data[2]["failure"] == "normalized_integer_division"
            && data[3]["result"] == 3
            && data[4]["status"] == 500
            && data[4]["failure"] == "normalized_integer_division"
            && data[5]["result"] == 1
            && rounds[10]["observations"]
                .as_array()
                .is_some_and(|r| r.len() == 1)
            && rounds[10]["observations"][0]["result"] == 1,
        "iteration commit, later rollback, separately committed writes, or restart readback differs",
    )?;
    for round in rounds {
        let rows = round["observations"]
            .as_array()
            .ok_or_else(|| DevError::corrupt("iteration round observations absent"))?;
        require(
            round["cleanup_complete"] == true
                && round["runtime"]["runs"] == 1
                && round["runtime"]["admitted_tasks"] == rows.len()
                && round["runtime"]["completed_tasks"] == rows.len()
                && round["runtime"]["failed_tasks"]
                    == rows.iter().filter(|r| r["status"] == 500).count(),
            "iteration HTTP ownership or completion counts differ",
        )?;
    }
    neutral(&parent.effects.resources["iteration"])
}

fn neutral(proof: &Value) -> Result<(), DevError> {
    let authority = proof["authority"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("iteration outgoing authority observations absent"))?;
    require(authority.len() == 2, "iteration authority tiers missing")?;
    for (index, tier) in ["production", "reference"].into_iter().enumerate() {
        let row = &authority[index];
        require(
            row["tier"] == tier
                && row["case"] == "outgoing-narrow-before-tail-invoke"
                && row["events"] == json!(["stride", "threshold"])
                && row["successful_result"] == false
                && row["component_has_grant"] == true
                && row["failure"]["message"]
                    .as_str()
                    .is_some_and(|m| m.contains("allowance")),
            "iteration tail target widened its outgoing allowance or executed the forbidden callback",
        )?;
    }
    let rows = proof["rows"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("neutral task iteration evidence absent"))?;
    require(
        rows.len() == 40
            && proof["maximum_control_bound"] == 6
            && proof["maximum_local_bound"] == 20
            && proof["effects_replayed"] == false,
        "task iteration control, timing, or stopping cells missing",
    )?;
    for (index, row) in rows.iter().enumerate() {
        let reference = index >= 20;
        let cell = index % 20;
        let (stride, threshold, case) = match cell {
            0 => (1, 0, "none"),
            1 => (1, 1, "none"),
            2 => (1, 31, "none"),
            3 => (1, 4097, "none"),
            4 => (1, 8193, "none"),
            5 => (2, 33, "none"),
            6 => (0, 3, "none"),
            7 => (-1, 3, "none"),
            8 => (1, -1, "none"),
            9 => (1_i64 << 62, i64::MAX, "overflow"),
            10 => (1, 8193, "cancelled"),
            11 => (1, 8193, "callback-failure"),
            12 => (1, 8193, "malformed-result"),
            13 => (1, 8193, "quota"),
            14 => (1, 8193, "fuel"),
            15 => (1, 8193, "allocation"),
            _ => (1, 8193, "none"),
        };
        require(
            row["tier"] == if reference { "reference" } else { "production" }
                && row["stride"] == stride
                && row["threshold"] == threshold
                && row["case"] == case
                && row["observed"] == (cell < 16)
                && row["policy"]["maximum_call_depth"] == 64
                && row["cleanup_complete"] == true
                && row["execution_nanoseconds"].as_u64().is_some_and(|n| n > 0),
            "iteration neutral identity, policy, or timing differs",
        )?;
        let events = row["events"]
            .as_array()
            .ok_or_else(|| DevError::corrupt("iteration neutral trace absent"))?;
        require(
            *events
                == (0..events.len())
                    .map(|n| {
                        json!(match n {
                            0 => "stride",
                            1 => "threshold",
                            _ => "tick",
                        })
                    })
                    .collect::<Vec<_>>(),
            "iteration ordered callbacks changed",
        )?;
        if !(6..16).contains(&cell) {
            let m = (threshold + stride - 1) / stride;
            require(
                events.len() == m as usize + 3
                    && row["output"]["updates"] == m
                    && row["output"]["callbacks"] == m + 1
                    && row["output"]["value"]
                        == json!({"position":m * stride,"total":stride * m * (m - 1) / 2})
                    && row["output"]["failure"].is_null(),
                "iteration independent complete result or stopping callback absent",
            )?;
        } else {
            let code = row["output"]["failure"]["code"]
                .as_str()
                .ok_or_else(|| DevError::corrupt("iteration failure absent"))?;
            let correct = match cell {
                6..=8 => events.len() == 2 && code.contains("integer_division"),
                9 => events.len() == 4 && code.contains("integer_overflow"),
                10 => events.len() == 5 && code == "execution_cancelled",
                11 => events.len() == 5 && code == "effect_probe_callback",
                12 => events.len() == 5 && code.contains("value_admission"),
                13 => events.len() == 7 && code.contains("grant_calls"),
                14 => {
                    events.len() > 2
                        && events.len() < 8196
                        && (code.contains("steps") || code.contains("expressions"))
                }
                15 => events.len() > 2 && events.len() < 8196 && code.contains("allocation"),
                _ => false,
            };
            require(
                correct && row["output"]["value"].is_null(),
                "iteration failure classification, prefix trace, or absence of result differs",
            )?;
        }
        if cell < 16 {
            let w = &row["work"];
            if cell < 6 {
                require(
                    w["value_work"]["capture_admission_nodes"] == 6
                        && w["value_work"]["internal_guard_descendant_visits"] == 0
                        && w["value_work"]["raw_result_admission_nodes"] == events.len(),
                    "iteration rescanned its captured prefix or skipped actual adapter result admission",
                )?;
            }
            for (name, bound) in [
                ("maximum_call_depth", 6),
                ("maximum_control_frames", 16),
                ("maximum_live_locals", 20),
                ("maximum_live_type_bindings", 6),
                ("maximum_live_allowances", 6),
            ] {
                require(
                    w[name].as_u64().is_some_and(|n| n <= bound),
                    "iteration exceeded constant control/local/substitution state",
                )?;
            }
            if reference {
                require(
                    w["maximum_live_effect_bindings"]
                        .as_u64()
                        .is_some_and(|n| n <= 4),
                    "iteration effect substitutions grew",
                )?;
            }
            for name in if reference {
                &[
                    "live_call_frames_after",
                    "live_control_frames_after",
                    "live_local_scopes_after",
                    "live_type_scopes_after",
                    "live_effect_scopes_after",
                    "live_allowances_after",
                    "live_transactions_after",
                    "live_handles_after",
                ][..]
            } else {
                &[
                    "live_call_frames_after",
                    "live_locals_after",
                    "live_type_bindings_after",
                    "live_operands_after",
                    "live_transactions_after",
                    "live_handles_after",
                ][..]
            } {
                require(
                    w[*name] == 0,
                    "iteration reader found retained invocation state",
                )?;
            }
        } else {
            require(
                row["work"].is_null(),
                "disabled observation was substituted",
            )?;
        }
    }
    Ok(())
}
