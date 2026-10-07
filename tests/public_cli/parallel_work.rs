//! Maintained native work: the same exact methods run serially, pooled, nested and over HTTP.
use super::native_byte_buffer::{Export, author, export, stage};
use super::*;
use base64::Engine;
use serde_json::json;

#[cfg(unix)]
use super::shared_runtime::process;

const CONTRACT: &str = include_str!("../../examples/parallel-work/contract.lkjc");
const CARRIERS: &str = include_str!("../../examples/parallel-work/carriers.lkjc");
const APPLICATION: &str = include_str!("../../examples/parallel-work/application.lkjc");
const HTTP: &str = include_str!("../../examples/parallel-work/http.lkjc");
const COMMAND: &str = include_str!("../../examples/parallel-work/command.deployment.json");
const SERVICE: &str = include_str!("../../examples/parallel-work/service.deployment.json");

fn substitute(source: &str, prefix: &str, package: &Export) -> String {
    source
        .replace(&format!("{prefix}_PACKAGE_REVISION"), &package.revision)
        .replace(&format!("{prefix}_PACKAGE"), &package.package)
        .replace(&format!("{prefix}_REVISION"), &package.semantic)
}

fn unchanged(public: &Native, module: &str) {
    let draft = public.root.path().join(format!("{module}.draft.lkjc"));
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            module,
            "--output",
            path(&draft),
        ],
        true,
    );
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );
}

struct Family {
    contract: Native,
    carriers: Native,
    application: Native,
    contract_export: Export,
    carriers_export: Export,
}

impl Family {
    fn author() -> Self {
        let contract = Native::new();
        author(&contract, CONTRACT);
        unchanged(&contract, "work-contract");
        let contract_export = export(&contract);
        // The command starter selects the exact built-in supplier. A `use std`
        // alias itself grants neither dependency selection nor implementation authority.
        let carriers = Native::template("command");
        stage(&carriers, &contract_export);
        author(
            &carriers,
            &substitute(CARRIERS, "CONTRACT", &contract_export),
        );
        unchanged(&carriers, "work-carriers");
        let carriers_export = export(&carriers);
        let application = Native::template("command");
        stage(&application, &contract_export);
        stage(&application, &carriers_export);
        let input = substitute(
            &substitute(APPLICATION, "CONTRACT", &contract_export),
            "CARRIERS",
            &carriers_export,
        );
        let before = application.revision();
        let invalid = application.input(
            "invalid-witness.lkjc",
            &format!(
                "request base={before}\n{}",
                input.replace(
                    "concrete@work-carriers::Octets concrete@work-carriers::Scalar",
                    "concrete@work-carriers::Octets concrete@work-carriers::Octets",
                ),
            ),
        );
        let rejected = application.plan(&invalid, false);
        assert!(
            rejected
                .iter()
                .any(|record| record.operation == "diagnostic"
                    && compact_field(record, "code") == "kernel_owned_contract")
        );
        assert_eq!(application.revision(), before);
        author(&application, &input);
        unchanged(&application, "work");
        author(&application, HTTP);
        unchanged(&application, "work-http");
        application.cli(
            &[
                "build",
                "--output",
                path(&application.root.path().join("work.lkja")),
            ],
            true,
        );
        Self {
            contract,
            carriers,
            application,
            contract_export,
            carriers_export,
        }
    }

    fn detach(&self) {
        for public in [&self.contract, &self.carriers, &self.application] {
            std::fs::remove_dir_all(&public.project).unwrap();
        }
        for package in [&self.contract_export.path, &self.carriers_export.path] {
            std::fs::remove_file(package).unwrap();
        }
    }

    fn retain(self) {
        if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
            for public in [self.contract, self.carriers, self.application] {
                println!(
                    "retained parallel work evidence: {}",
                    public.root.keep().display()
                );
            }
        }
    }
}

/// Independent expected bytes: Rust remainder and a simple loop, rather than
/// graph divide/subtract arithmetic, native carrier helpers or runtime execution.
fn expected(n: usize, rounds: usize, seed: u8) -> Value {
    let original = (0..n)
        .map(|index| (usize::from(seed) + index) % 256)
        .map(|octet| u8::try_from(octet).unwrap())
        .collect::<Vec<_>>();
    let transformed = original
        .iter()
        .map(|&octet| {
            let mut value = u32::from(octet);
            for _ in 0..rounds {
                value = (37 * value + 11) % 256;
            }
            u8::try_from(value).unwrap()
        })
        .collect::<Vec<_>>();
    let checksum = transformed
        .iter()
        .enumerate()
        .map(|(index, &octet)| i64::try_from(index + 1).unwrap() * i64::from(octet))
        .sum::<i64>();
    let mut payload = original;
    payload.extend(transformed);
    json!({"payload": {"$bytes": base64::engine::general_purpose::STANDARD.encode(payload)},
        "buffer_sum": checksum, "scalar_sum": checksum})
}

fn run(public: &Native, target: &str, n: usize, rounds: usize, seed: u8, label: &str) -> Value {
    let mut descriptor: Value = serde_json::from_str(COMMAND).unwrap();
    descriptor["target"] = json!(target);
    let deployment = public.input(&format!("{label}.deployment.json"), &descriptor.to_string());
    let arguments = public.input(
        &format!("{label}.arguments.json"),
        &json!([n, rounds, seed]).to_string(),
    );
    let output = public.root.path().join(format!("{label}.result.json"));
    let records = public.cli(
        &[
            "run",
            "--deployment",
            path(&deployment),
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&output),
        ],
        true,
    );
    let execution = compact_record(&records, "execution");
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["remaining_tasks"], 0);
    assert_eq!(cleanup["cleanup_failures"], json!([]));
    let observation: Value =
        serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
    assert_eq!(
        observation["parallel_scopes"],
        match target {
            "serial" => 0,
            "parallel" => 1,
            "nested" => 3,
            _ => unreachable!(),
        }
    );
    assert_eq!(observation["capability_calls"], 0);
    assert_eq!(observation["live_call_frames_after"], 0);
    assert_eq!(observation["live_handles_after"], 0);
    let executor: Value =
        serde_json::from_str(compact_field(execution, "executor-observation")).unwrap();
    assert_eq!(executor["dispatch_open"], false);
    assert_eq!(executor["active_dispatches"], 0);
    assert_eq!(executor["remaining_workers"], 0);
    // This CLI invocation owns one executor; no foreign process can lend a worker.
    assert_eq!(executor["workers_received"], 0);
    assert_eq!(executor["workers_handed_off"], 0);
    assert_eq!(executor["joined_workers"], executor["workers_started"]);
    assert!(
        observation["parallel_worker_dispatches"].as_u64().unwrap()
            <= observation["parallel_scopes"].as_u64().unwrap()
    );
    assert!(observation.get("parallel_workers_spawned").is_none());
    let value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(value, expected(n, rounds, seed));
    value
}

#[test]
fn native_parallel_work_three_packages_exact_methods_complete_payloads_and_detached_execution() {
    assert_eq!(expected(5, 3, 0)["buffer_sum"], 1_635);
    assert_eq!(expected(4096, 32, 0)["buffer_sum"], 1_069_109_248);
    let family = Family::author();
    for detached in [false, true] {
        if detached {
            family.detach();
        }
        for (n, rounds, seed) in [
            (0, 3, 0),
            (1, 0, 255),
            (5, 3, 0),
            (5, 3, 73),
            (256, 3, 0),
            (4096, 32, 0),
        ] {
            let serial = run(
                &family.application,
                "serial",
                n,
                rounds,
                seed,
                &format!("serial-{detached}-{n}-{rounds}-{seed}"),
            );
            for target in ["parallel", "nested"] {
                assert_eq!(
                    run(
                        &family.application,
                        target,
                        n,
                        rounds,
                        seed,
                        &format!("{target}-{detached}-{n}-{rounds}-{seed}")
                    ),
                    serial
                );
            }
        }
    }
    family.retain();
}

#[cfg(unix)]
#[test]
fn native_parallel_work_shared_cpu_service_and_independent_http_join_and_restart() {
    use super::native_http as http;
    use rustix::process::Signal;
    use std::sync::{Arc, Barrier};
    let family = Family::author();
    let sibling = Native::template("http");
    sibling.cli(
        &[
            "build",
            "--output",
            path(&sibling.root.path().join("independent.lkja")),
        ],
        true,
    );
    let mut independent: Value = serde_json::from_slice(
        &std::fs::read(sibling.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    std::fs::copy(
        sibling.root.path().join("independent.lkja"),
        family.application.root.path().join("independent.lkja"),
    )
    .unwrap();
    independent["artifact"] = json!("independent.lkja");
    independent["runtime"]["request_deadline_milliseconds"] = json!(120_000);
    let cpu: Value = serde_json::from_str(SERVICE).unwrap();
    let descriptors = [cpu, independent];
    family.detach();
    std::fs::remove_dir_all(&sibling.project).unwrap();
    for cycle in 0..2 {
        let group = process::Group::start(&family.application, &descriptors, &[]);
        let independent_address = group.address(1);
        let baseline = http::send(independent_address, "GET", "/", &[], "");
        assert_eq!(baseline.status, 200);
        std::thread::scope(|scope| {
            let barrier = Arc::new(Barrier::new(5));
            let mut requests = Vec::new();
            for _ in 0..4 {
                let barrier = barrier.clone();
                let address = group.address(0);
                requests.push(scope.spawn(move || {
                    barrier.wait();
                    let reply = http::send(address, "GET", "/work", &[], "");
                    assert_eq!(reply.status, 200);
                    assert_eq!(
                        serde_json::from_str::<Value>(&reply.body).unwrap(),
                        expected(4096, 32, 0)
                    );
                }));
            }
            barrier.wait();
            for _ in 0..16 {
                let reply = http::send(independent_address, "GET", "/", &[], "");
                assert_eq!(reply.status, 200);
                assert_eq!(reply.body, baseline.body);
            }
            for request in requests {
                request.join().unwrap();
            }
        });
        // These requests start after the concurrent batch has fully joined. More
        // completed dispatches than physical starts proves reuse without a timing race.
        for _ in 0..2 {
            let reply = http::send(group.address(0), "GET", "/work", &[], "");
            assert_eq!(reply.status, 200);
            assert_eq!(
                serde_json::from_str::<Value>(&reply.body).unwrap(),
                expected(4096, 32, 0)
            );
        }
        let stopped = group.stop(if cycle == 0 {
            Signal::INT
        } else {
            Signal::TERM
        });
        assert_eq!(stopped["instances"].as_array().unwrap().len(), 2);
        let executor = &stopped["shared_runtime"]["executor"];
        println!("public parallel work executor cycle {cycle}: {executor}");
        let started = executor["workers_started"].as_u64().unwrap();
        if started > 0 {
            assert!(executor["completed_dispatches"].as_u64().unwrap() > started);
        } else {
            // A zero ceiling or native thread-creation refusal can legitimately
            // run every child inline; neither establishes physical worker reuse.
            assert_eq!(executor["completed_dispatches"], 0);
            assert!(executor["inline_fallbacks"].as_u64().unwrap() > 0);
        }
    }
    family.retain();
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        println!(
            "retained independent HTTP evidence: {}",
            sibling.root.keep().display()
        );
    }
}
