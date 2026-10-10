use super::*;

impl Fifo {
    pub fn run(
        &self,
        target: &str,
        input: &Value,
        detached: bool,
        success: bool,
    ) -> (Vec<CompactRecord>, Option<Value>) {
        let serial = self.serial.get();
        self.serial.set(serial + 1);
        let arguments = self
            .consumer
            .input(&format!("arguments-{serial}.json"), &input.to_string());
        let result = self
            .consumer
            .root
            .path()
            .join(format!("result-{serial}.json"));
        assert!(!result.exists());
        let descriptor = self
            .consumer
            .root
            .path()
            .join(format!("{target}.deployment.json"));
        let mut command = if detached {
            vec!["run", "--deployment", path(&descriptor)]
        } else {
            vec!["run", target]
        };
        command.extend([
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&result),
        ]);
        let records = self.consumer.cli(&command, success);
        std::fs::write(
            self.consumer
                .root
                .path()
                .join(format!("execution-{serial}.log")),
            format!("target={target} detached={detached} success={success}\n{records:#?}\n"),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(&arguments).unwrap(),
            input.to_string().as_bytes()
        );
        let value = if success {
            let execution = compact_record(&records, "execution");
            if detached {
                joined(execution);
            } else {
                assert_eq!(compact_field(execution, "differential"), "equal");
            }
            Some(serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap())
        } else {
            assert!(
                !result.exists(),
                "failed execution exposed a partial result"
            );
            None
        };
        (records, value)
    }

    pub fn expected(&self, target: &str, input: &Value, expected: &Value, detached: bool) {
        let (_, actual) = self.run(target, input, detached, true);
        assert_eq!(&actual.unwrap(), expected, "{target}, detached={detached}");
    }
}

pub(super) fn joined(execution: &CompactRecord) {
    let observation: Value =
        serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
    for field in [
        "live_call_frames_after",
        "live_handles_after",
        "live_locals_after",
        "live_operands_after",
        "live_transactions_after",
        "live_type_bindings_after",
    ] {
        assert_eq!(observation[field], 0, "unreleased {field}");
    }
    assert_eq!(observation["capability_calls"], 0);
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["admission_stopped"], true);
    assert_eq!(cleanup["remaining_tasks"], 0);
    assert_eq!(cleanup["cleanup_failures"], json!([]));
    let executor: Value =
        serde_json::from_str(compact_field(execution, "executor-observation")).unwrap();
    assert_eq!(executor["dispatch_open"], false);
    assert_eq!(executor["active_dispatches"], 0);
    assert_eq!(executor["remaining_workers"], 0);
}

pub(super) fn failed_join(diagnostic: &CompactRecord) {
    let notes: Vec<String> = serde_json::from_str(compact_field(diagnostic, "notes")).unwrap();
    assert!(notes.iter().any(|note| note
        == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
    assert!(notes.iter().any(|note| note ==
        "structured worker cleanup: dispatch-stopped=true active=0 remaining-workers=0 joined-workers=0"));
}
