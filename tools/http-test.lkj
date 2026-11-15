//! Direct lkjscript TCP functional requests; the shell supervisor owns the server.
mod http_client;
use http_client::{Connection, find, io, parse_header, require};
struct Case {
    method: String,
    target: String,
    headers: Vec<String>,
    status: u16,
    body: Vec<u8>,
    input: Vec<u8>,
    head: bool,
    one_zero: bool,
}

fn bytes(text: &str) -> Vec<u8> { text.as_bytes().to_vec() }
fn response(status: u16) -> Vec<u8> {
    bytes(match status {
        400 => "400 Bad Request\n",
        403 => "403 Forbidden\n",
        404 => "404 Not Found\n",
        405 => "405 Method Not Allowed\n",
        417 => "417 Expectation Failed\n",
        501 => "501 Not Implemented\n",
        _ => "unexpected\n",
    })
}
fn ordinary(target: &str, body: &[u8]) -> Case {
    Case {
        method: "GET".to_string(),
        target: target.to_string(),
        headers: Vec::new(),
        status: 200u16,
        body: body.to_vec(),
        input: Vec::new(),
        head: false,
        one_zero: false,
    }
}
fn bad(target: &str, status: u16) -> Case {
    let body = response(status);
    let mut case = ordinary(target, &body);
    case.status = status;
    case
}

fn request(case: &Case, connection: &str) -> Vec<u8> {
    let version = if case.one_zero { "HTTP/1.0" } else { "HTTP/1.1" };
    let mut output = format!("{} {} {version}\r\n", case.method, case.target);
    let mut host_override = false;
    let mut length_override = false;
    for header in &case.headers {
        if header.starts_with("Host:") { host_override = true; }
        if header.starts_with("Content-Length:") { length_override = true; }
    }
    if !host_override { output.push_str("Host: localhost\r\n"); }
    output.push_str(&format!("Connection: {connection}\r\n"));
    for header in &case.headers {
        if header != "Host:" {
            output.push_str(header);
            output.push_str("\r\n");
        }
    }
    if !case.input.is_empty() && !length_override {
        output.push_str(&format!("Content-Length: {}\r\n", case.input.len()));
    }
    output.push_str("\r\n");
    let mut bytes = output.as_bytes().to_vec();
    bytes.extend_from_slice(&case.input);
    bytes
}
fn validate(case: &Case, actual: &[u8]) -> Result<(), String> {
    let label = format!("{} {}", case.method, case.target);
    let end = match find(actual, b"\r\n\r\n") {
        Some(at) => at + 4usize,
        None => return Err(format!("{label}: missing HTTP headers")),
    };
    let response = parse_header(actual[..end].to_vec(), case.head)?;
    require(response.status == case.status, &format!("{label}: response status"))?;
    require(response.length == case.body.len(), &format!("{label}: content length"))?;
    require(response.close, &format!("{label}: connection lifetime"))?;
    require(find(&response.header, b"\r\nX-Elapsed-Nanoseconds: ").is_some(),
        &format!("{label}: clock observation"))?;
    if case.status == 405 {
        require(find(&response.header, b"\r\nAllow: GET, HEAD\r\n").is_some(),
            &format!("{label}: allowed methods"))?;
    }
    let expected = if case.head { &case.body[..0] } else { case.body.as_slice() };
    require(&actual[end..] == expected,
        &format!("{label}: response bytes; expected {}, got {}", expected.len(), actual.len() - end))
}
fn read(root: &mut Directory, path: &str) -> Vec<u8> {
    root.open(path).unwrap().read_to_end().unwrap()
}
fn cases(root: &mut Directory) -> Vec<Case> {
    let language = read(root, "language.md");
    let ir = read(root, "ir.md");
    let direction = read(root, "direction.md");
    let mut cases = vec![
        ordinary("/empty.bin", &read(root, "empty.bin")),
        ordinary("/chunk.bin", &read(root, "chunk.bin")),
        ordinary("/chunk-tail.bin", &read(root, "chunk-tail.bin")),
        ordinary("/", &language),
        ordinary("/language.md", &language),
        ordinary("/ir.md", &ir),
        ordinary("/direction.md", &direction),
        ordinary("/%6canguage.md?revision=1", &language),
        ordinary("http://localhost/language.md", &language),
        ordinary("/link-inside", &language),
        bad("/missing-file", 404u16),
        bad("/../outside.txt", 403u16),
        bad("/%2e%2e/outside.txt", 403u16),
        bad("/%2F..%2Foutside.txt", 403u16),
        bad("/link-outside", 403u16),
        bad("/%00", 400u16),
        bad("/%GG", 400u16),
    ];
    let mut head = ordinary("/ir.md", &ir);
    head.method = "HEAD".to_string();
    head.head = true;
    cases.push(head);
    let mut head = bad("/missing-file", 404u16);
    head.method = "HEAD".to_string();
    head.head = true;
    cases.push(head);
    let mut missing = bad("/", 400u16);
    missing.headers.push("Host:".to_string());
    cases.push(missing);
    let mut duplicate = bad("/", 400u16);
    duplicate
        .headers
        .push("X-Injected: value\r\nHost: duplicate".to_string());
    cases.push(duplicate);
    let mut malformed = bad("/", 400u16);
    malformed.headers.push("Bad Header: value".to_string());
    cases.push(malformed);
    let mut wrong_host = bad("/", 400u16);
    wrong_host.headers.push("Host: user@host".to_string());
    cases.push(wrong_host);
    let mut method = bad("/", 405u16);
    method.method = "POST".to_string();
    cases.push(method);
    let mut method = bad("/", 400u16);
    method.method = "G ET".to_string();
    cases.push(method);
    // Withhold the declared body: an unsupported expectation must get a final reply now.
    let mut expectation = bad("/", 417u16);
    expectation.headers.push("Expect: 100-continue".to_string());
    expectation.headers.push("Content-Length: 1".to_string());
    cases.push(expectation);
    let mut old = ordinary("/language.md", &language);
    old.one_zero = true;
    old.headers.push("Host:".to_string());
    cases.push(old);
    let mut body = ordinary("/language.md", &language);
    for index in 0usize..8193usize {
        body.input.push((index % 256usize) as u8);
    }
    cases.push(body);
    let mut large = ordinary("/language.md", &language);
    let mut header = "X-Large: ".to_string();
    for _index in 0usize..65537usize {
        header.push('x');
    }
    large.headers.push(header);
    cases.push(large);
    cases
}

fn persistent_response(connection: &mut Connection, case: &Case, close: bool) -> Result<(), String> {
    let response = connection.header(case.head)?;
    require(response.status == case.status, "persistent response status")?;
    require(response.length == case.body.len(), "persistent response Content-Length")?;
    require(response.close == close, "persistent response Connection")?;
    if !case.head { connection.verify_body(&case.body)?; }
    Ok(())
}
fn persistence(target: &mut Connector, root: &mut Directory) -> Result<usize, String> {
    let language = read(root, "language.md");
    let ir = read(root, "ir.md");
    let mut get = ordinary("/language.md", &language);
    let mut head = ordinary("/ir.md", &ir);
    head.method = "HEAD".to_string();
    head.head = true;
    let missing = bad("/missing-file", 404u16);
    let mut connection = Connection::new(io(target.connect(), "connect persistent sequence")?);
    for _index in 0usize..3usize {
        connection.send(&request(&get, "keep-alive"))?;
        persistent_response(&mut connection, &get, false)?;
        connection.send(&request(&head, "keep-alive"))?;
        persistent_response(&mut connection, &head, false)?;
    }
    connection.send(&request(&missing, "close"))?;
    persistent_response(&mut connection, &missing, true)?;
    connection.eof()?;

    let mut connection = Connection::new(io(target.connect(), "connect pipeline")?);
    for index in 0usize..8193usize { get.input.push((index % 256usize) as u8); }
    let mut old = ordinary("/language.md", &language);
    old.one_zero = true;
    old.headers.push("Host:".to_string());
    let mut pipeline = request(&get, "keep-alive");
    pipeline.extend_from_slice(&request(&head, "keep-alive"));
    pipeline.extend_from_slice(&request(&missing, "keep-alive"));
    pipeline.extend_from_slice(&request(&old, "keep-alive"));
    pipeline.extend_from_slice(&request(&get, "keep-alive, CLOSE"));
    connection.send(&pipeline)?;
    persistent_response(&mut connection, &get, false)?;
    persistent_response(&mut connection, &head, false)?;
    persistent_response(&mut connection, &missing, false)?;
    persistent_response(&mut connection, &old, false)?;
    persistent_response(&mut connection, &get, true)?;
    connection.eof()?;
    Ok(12usize)
}
fn exercise(world: &mut World, port: u16, root: &mut Directory) -> Result<(), String> {
    let mut target = io(world.connector("127.0.0.1", port), "grant loopback connector")?;
    let cases = cases(root);
    for case in &cases {
        let outgoing = request(case, "close");
        let label = format!("functional {} {} ({} request bytes)", case.method, case.target, outgoing.len());
        let mut stream = io(target.connect(), &format!("connect {label}"))?;
        io(stream.write_all(&outgoing), &format!("send {label}"))?;
        let actual = io(stream.read_to_end(), &format!("read {label}"))?;
        validate(case, &actual)?;
    }
    let persistent = persistence(&mut target, root)?;
    world.stdout().write(format!("ok http {} functional cases, {persistent} persistent/pipelined requests\n", cases.len()).as_bytes());
    Ok(())
}
pub fn main(mut world: World) {
    let args = world.args();
    assert_eq!(args.len(), 2usize);
    let port = String::from_utf8(args[0].clone()).unwrap().as_str().parse::<u16>().unwrap();
    let root_path = String::from_utf8(args[1].clone()).unwrap();
    let mut root = world.directory(&root_path).unwrap();
    if let Err(problem) = exercise(&mut world, port, &mut root) {
        world.stderr().write(format!("FAIL http: {problem}\n").as_bytes());
        world.set_exit_status(1u8);
    }
}
