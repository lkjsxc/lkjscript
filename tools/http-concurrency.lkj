//! Concurrent HTTP load and deadline adversaries, using only granted capabilities.
//! Stdin contains measurement seconds and the number of silent peers.
//! The shell supervisor supplies fixtures, records host load/RSS and enforces a watchdog.
mod http_client;
use http_client::{io, parse_header, require};

const ACTIVE: usize = 10;
const PIPELINE: usize = 32;
const BODY: usize = 40108;
const SECOND: usize = 1_000_000_000;

struct Active {
    stream: File,
    sent: usize,
    pending: usize,
    header: Vec<u8>,
    tail: u32,
    body: usize,
    in_body: bool,
    completed: usize,
}
fn interests(client: &Active, request: &[u8]) -> u8 {
    if client.sent < request.len() { 3u8 } else { 1u8 }
}
fn completed(client: &mut Active) -> Result<(), String> {
    require(client.pending > 0usize, "unsolicited HTTP response")?;
    client.pending -= 1usize;
    client.completed += 1usize;
    client.in_body = false;
    if client.pending == 0usize {
        client.pending = PIPELINE;
        client.sent = 0usize;
    }
    Ok(())
}
fn consume(client: &mut Active, bytes: &[u8]) -> Result<(), String> {
    let mut at = 0usize;
    while at < bytes.len() {
        if client.in_body {
            let remaining = bytes.len() - at;
            let amount = if client.body < remaining { client.body } else { remaining };
            client.body -= amount;
            at += amount;
            if client.body == 0usize { completed(client)?; }
        } else {
            let byte = bytes[at];
            at += 1usize;
            client.header.push(byte);
            client.tail = (client.tail << 8u64) | byte as u32;
            if client.tail == 0x0d0a0d0au32 {
                let response = parse_header(client.header.clone(), false)?;
                client.header.clear();
                client.tail = 0u32;
                require(response.status == 200u16, "concurrent response status differs")?;
                require(response.length == BODY, "concurrent response length differs")?;
                require(!response.close, "concurrent connection closed unexpectedly")?;
                client.body = response.length;
                client.in_body = true;
                if client.body == 0usize { completed(client)?; }
            }
        }
    }
    Ok(())
}
fn dispatch(client: &mut Active, token: usize, flags: u8, ready: &mut Readiness,
    request: &[u8]) -> Result<(), String> {
    let before = interests(client, request);
    if (flags & 2u8) != 0u8 && client.sent < request.len() {
        match client.stream.write(&request[client.sent..]) {
            Ok(count) => {
                require(count > 0usize, "request write made no progress")?;
                client.sent += count;
            }
            Err(11) => {}
            Err(error) => return Err(format!("concurrent request write: errno {error}")),
        }
    }
    if (flags & 13u8) != 0u8 {
        let mut bytes = [0u8; 8192];
        match client.stream.read(&mut bytes) {
            Ok(count) => {
                require(count > 0usize, "EOF during concurrent load")?;
                consume(client, &bytes[..count])?;
            }
            Err(11) => {}
            Err(error) => return Err(format!("concurrent response read: errno {error}")),
        }
    }
    let after = interests(client, request);
    if after != before {
        io(ready.watch_file(&mut client.stream, token, after), "update client interest")?;
    }
    Ok(())
}
fn totals(clients: &[Active]) -> Vec<usize> {
    let mut result = Vec::new();
    for client in clients { result.push(client.completed); }
    result
}
fn report(output: &mut Stdout, name: &str, clients: &[Active], before: &[usize],
    elapsed: usize) -> Result<(), String> {
    let mut count = 0usize;
    let mut minimum = !0usize;
    let mut maximum = 0usize;
    for index in 0usize..clients.len() {
        let current = clients[index].completed - before[index];
        count += current;
        if current < minimum { minimum = current; }
        if current > maximum { maximum = current; }
    }
    require(minimum > 0usize, "an active connection was starved")?;
    let numerator = count.checked_mul(SECOND).ok_or("measurement rate overflow".to_string())?;
    let rate = numerator / elapsed;
    output.write(format!("ok concurrent {name}: {count} responses, {elapsed} ns, {rate} requests/s; per-client min/max {minimum}/{maximum}; framing verified, body bytes drained\n").as_bytes());
    Ok(())
}
fn phase(output: &mut Stdout, name: &str, clients: &mut Vec<Active>,
    ready: &mut Readiness, clock: &mut Clock, request: &[u8], seconds: usize) -> Result<(), String> {
    let before = totals(clients);
    let start = io(clock.nanoseconds(), "phase clock")?;
    let deadline = start.checked_add(seconds * SECOND).ok_or("phase deadline overflow".to_string())?;
    output.write(format!("phase {name}\n").as_bytes());
    loop {
        if io(clock.nanoseconds(), "phase clock")? >= deadline { break; }
        let events = io(ready.wait(clock, deadline), "wait for concurrent clients")?;
        for (token, flags) in events {
            require(token < clients.len(), "unknown active readiness token")?;
            dispatch(&mut clients[token], token, flags, ready, request)?;
        }
    }
    report(output, name, clients, &before, io(clock.nanoseconds(), "phase clock")? - start)
}
fn close_notice(output: &mut Stdout, name: &str, elapsed: usize) -> Result<(), String> {
    output.write(format!("ok {name} closed after {elapsed} ns; application deadline 2000000000 ns\n").as_bytes());
    require(elapsed <= 4usize * SECOND, "hostile client survived its deadline allowance")
}
fn hostile(output: &mut Stdout, target: &mut Connector, clients: &mut Vec<Active>,
    ready: &mut Readiness, clock: &mut Clock, request: &[u8],
    inspect: &mut Directory, base_owned: usize) -> Result<(), String> {
    let before = totals(clients);
    let mut slow = io(target.connect(), "connect slow header")?;
    io(slow.write_all(b"G"), "start slow header")?;
    io(slow.set_nonblocking(true), "make slow peer nonblocking")?;
    io(ready.watch_file(&mut slow, ACTIVE, 1u8), "watch slow peer")?;
    let mut blocked = io(target.connect(), "connect nonreader")?;
    io(blocked.write_all(b"GET /blocked.bin HTTP/1.1\r\nHost: localhost\r\n\r\n"), "start blocked response")?;
    io(blocked.set_nonblocking(true), "make blocked peer nonblocking")?;
    io(ready.watch_file(&mut blocked, ACTIVE + 1usize, 1u8), "watch response prefix")?;
    {
        let mut vanished = io(target.connect(), "connect vanishing peer")?;
        io(vanished.write_all(b"GET /load.bin HTTP/1.1\r\nHost:"), "start vanishing peer")?;
    }
    let start = io(clock.nanoseconds(), "hostile clock")?;
    let end = start + 5usize * SECOND;
    let mut next_byte = start + SECOND;
    let mut sent_bytes = 1usize;
    let mut slow_closed = false;
    let mut blocked_closed = false;
    let mut blocked_bytes = 0usize;
    let mut paused = false;
    let mut resumed = false;
    let mut resume_at = end;
    let mut next_inspect = start;
    let mut saw_hostile_descriptors = false;
    let mut server_released = false;
    output.write(b"phase hostile\n");
    loop {
        let now = io(clock.nanoseconds(), "hostile clock")?;
        if now >= end { break; }
        if !server_released && now >= next_inspect {
            let owned = io(inspect.list(), "inspect owned server descriptors")?.len();
            if owned >= base_owned + ACTIVE + 2usize { saw_hostile_descriptors = true; }
            if saw_hostile_descriptors && owned <= base_owned + ACTIVE {
                let elapsed = io(clock.nanoseconds(), "server descriptor clock")? - start;
                output.write(format!("ok all hostile server descriptors released after {elapsed} ns; fixed deadline 2000000000 ns\n").as_bytes());
                require(elapsed <= 3usize * SECOND, "server retained a hostile descriptor past its deadline allowance")?;
                server_released = true;
            }
            next_inspect = now + 50_000_000usize;
        }
        if !slow_closed && now >= next_byte {
            match slow.write(b"x") {
                Ok(1) => { sent_bytes += 1usize; }
                Ok(_) => return Err("slow header write made no progress".to_string()),
                Err(11) => {}
                Err(32) | Err(104) => {
                    slow_closed = true;
                    io(ready.unwatch_file(&mut slow), "unwatch closed slow peer")?;
                    close_notice(output, "slow header", now - start)?;
                }
                Err(error) => return Err(format!("slow header write: errno {error}")),
            }
            next_byte += SECOND;
        }
        if paused && !resumed && now >= resume_at {
            io(ready.watch_file(&mut blocked, ACTIVE + 1usize, 1u8), "resume blocked response drain")?;
            resumed = true;
        }
        let mut deadline = end;
        if !slow_closed && next_byte < deadline { deadline = next_byte; }
        if paused && !resumed && resume_at < deadline { deadline = resume_at; }
        if !server_released && next_inspect < deadline { deadline = next_inspect; }
        let events = io(ready.wait(clock, deadline), "wait during hostile clients")?;
        for (token, flags) in events {
            if token < ACTIVE {
                dispatch(&mut clients[token], token, flags, ready, request)?;
            } else if token == ACTIVE && !slow_closed {
                let mut byte = [0u8; 512];
                let closed = match slow.read(&mut byte) {
                    Ok(0) => true,
                    Ok(_) => false,
                    Err(11) => false,
                    Err(104) => true,
                    Err(error) => return Err(format!("read slow peer: errno {error}")),
                };
                if closed {
                    slow_closed = true;
                    io(ready.unwatch_file(&mut slow), "unwatch slow peer")?;
                    close_notice(output, "slow header", io(clock.nanoseconds(), "slow close clock")? - start)?;
                }
            } else if token == ACTIVE + 1usize && !blocked_closed {
                let mut bytes = [0u8; 65536];
                let closed = match blocked.read(&mut bytes) {
                    Ok(0) => true,
                    Ok(count) => { blocked_bytes += count; false }
                    Err(11) => false,
                    Err(104) => true,
                    Err(error) => return Err(format!("read blocked peer: errno {error}")),
                };
                let observed = io(clock.nanoseconds(), "blocked clock")?;
                if closed {
                    blocked_closed = true;
                    io(ready.unwatch_file(&mut blocked), "unwatch blocked peer")?;
                    require(paused && resumed, "large response closed before the nonreader test")?;
                    require(blocked_bytes < 32usize * 1024usize * 1024usize, "nonreader received an entire blocked response")?;
                    close_notice(output, "nonreading response", observed - start)?;
                } else if !paused && blocked_bytes >= 1024usize {
                    io(ready.unwatch_file(&mut blocked), "pause response reads")?;
                    paused = true;
                    resume_at = observed + 2_500_000_000usize;
                }
            } else {
                return Err("unknown hostile readiness token".to_string());
            }
        }
    }
    require(slow_closed && blocked_closed, "hostile client stayed open after deadline")?;
    require(server_released, "server retained a hostile client descriptor")?;
    require(sent_bytes >= 2usize, "slow header did not drip across a second")?;
    output.write(format!("ok hostile peers: slow header {sent_bytes} bytes at one per second; nonreader drained {blocked_bytes} bytes before EOF; vanished peer released its partial request\n").as_bytes());
    report(output, "hostile", clients, &before, io(clock.nanoseconds(), "hostile clock")? - start)
}
fn exercise(output: &mut Stdout, input: &mut Stdin, target: &mut Connector,
    ready: &mut Readiness, clock: &mut Clock, inspect: &mut Directory) -> Result<(), String> {
    let text = match String::from_utf8(io(input.read_to_end(), "read concurrency configuration")?) {
        Ok(text) => text,
        Err(_) => return Err("concurrency configuration is not UTF-8".to_string()),
    };
    let mut lines = text.as_str().split('\n');
    let seconds = match lines.next().unwrap_or("").parse::<usize>() {
        Ok(value) => value,
        Err(_) => return Err("invalid measurement seconds".to_string()),
    };
    let idle = match lines.next().unwrap_or("").parse::<usize>() {
        Ok(value) => value,
        Err(_) => return Err("invalid silent peer count".to_string()),
    };
    require(seconds > 0usize && seconds < 45usize, "measurement seconds must be 1..44 to fit the server idle deadline")?;
    for line in lines { require(line.is_empty(), "extra concurrency configuration")?; }
    let mut request = Vec::new();
    for _index in 0usize..PIPELINE {
        request.extend_from_slice(b"GET /load.bin HTTP/1.1\r\nHost: localhost\r\n\r\n");
    }
    let base_owned = io(inspect.list(), "inspect unloaded server descriptors")?.len();
    let mut clients = Vec::new();
    for token in 0usize..ACTIVE {
        let mut stream = io(target.connect(), "connect active peer")?;
        io(stream.set_nonblocking(true), "make active peer nonblocking")?;
        io(ready.watch_file(&mut stream, token, 3u8), "watch active peer")?;
        clients.push(Active {
            stream, sent: 0usize, pending: PIPELINE, header: Vec::new(),
            tail: 0u32, body: 0usize, in_body: false, completed: 0usize,
        });
    }
    phase(output, "warmup", &mut clients, ready, clock, &request, 1usize)?;
    phase(output, "baseline-before", &mut clients, ready, clock, &request, seconds)?;
    let mut silent = Vec::new();
    for _index in 0usize..idle { silent.push(io(target.connect(), "connect silent peer")?); }
    output.write(format!("open peers: {} total, {idle} silent, {ACTIVE} active\n", idle + ACTIVE).as_bytes());
    phase(output, "idle-warmup", &mut clients, ready, clock, &request, 1usize)?;
    phase(output, "idle-loaded", &mut clients, ready, clock, &request, seconds)?;
    silent.clear();
    phase(output, "baseline-after", &mut clients, ready, clock, &request, seconds)?;
    hostile(output, target, &mut clients, ready, clock, &request, inspect, base_owned)?;
    for index in 0usize..clients.len() {
        io(ready.unwatch_file(&mut clients[index].stream), "unwatch active peer")?;
    }
    clients.clear();
    Ok(())
}
pub fn main(mut output: Stdout, mut input: Stdin, mut target: Connector,
    mut ready: Readiness, mut clock: Clock, mut inspect: Directory) {
    match exercise(&mut output, &mut input, &mut target, &mut ready, &mut clock, &mut inspect) {
        Ok(()) => output.write(b"phase closed\nok concurrent HTTP and deadline cases\n"),
        Err(problem) => panic!("{problem}"),
    }
}
