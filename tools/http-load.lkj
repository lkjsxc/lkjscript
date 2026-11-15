//! Verified persistent HTTP load. No child process, reconnect or file read
//! occurs inside the measured loop. Stdin: seconds, pipeline width, relative path.
//! Pipeline width describes a test workload; blocking writes require its request
//! batch to fit the connection's send capacity. The supervisor has a watchdog.
//! Optional fourth configuration line: full (default) or framing.
//! Both modes validate status, length and connection lifetime and consume every
//! body byte. Full compares the body; framing drains it without comparison.
mod http_client;
use http_client::{Connection, io, require};
struct Config { seconds: usize, pipeline: usize, path: String, full: bool }
fn parse_configuration(text: &str) -> Result<Config, String> {
    let mut lines = text.split('\n');
    let seconds = match lines.next().ok_or("missing seconds".to_string())?.parse::<usize>() {
        Ok(value) => value,
        Err(_) => return Err("invalid seconds".to_string()),
    };
    let pipeline = match lines.next().ok_or("missing pipeline width".to_string())?.parse::<usize>() {
        Ok(value) => value,
        Err(_) => return Err("invalid pipeline width".to_string()),
    };
    let path = lines.next().ok_or("missing relative path".to_string())?.to_string();
    let mode = lines.next().unwrap_or("");
    let full = match mode {
        "" | "full" => true,
        "framing" => false,
        _ => return Err("verification must be full or framing".to_string()),
    };
    for extra in lines {
        require(extra.is_empty(), "unexpected line after verification mode")?;
    }
    require(pipeline > 0usize, "pipeline width must be positive")?;
    require(!path.is_empty(), "empty load path")?;
    for byte in path.as_bytes() { require(*byte > 32u8 && *byte < 127u8, "invalid load path byte")?; }
    require(seconds <= (!0usize) / 1_000_000_000usize, "load duration overflows nanoseconds")?;
    Ok(Config { seconds, pipeline, path, full })
}
fn configuration(input: &mut Stdin) -> Result<Config, String> {
    let bytes = io(input.read_to_end(), "read load configuration")?;
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return Err("load configuration is not UTF-8".to_string()),
    };
    parse_configuration(text.as_str())
}
fn batch(connection: &mut Connection, request: &[u8], expected: &[u8], count: usize, full: bool) -> Result<(), String> {
    connection.send(request)?;
    for _index in 0usize..count {
        let response = connection.header(false)?;
        require(response.status == 200u16, "load response status is not 200")?;
        require(response.length == expected.len(), "load response Content-Length differs")?;
        require(!response.close, "load response closed a persistent connection")?;
        if full { connection.verify_body(expected)?; }
        else { connection.discard_body(response.length)?; }
    }
    Ok(())
}
fn exercise(output: &mut Stdout, input: &mut Stdin, target: &mut Connector,
    root: &mut Directory, clock: &mut Clock) -> Result<(), String> {
    let config = configuration(input)?;
    let verification = if config.full { "full" } else { "framing" };
    let detail = if config.full { "body bytes compared" } else { "body bytes drained without comparison" };
    let mut file = io(root.open(&config.path), "open expected body")?;
    let expected = io(file.read_to_end(), "read expected body")?;
    drop(file);
    let single = format!("GET /{} HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n", config.path);
    let mut request = Vec::new();
    for _index in 0usize..config.pipeline { request.extend_from_slice(single.as_bytes()); }
    let mut connection = Connection::new(io(target.connect(), "connect load")?);
    for _round in 0usize..8usize { batch(&mut connection, &request, &expected, config.pipeline, config.full)?; }
    output.write(format!("warmup complete; persistent connections 1, pipeline {}, body {} bytes, path /{}; verification {verification}\n",
        config.pipeline, expected.len(), config.path).as_bytes());
    let started = io(clock.nanoseconds(), "start clock")?;
    let duration = config.seconds * 1_000_000_000usize;
    let mut elapsed = 0usize;
    let mut requests = 0usize;
    let mut batches = 0usize;
    let mut next_report = 60_000_000_000usize;
    while elapsed < duration {
        batch(&mut connection, &request, &expected, config.pipeline, config.full)?;
        requests += config.pipeline;
        batches += 1usize;
        elapsed = io(clock.nanoseconds(), "load clock")? - started;
        if elapsed >= next_report {
            output.write(format!("http load progress: {requests} responses, {batches} batches, {elapsed} ns; verification {verification}\n").as_bytes());
            next_report = (elapsed / 60_000_000_000usize + 1usize) * 60_000_000_000usize;
        }
    }
    let per_second = rate(requests, elapsed);
    output.write(format!("ok http load {requests} responses, {batches} batches, {elapsed} ns, {per_second} requests/s, 1 persistent connection; verification {verification}: {detail}\n").as_bytes());
    Ok(())
}
// Decimal requests/s without overflowing an intermediate count * 1e9.
// The remainder is always below elapsed; doubling/addition use subtraction first.
fn rate(count: usize, elapsed: usize) -> String {
    if elapsed == 0usize { return "0".to_string(); }
    let whole = count / elapsed;
    let numerator = count % elapsed;
    let mut remainder = 0usize;
    let mut fraction = 0usize;
    let mut bit = 30usize;
    while bit > 0usize {
        bit -= 1usize;
        fraction += fraction;
        if remainder >= elapsed - remainder {
            remainder -= elapsed - remainder;
            fraction += 1usize;
        } else { remainder += remainder; }
        if ((1_000_000_000usize >> (bit as u64)) & 1usize) != 0usize {
            if remainder >= elapsed - numerator {
                remainder -= elapsed - numerator;
                fraction += 1usize;
            } else { remainder += numerator; }
        }
    }
    if whole == 0usize { format!("{fraction}") } else { format!("{whole}{fraction:09}") }
}
#[test]
fn rate_without_intermediate_overflow() {
    assert_eq!(rate(3usize, 2usize), "1500000000");
    assert_eq!(rate(!0usize, !0usize), "1000000000");
    assert_eq!(rate((!0usize) - 1usize, !0usize), "999999999");
    assert_eq!(rate(!0usize, 1usize), "18446744073709551615000000000");
    assert_eq!(rate(0usize, 1usize), "0");
}
#[test]
fn verification_configuration() {
    assert!(parse_configuration("0\n32\nlanguage.md").unwrap().full);
    assert!(parse_configuration("0\n32\nlanguage.md\n").unwrap().full);
    assert!(parse_configuration("0\n32\nlanguage.md\nfull\n").unwrap().full);
    let framing = parse_configuration("2\n7\nchunk-tail.bin\nframing\n").unwrap();
    assert!(!framing.full);
    assert_eq!(framing.seconds, 2usize);
    assert_eq!(framing.pipeline, 7usize);
    assert_eq!(framing.path, "chunk-tail.bin");
    assert!(parse_configuration("0\n32\nlanguage.md\nunknown\n").is_err());
    assert!(parse_configuration("0\n32\nlanguage.md\nframing\nfull\n").is_err());
    assert!(parse_configuration("0\n0\nlanguage.md\nframing\n").is_err());
}
pub fn main(mut output: Stdout, mut input: Stdin, mut target: Connector,
    mut root: Directory, mut clock: Clock) {
    if let Err(problem) = exercise(&mut output, &mut input, &mut target, &mut root, &mut clock) {
        panic!("{problem}");
    }
}
