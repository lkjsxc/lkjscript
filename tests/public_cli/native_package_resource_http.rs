use super::super::super::native_http as http;
use super::*;

const HTTP_TYPES: &str = r#"
  (use std builtin)
  (type-alias Header (record (name Text) (value Bytes)))
  (type-alias Request (record (body (stream Bytes)) (headers (list Header))
    (method Text) (path Text) (query Text) (query_parameters (map Text (list Text)))))
  (type-alias Response (record (body Bytes) (headers (list Header)) (status I64)))
"#;
const HTTP_MODULE: &str = r#"
    (function create respond (visibility private)
      (parameter create request (type Request)) (returns Response)
      (effect (task (requirement lib::queue::jobs)))
      (body (record structural (field status (i64 200)) (field headers (list Header))
        (field body (call std::bytes-from-text (call text))))))
    (component create server (visibility private)
      (requirement create streams (interface std::ByteStream)
        (operations std::ByteStream::read-all) (limits (maximum_calls 4 calls)))
      (port create respond
        (type (task-function (Request) Response (row (requirement lib::queue::jobs))))
        (function respond)))
    (component create app (visibility private)
"#;

#[test]
fn native_package_resources_http_requires_declared_foreign_authority_and_joins() {
    let source = CONSUMER
        .replace("(use std builtin)", HTTP_TYPES)
        .replace("(component create app (visibility private)", HTTP_MODULE)
        .replace(
            "(target create numbers",
            r#"
  (target create web (component resource-consumer::server) (runner http)
    (route create read (method GET) (path "/read") (port resource-consumer::server::respond)))
  (target create numbers"#,
        );
    let packages = Packages::stage(LIBRARY);
    packages.apply(&source);
    let data = packages.detach();
    let public = &packages.consumer;
    let template = Native::template("http");
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(template.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = serde_json::json!("generic.lkja");
    descriptor["target"] = serde_json::json!("web");
    descriptor["listen"] = serde_json::json!("127.0.0.1:0");
    let command = public.root.path().join("command.json");
    write_deployment(&command, "text");
    let queue: Value = serde_json::from_slice(&std::fs::read(command).unwrap()).unwrap();
    descriptor["grants"]
        .as_array_mut()
        .unwrap()
        .push(queue["grants"][0].clone());
    for (label, expected) in [("first", "日本語 + generic"), ("restart", "absent")] {
        let server = http::Server::start(public, label, &descriptor, &[]);
        let reply = http::send(server.address, "GET", "/read", &[("Host", "localhost")], "");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, expected);
        server.stop();
    }
    assert_completed_job(&read_job(&data, "text"), "text");
}
