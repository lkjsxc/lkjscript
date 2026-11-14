//! Native generated HTTP traffic. The supervisor owns the server process.
struct Outcome {
    status: i64,
    output: Vec<u8>,
    errors: Vec<u8>,
}
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
fn bytes(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}
fn option(arguments: &mut Vec<Vec<u8>>, key: &str, value: &str) {
    arguments.push(bytes(key));
    arguments.push(bytes(value));
}
fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string())
    }
}
fn open_executable(world: &mut World, path: &str) -> File {
    let mut split = 0usize;
    for (index, byte) in path.as_bytes().iter().enumerate() {
        if *byte == b'/' {
            split = index + 1;
        }
    }
    let parent = if split == 0 { "." } else { &path[..split] };
    let mut directory = world.directory(parent).unwrap();
    directory.open(&path[split..]).unwrap()
}
fn invoke(
    world: &mut World,
    executable: &File,
    arguments: &[Vec<u8>],
    input: &[u8],
) -> Result<Outcome, String> {
    let mut stdin = world.temporary_file().unwrap();
    let mut stdout = world.temporary_file().unwrap();
    let mut stderr = world.temporary_file().unwrap();
    let mut result = world.temporary_file().unwrap();
    stdin.write_all(input).unwrap();
    stdin.rewind().unwrap();
    let mut process = world.process();
    let mut child =
        match process.spawn_file(executable, arguments, &stdin, &stdout, &stderr, &result) {
            Ok(child) => child,
            Err(errno) => return Err(format!("curl spawn errno {errno}")),
        };
    let status = child.wait().unwrap();
    stdout.rewind().unwrap();
    stderr.rewind().unwrap();
    result.rewind().unwrap();
    require(
        result.read_to_end().unwrap().is_empty(),
        "curl wrote the driver result descriptor",
    )?;
    Ok(Outcome {
        status,
        output: stdout.read_to_end().unwrap(),
        errors: stderr.read_to_end().unwrap(),
    })
}
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if haystack.len() < needle.len() {
        return None;
    }
    let mut at = 0usize;
    while at <= haystack.len() - needle.len() {
        if &haystack[at..at + needle.len()] == needle {
            return Some(at);
        }
        at += 1;
    }
    None
}
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
fn arguments(url: &str, case: &Case, include: bool) -> Vec<Vec<u8>> {
    let mut arguments = vec![
        bytes("--silent"),
        bytes("--show-error"),
        bytes("--http1.1"),
        bytes("--path-as-is"),
        bytes("--globoff"),
        bytes("--noproxy"),
        bytes("*"),
        bytes("--max-time"),
        bytes("600"),
        bytes("--proto"),
        bytes("=http"),
    ];
    option(&mut arguments, "--request", &case.method);
    option(&mut arguments, "--request-target", &case.target);
    option(
        &mut arguments,
        "--write-out",
        "\nLKJ_HTTP_STATUS:%{http_code}\n",
    );
    if include {
        arguments.push(bytes("--include"));
    }
    // Custom HEAD keeps body reading enabled; ignoring length exposes any forbidden body before EOF.
    if case.head {
        arguments.push(bytes("--ignore-content-length"));
    }
    if case.one_zero {
        arguments.push(bytes("--http1.0"));
    }
    for header in &case.headers {
        option(&mut arguments, "--header", header);
    }
    if !case.input.is_empty() {
        option(&mut arguments, "--data-binary", "@-");
        option(&mut arguments, "--header", "Expect:");
    }
    option(&mut arguments, "--url", url);
    arguments
}
fn append_expected(expected: &mut Vec<u8>, case: &Case) {
    if !case.head {
        expected.extend_from_slice(&case.body);
    }
    expected.extend_from_slice(format!("\nLKJ_HTTP_STATUS:{}\n", case.status).as_bytes());
}
fn validate(case: &Case, actual: &Outcome) -> Result<(), String> {
    let label = format!("{} {}", case.method, case.target);
    require(
        actual.status == 0,
        &format!(
            "{label}: curl status {}; {}",
            actual.status,
            String::from_utf8(actual.errors.clone()).unwrap()
        ),
    )?;
    require(actual.errors.is_empty(), &format!("{label}: curl stderr"))?;
    let end = match find(&actual.output, b"\r\n\r\n") {
        Some(at) => at + 4,
        None => return Err(format!("{label}: missing HTTP headers")),
    };
    let header = &actual.output[..end];
    let prefix = format!("HTTP/1.1 {} ", case.status);
    require(
        header.len() >= prefix.len() && &header[..prefix.len()] == prefix.as_bytes(),
        &format!("{label}: response status"),
    )?;
    let length = format!("\r\nContent-Length: {}\r\n", case.body.len());
    require(
        find(header, length.as_bytes()).is_some(),
        &format!("{label}: content length"),
    )?;
    require(
        find(header, b"\r\nConnection: close\r\n").is_some(),
        &format!("{label}: connection lifetime"),
    )?;
    require(
        find(header, b"\r\nX-Elapsed-Nanoseconds: ").is_some(),
        &format!("{label}: clock observation"),
    )?;
    if case.status == 405 {
        require(
            find(header, b"\r\nAllow: GET, HEAD\r\n").is_some(),
            &format!("{label}: allowed methods"),
        )?;
    }
    let mut expected = Vec::new();
    append_expected(&mut expected, case);
    require(
        &actual.output[end..] == expected.as_slice(),
        &format!(
            "{label}: response bytes; expected {}, got {}",
            expected.len(),
            actual.output.len() - end
        ),
    )
}
fn read(root: &mut Directory, path: &str) -> Vec<u8> {
    root.open(path).unwrap().read_to_end().unwrap()
}
fn cases(root: &mut Directory) -> Vec<Case> {
    let language = read(root, "language.md");
    let ir = read(root, "ir.md");
    let direction = read(root, "direction.md");
    let mut cases = vec![
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
fn batch(
    world: &mut World,
    curl: &File,
    url: &str,
    cases: &[Case],
    indices: &[usize],
) -> Result<usize, String> {
    let mut args = vec![bytes("--disable")];
    let mut expected = Vec::new();
    for (position, index) in indices.iter().enumerate() {
        if position != 0 {
            args.push(bytes("--next"));
        }
        for argument in arguments(url, &cases[*index], false) {
            args.push(argument);
        }
        append_expected(&mut expected, &cases[*index]);
    }
    let actual = invoke(world, curl, &args, &[0u8; 0])?;
    require(
        actual.status == 0,
        &format!(
            "batch curl status {}; {}",
            actual.status,
            String::from_utf8(actual.errors.clone()).unwrap()
        ),
    )?;
    require(actual.errors.is_empty(), "batch curl stderr")?;
    require(
        actual.output == expected,
        &format!(
            "batch response bytes: expected {}, got {}",
            expected.len(),
            actual.output.len()
        ),
    )?;
    Ok(indices.len())
}
fn exercise(
    world: &mut World,
    curl: &File,
    url: &str,
    root: &mut Directory,
    seconds: usize,
) -> Result<(), String> {
    let cases = cases(root);
    for case in &cases {
        let mut args = vec![bytes("--disable")];
        for argument in arguments(url, case, true) {
            args.push(argument);
        }
        let actual = invoke(world, curl, &args, &case.input)?;
        validate(case, &actual)?;
    }
    world
        .stdout()
        .write(format!("ok http {} functional cases\n", cases.len()).as_bytes());
    if seconds == 0 {
        world.stdout().write(b"warmup complete\n");
        return Ok(());
    }
    let mut choices: Vec<usize> = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        if !case.head && case.input.is_empty() {
            choices.push(index);
        }
    }
    for _round in 0usize..8usize {
        batch(world, curl, url, &cases, &choices)?;
    }
    world.stdout().write(b"warmup complete\n");
    let mut clock = world.clock();
    let start = clock.nanoseconds().unwrap();
    let duration = seconds * 1_000_000_000usize;
    let mut seed = 9173u64;
    let mut requests = 0usize;
    let mut batches = 0usize;
    let mut next_report = 60_000_000_000usize;
    while clock.nanoseconds().unwrap() - start < duration {
        let mut indices = Vec::new();
        for _index in 0usize..16usize {
            seed = (seed * 48271u64) % 2147483647u64;
            indices.push(choices[(seed % (choices.len() as u64)) as usize]);
        }
        requests += batch(world, curl, url, &cases, &indices)?;
        batches += 1;
        let elapsed = clock.nanoseconds().unwrap() - start;
        if elapsed >= next_report {
            let elapsed_seconds = elapsed / 1_000_000_000usize;
            world.stdout().write(format!("http load progress: {requests} verified requests, {batches} batches, {elapsed_seconds} s\n").as_bytes());
            next_report = (elapsed / 60_000_000_000usize + 1usize) * 60_000_000_000usize;
        }
    }
    let elapsed = clock.nanoseconds().unwrap() - start;
    let per_second = if elapsed == 0 {
        0usize
    } else {
        requests * 1_000_000_000usize / elapsed
    };
    world.stdout().write(format!("ok http load {requests} requests, {batches} batches, {elapsed} ns, {per_second} requests/s\n").as_bytes());
    Ok(())
}
pub fn main(mut world: World) {
    let args = world.args();
    assert_eq!(args.len(), 4usize);
    let curl_path = String::from_utf8(args[0].clone()).unwrap();
    let url = String::from_utf8(args[1].clone()).unwrap();
    let root_path = String::from_utf8(args[2].clone()).unwrap();
    let seconds = String::from_utf8(args[3].clone())
        .unwrap()
        .as_str()
        .parse::<usize>()
        .unwrap();
    let curl = open_executable(&mut world, &curl_path);
    let mut root = world.directory(&root_path).unwrap();
    if let Err(problem) = exercise(&mut world, &curl, &url, &mut root, seconds) {
        world
            .stderr()
            .write(format!("FAIL http: {problem}\n").as_bytes());
        world.set_exit_status(1u8);
    }
}
