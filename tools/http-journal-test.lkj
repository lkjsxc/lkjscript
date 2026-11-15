//! SIGKILL journal campaign. The supervisor and all HTTP traffic are lkjscript.
//! Each round kills a writer with requests in flight, kills its next recovery
//! after an observed replay checkpoint, then verifies every acknowledgement.
mod http_client;
mod http_journal_peer;
mod http_journal_model;
mod http_journal_auth;
use http_client::{find, io, require};
use http_journal_peer::{Peer, exchange};
use http_journal_model::{Model, Snapshot};
use std::json::Value;

struct Random { state: u64 }
impl Random {
    fn next(&mut self) -> usize {
        self.state ^= self.state << 13u64;
        self.state ^= self.state >> 7u64;
        self.state ^= self.state << 17u64;
        self.state as usize
    }
}
struct Server {
    child: Child,
    output: File,
    errors: File,
    transcript: Vec<u8>,
    port: u16,
}
impl Server {
    fn poll(&mut self) -> Result<(), String> {
        loop {
            let mut bytes = [0u8; 4096];
            let count = io(self.output.read(&mut bytes), "read server progress")?;
            if count == 0usize { return Ok(()); }
            self.transcript.extend_from_slice(&bytes[..count]);
        }
    }
    fn alive(&mut self) -> Result<(), String> {
        if let Some(status) = io(self.child.try_wait(), "poll server process")? {
            let bytes = io(self.errors.read_to_end(), "read server failure")?;
            let errors = String::from_utf8(bytes).unwrap_or("invalid error bytes".to_string());
            return Err(format!("journal exited with status {status}: {errors}"));
        }
        Ok(())
    }
}
struct Harness {
    process: Process,
    clock: Clock,
    ready: Readiness,
    executable: File,
    data: Directory,
    output: Stdout,
    started: usize,
    reaped: usize,
    repairs: usize,
    recovery_misses: usize,
    token: String,
}
impl Harness {
    fn spawn(&mut self, world: &mut World) -> Result<Server, String> {
        let mut output = io(world.temporary_file(), "capture server output")?;
        let reader = io(output.readonly(), "independent server output reader")?;
        let mut errors = io(world.temporary_file(), "capture server errors")?;
        let error_reader = io(errors.readonly(), "independent server error reader")?;
        let input = io(world.temporary_file(), "server empty stdin")?;
        let result = io(world.temporary_file(), "server result stream")?;
        let args: Vec<Vec<u8>> = Vec::new();
        let child = io(self.process.spawn_file(&self.executable, &args, &input, &output, &errors, &result),
            "spawn journal server")?;
        self.started += 1usize;
        Ok(Server { child, output: reader, errors: error_reader, transcript: Vec::new(), port: 0u16 })
    }
    fn start(&mut self, world: &mut World) -> Result<Server, String> {
        let mut server = self.spawn(world)?;
        let deadline = io(self.clock.nanoseconds(), "start recovery clock")? + 60_000_000_000usize;
        loop {
            server.poll()?;
            let port = {
                let mut parsed = None;
                if let Some(at) = find(&server.transcript, b"\nport ") {
                    let text = &server.transcript[at + 6usize..];
                    if let Some(end) = find(text, b"\n") {
                        parsed = Some(match String::from_utf8(text[..end].to_vec()).unwrap().as_str().parse::<u16>() {
                            Ok(port) => port,
                            Err(_) => return Err("invalid server port".to_string()),
                        });
                    }
                }
                parsed
            };
            if let Some(port) = port {
                server.port = port;
                require(find(&server.transcript, b"recovery done ").is_some(), "service started before recovery")?;
                if find(&server.transcript, b"recovery repaired ").is_some() { self.repairs += 1usize; }
                let now = io(self.clock.unix_seconds(), "session wall clock")?;
                let mut entropy = world.entropy();
                self.token = http_journal_auth::issue(&mut self.data, &mut entropy, now, 3600i64)?;
                return Ok(server);
            }
            server.alive()?;
            let now = io(self.clock.nanoseconds(), "recovery watchdog")?;
            require(now < deadline, "journal startup watchdog expired")?;
            io(self.ready.wait(&mut self.clock, now + 1_000_000usize), "wait for startup")?;
        }
    }
    fn kill(&mut self, server: &mut Server) -> Result<(), String> {
        io(server.child.terminate(), "SIGKILL journal")?;
        let status = io(server.child.wait(), "reap journal")?;
        self.reaped += 1usize;
        require(status == 137i64, &format!("expected SIGKILL status 137, got {status}"))?;
        server.poll()
    }
    fn request(&mut self, world: &mut World, port: u16, method: &str, path: &str,
        body: &[u8], expected: u16) -> Result<Value, String> {
        exchange(world, port, &mut self.clock, &mut self.ready, &self.token, method, path, body, expected)
    }
    fn mutate(&mut self, world: &mut World, port: u16, model: &mut Model,
        id: usize, size: usize) -> Result<(), String> {
        let index = model.issue(id, size);
        let body = model.body(index)?;
        model.attempts[index].sent = true;
        let path = if id == 0usize { "/entries".to_string() } else { format!("/entries/{id}") };
        let method = if id == 0usize { "POST" } else { "PUT" };
        let expected = if id == 0usize { 201u16 } else { 200u16 };
        let value = self.request(world, port, method, &path, &body, expected)?;
        model.acknowledge(index, &value)
    }
    fn verify(&mut self, world: &mut World, server: &Server, model: &mut Model) -> Result<Snapshot, String> {
        let snapshot = model.disk(&mut self.data)?;
        let empty: Vec<u8> = Vec::new();
        let all = self.request(world, server.port, "GET", "/entries", &empty, 200u16)?;
        http_journal_model::all(&all, &snapshot)?;
        let status = self.request(world, server.port, "GET", "/status", &empty, 200u16)?;
        require(http_journal_model::number(&status, "entries")? == snapshot.entries.len()
            && http_journal_model::number(&status, "next_id")? == snapshot.entries.len() + 1usize
            && http_journal_model::number(&status, "revision")? == snapshot.revision,
            "HTTP status differs from durable replay")?;
        if !snapshot.entries.is_empty() {
            let single = self.request(world, server.port, "GET", "/entries/1", &empty, 200u16)?;
            http_journal_model::entry(&single, &snapshot.entries[0usize])?;
            let range = self.request(world, server.port, "GET", "/entries/1/2", &empty, 200u16)?;
            let values = range.get("entries").unwrap().as_array().unwrap();
            require(values.len() == 1usize, "range is not inclusive-first/exclusive-end")?;
            http_journal_model::entry(&values[0usize], &snapshot.entries[0usize])?;
        }
        Ok(snapshot)
    }
    fn recovery_kill(&mut self, world: &mut World, random: &mut Random, known_revision: usize) -> Result<(), String> {
        let mut attempts = 0usize;
        loop {
            let mut server = self.spawn(world)?;
            let segment = random.next() % (known_revision / 128usize + 1usize);
            let checkpoint = if segment == 0usize { 1usize } else { segment * 128usize };
            let marker = format!("recovery record {checkpoint} ");
            let deadline = io(self.clock.nanoseconds(), "start recovery-kill watchdog")? + 60_000_000_000usize;
            loop {
                server.poll()?;
                if find(&server.transcript, marker.as_bytes()).is_some()
                    || find(&server.transcript, b"recovery done ").is_some() { break; }
                server.alive()?;
                require(io(self.clock.nanoseconds(), "recovery-kill watchdog")? < deadline,
                    "no recovery progress before watchdog")?;
            }
            let delay = random.next() % 200_001usize;
            let target = io(self.clock.nanoseconds(), "recovery checkpoint time")? + delay;
            while io(self.clock.nanoseconds(), "random recovery kill time")? < target {}
            self.kill(&mut server)?;
            if find(&server.transcript, b"recovery repaired ").is_some() { self.repairs += 1usize; }
            // Read the output again after wait: a completion racing the signal
            // makes this a missed window, never a claimed recovery kill.
            if find(&server.transcript, marker.as_bytes()).is_some()
                && find(&server.transcript, b"recovery done ").is_none() {
                return Ok(());
            }
            self.recovery_misses += 1usize;
            attempts += 1usize;
            require(attempts < 100usize, "could not hit an observed recovery window")?;
        }
    }
}
fn enqueue(peer: &mut Peer, model: &mut Model, pending: &mut Vec<(usize, usize)>,
    random: &mut Random, entries: usize, token: &str) -> Result<(), String> {
    for _ in 0usize..16usize {
        let value = random.next();
        let id = if value % 8usize == 0usize { 0usize } else { 1usize + value % entries };
        let index = model.issue(id, 128usize + random.next() % 1921usize);
        let body = model.body(index)?;
        let path = if id == 0usize { "/entries".to_string() } else { format!("/entries/{id}") };
        let method = if id == 0usize { "POST" } else { "PUT" };
        let end = peer.queue(&http_journal_peer::request(token, method, &path, &body, false));
        pending.push((index, end));
    }
    Ok(())
}
fn collect(peer: &mut Peer, model: &mut Model, pending: &[(usize, usize)],
    completed: &mut usize) -> Result<(), String> {
    for item in pending {
        if item.1 <= peer.sent { model.attempts[item.0].sent = true; }
    }
    loop {
        let (response, bytes) = match peer.response()? {
            Some(value) => value,
            None => return Ok(()),
        };
        require(*completed < pending.len(), "unsolicited mutation response")?;
        let index = pending[*completed].0;
        let expected = if model.attempts[index].id == 0usize { 201u16 } else { 200u16 };
        require(response.status == expected, "mutation response was not successful")?;
        let value = http_journal_peer::json(&bytes)?;
        model.acknowledge(index, &value)?;
        *completed += 1usize;
    }
}
fn write_kill(harness: &mut Harness, world: &mut World, server: &mut Server,
    model: &mut Model, random: &mut Random, entries: usize) -> Result<usize, String> {
    let mut peer = Peer::connect(world, server.port)?;
    let mut pending: Vec<(usize, usize)> = Vec::new();
    let mut completed = 0usize;
    enqueue(&mut peer, model, &mut pending, random, entries, &harness.token)?;
    let start = io(harness.clock.nanoseconds(), "write-load start")?;
    let deadline = start + 250_000usize + random.next() % 4_750_001usize;
    loop {
        peer.step()?;
        collect(&mut peer, model, &pending, &mut completed)?;
        require(!peer.ended, "server closed while write load was active")?;
        if completed == pending.len() { enqueue(&mut peer, model, &mut pending, random, entries, &harness.token)?; }
        let now = io(harness.clock.nanoseconds(), "write-kill deadline")?;
        require(now - start < 30_000_000_000usize, "no in-flight write before kill watchdog")?;
        // A queued request alone is not load. At the signal, at least one complete
        // request has reached the socket and its complete response is unobserved.
        if now >= deadline && completed < pending.len() && pending[completed].1 <= peer.sent { break; }
        peer.wait(&mut harness.clock, &mut harness.ready, deadline)?;
    }
    let delay = io(harness.clock.nanoseconds(), "actual write kill time")? - start;
    harness.kill(server)?;
    let drain_deadline = io(harness.clock.nanoseconds(), "drain acknowledgement start")? + 5_000_000_000usize;
    loop {
        peer.step()?;
        collect(&mut peer, model, &pending, &mut completed)?;
        if peer.ended { break; }
        require(io(harness.clock.nanoseconds(), "drain acknowledgement watchdog")? < drain_deadline,
            "killed server retained a connection")?;
        peer.wait(&mut harness.clock, &mut harness.ready, drain_deadline)?;
    }
    peer.unwatch(&mut harness.ready)?;
    Ok(delay)
}
fn exercise(world: &mut World, path: &str, cycles: usize, seed: u64) -> Result<(), String> {
    let mut directory = io(world.directory(path), "open supervisor directory")?;
    let executable = io(directory.open("server"), "open journal executable")?;
    let data = io(directory.open_directory("data"), "open journal data")?;
    let mut harness = Harness { process: world.process(), clock: world.clock(),
        ready: io(world.readiness(), "supervisor wait set")?, executable, data, output: world.stdout(),
        started: 0usize, reaped: 0usize, repairs: 0usize, recovery_misses: 0usize, token: String::new() };
    let mut random = Random { state: seed };
    let mut model = Model::new();
    let mut server = harness.start(world)?;
    let empty: Vec<u8> = Vec::new();
    harness.request(world, server.port, "GET", "/entries/1", &empty, 404u16)?;
    harness.request(world, server.port, "PUT", "/entries/1", b"{\"text\":\"missing\"}", 404u16)?;
    harness.request(world, server.port, "POST", "/entries", b"{\"text\":\"a\",\"text\":\"b\"}", 400u16)?;
    harness.request(world, server.port, "POST", "/entries", b"{\"text\":\"\\ud800\"}", 400u16)?;
    harness.request(world, server.port, "POST", "/entries", b"{\"text\":1}", 400u16)?;
    harness.request(world, server.port, "GET", "/entries/0", &empty, 400u16)?;
    for _ in 0usize..128usize { harness.mutate(world, server.port, &mut model, 0usize, 1024usize)?; }
    harness.mutate(world, server.port, &mut model, 1usize, 4096usize)?;
    let mut snapshot = harness.verify(world, &server, &mut model)?;
    let start = io(harness.clock.nanoseconds(), "campaign start")?;
    let mut shortest = !0usize;
    let mut longest = 0usize;
    for cycle in 0usize..cycles {
        // Each round has a freshly acknowledged mutation before the random
        // in-flight batch, so every restart must retain new acknowledged data.
        let id = 1usize + random.next() % snapshot.entries.len();
        harness.mutate(world, server.port, &mut model, id, 256usize)?;
        let delay = write_kill(&mut harness, world, &mut server, &mut model, &mut random, snapshot.entries.len())?;
        if delay < shortest { shortest = delay; }
        if delay > longest { longest = delay; }
        drop(server);
        harness.recovery_kill(world, &mut random, snapshot.revision)?;
        server = harness.start(world)?;
        snapshot = harness.verify(world, &server, &mut model)?;
        if (cycle + 1usize) % 100usize == 0usize || cycle + 1usize == cycles {
            harness.output.write(format!("journal progress {}: acknowledged {}, records {}, entries {}, log {} bytes, recovery-window misses {}\n",
                cycle + 1usize, model.acknowledgements, snapshot.revision, snapshot.entries.len(),
                snapshot.bytes, harness.recovery_misses).as_bytes());
        }
    }
    let elapsed = io(harness.clock.nanoseconds(), "campaign finish")? - start;
    harness.kill(&mut server)?;
    require(harness.started == harness.reaped, "supervisor retained a child")?;
    harness.output.write(format!("ok journal: {cycles} write-load SIGKILLs, {cycles} confirmed recovery SIGKILLs; {} acknowledged mutations, {} complete records, {} verified frames across {} snapshots; {} observed tail repairs; {} recovery-window misses\n",
        model.acknowledgements, snapshot.revision, model.verified_frames, model.snapshots,
        harness.repairs, harness.recovery_misses).as_bytes());
    harness.output.write(format!("journal timing: {} ns; write-kill delay min/max {shortest}/{longest} ns; seed {seed}\n", elapsed).as_bytes());
    harness.output.write(format!("journal children: {} started, {} SIGKILLed and reaped, zero retained; final fresh port {}\n",
        harness.started, harness.reaped, server.port).as_bytes());
    Ok(())
}
pub fn main(mut world: World) {
    let args = world.args();
    if args.len() != 3usize {
        world.stderr().write(b"usage: http-journal-test WORK_DIRECTORY CYCLES NONZERO_SEED\n");
        world.set_exit_status(2u8);
        return;
    }
    let path = String::from_utf8(args[0usize].clone()).unwrap();
    let cycles = String::from_utf8(args[1usize].clone()).unwrap().as_str().parse::<usize>().unwrap();
    let seed = String::from_utf8(args[2usize].clone()).unwrap().as_str().parse::<u64>().unwrap();
    assert!(cycles > 0usize && seed != 0u64);
    if let Err(problem) = exercise(&mut world, &path, cycles, seed) {
        world.stderr().write(format!("FAIL journal: {problem}\n").as_bytes());
        world.set_exit_status(1u8);
    }
}
