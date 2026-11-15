//! Run the consumer's HTTP parser without socket or filesystem work.
// bench/run.sh stages examples/http_protocol.rs beside this source.
mod common;
mod http_protocol;

fn small_check() {
    let bytes = b"HEAD /%6canguage.md?view=1 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    assert!(http_protocol::header_end(bytes, 0usize) == Some(bytes.len()));
    let request = http_protocol::parse(bytes).unwrap();
    assert!(request.path == "language.md" && request.head);
    assert!(request.body == 0usize && !request.keep_alive);
    assert!(http_protocol::parse(b"GET / HTTP/1.1\r\n\r\n").is_err());
}

pub fn main(mut world: World) {
    small_check();
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 200000u64);
    let seed = common::argument(&arguments, 1usize, 7u64);
    let mut bytes = b"GET /doc0/language.md HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: lkjscript-bench/1\r\nAccept: text/plain\r\nConnection: keep-alive\r\n\r\n"[..].to_vec();
    let original = http_protocol::parse(&bytes).unwrap();
    assert!(original.path == "doc0/language.md" && !original.head);
    assert!(original.body == 0usize && original.keep_alive);

    let mut checksum = 0u64;
    let mut iteration = 0u64;
    while iteration < count {
        // The parsed path changes each iteration and contributes to the result.
        bytes[8usize] = b'0' + (seed.wrapping_add(iteration) % 10u64) as u8;
        let end = http_protocol::header_end(&bytes, 0usize).unwrap();
        let request = http_protocol::parse(&bytes[..end]).unwrap();
        checksum = checksum.wrapping_add(
            end as u64
                + request.path.len() as u64
                + request.path.as_bytes()[3usize] as u64
                + request.body as u64
                + if request.head { 1u64 } else { 0u64 }
                + if request.keep_alive { 2u64 } else { 0u64 },
        );
        iteration += 1u64;
    }
    let output = common::decimal(checksum);
    world.stdout().write(&output);
}
