// Seed-compatible socket effects for the independent interpreter.
// Each relay is a small syscall-only ELF. Its byte transport has no IR knowledge.
use crate::x86::*;

pub struct Network {
    children: Vec<Child>,
    sleeper: Option<File>,
}

fn read_exact(file: &mut File, bytes: &mut [u8]) -> Result<(), i64> {
    let mut offset = 0usize;
    while offset < bytes.len() {
        let count = file.read(&mut bytes[offset..])?;
        if count == 0 { return Err(5); }
        offset += count;
    }
    Ok(())
}

fn word(bytes: &[u8]) -> u64 {
    let mut result = 0u64;
    for index in 0usize..8usize {
        result |= (bytes[index] as u64) << ((index * 8) as u64);
    }
    result
}

impl Network {
    pub fn new() -> Self {
        Self { children: Vec::new(), sleeper: None }
    }

    pub fn finish(&mut self) {
        // Host closes its stream handles first. Let each relay flush bytes that
        // successful writes already accepted before reaping its process.
        while let Some(mut child) = self.children.pop() {
            let _ = child.wait();
        }
    }

    fn reap(&mut self) {
        let mut active = Vec::new();
        while let Some(mut child) = self.children.pop() {
            if matches!(child.try_wait(), Ok(None)) {
                active.push(child);
            }
        }
        self.children = active;
    }

    fn pause(&mut self, world: &mut World, empty: &File) -> Result<(), i64> {
        if self.sleeper.is_none() {
            let mut file = world.temporary_file()?;
            file.write_all(&sleep_image()?)?;
            file.set_executable()?;
            self.sleeper = Some(file);
        }
        let executable = self.sleeper.as_ref().ok_or(5i64)?;
        let mut process = world.process();
        let arguments: Vec<Vec<u8>> = Vec::new();
        let mut child = process.spawn_file(executable, &arguments, empty, empty, empty, empty)?;
        if child.wait()? != 0 { return Err(5); }
        Ok(())
    }

    fn started(
        &mut self,
        world: &mut World,
        child: &mut Child,
        status: &mut File,
        empty: &File,
    ) -> Result<(), i64> {
        let mut bytes = [0u8; 8];
        let mut exited = false;
        loop {
            status.rewind()?;
            let count = status.read(&mut bytes)?;
            if count == 8 {
                let error = word(&bytes);
                if error == 0 { return Ok(()); }
                if error > 4095 { return Err(5); }
                return Err(error as i64);
            }
            if exited { return Err(5); }
            exited = child.try_wait()?.is_some();
            if exited { continue; }
            // The seed has no readiness or sleep operation. This startup-only
            // subprocess waits in nanosleep; the transport itself blocks in poll.
            self.pause(world, empty)?;
        }
    }

    pub fn open_stream(&mut self, world: &mut World, address: &[u8]) -> Result<File, i64> {
        if address.len() != 16 || address[0] != 2 || address[1] != 0 {
            return Err(22);
        }
        self.reap();
        let mut random_root = world.directory("/dev")?;
        let mut random = random_root.open("urandom")?;
        let mut nonce = [0u8; 32];
        read_exact(&mut random, &mut nonce)?;
        let mut listener = world.listen("127.0.0.1", 0u16)?;
        let mut local = [0u8; 16];
        let result = listener.__lkjscript_address(&mut local);
        if result < 0 { return Err(-result); }
        let image = relay_image(&local, address, &nonce)?;
        let mut executable = world.temporary_file()?;
        executable.write_all(&image)?;
        executable.set_executable()?;
        let empty = world.temporary_file()?;
        let mut startup = world.temporary_file()?;
        let mut process = world.process();
        let arguments: Vec<Vec<u8>> = Vec::new();
        let mut child = process.spawn_file(
            &executable, &arguments, &empty, &startup, &empty, &empty,
        )?;
        self.started(world, &mut child, &mut startup, &empty)?;
        let mut stream = listener.accept()?;
        let mut claimed = [0u8; 32];
        read_exact(&mut stream, &mut claimed)?;
        if claimed != nonce { return Err(13); }
        let mut response = [0u8; 8];
        read_exact(&mut stream, &mut response)?;
        let error = word(&response);
        if error != 0 {
            if error > 4095 { return Err(5); }
            return Err(error as i64);
        }
        self.children.push(child);
        Ok(stream)
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        // Early failure may still hold streams elsewhere. Child destruction
        // kills and reaps instead of blocking on their unclosed handles.
        self.children = Vec::new();
    }
}

fn append_word(bytes: &mut Vec<u8>, value: u64, width: usize) {
    for index in 0usize..width {
        bytes.push((value >> ((index * 8) as u64)) as u8);
    }
}

fn elf(code: Vec<u8>) -> Vec<u8> {
    let header = 176usize;
    let base = 0x400000u64;
    let size = (header + code.len()) as u64;
    let mut bytes = vec![0x7fu8, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    append_word(&mut bytes, 2, 2);
    append_word(&mut bytes, 62, 2);
    append_word(&mut bytes, 1, 4);
    append_word(&mut bytes, base + header as u64, 8);
    append_word(&mut bytes, 64, 8);
    append_word(&mut bytes, 0, 8);
    append_word(&mut bytes, 0, 4);
    append_word(&mut bytes, 64, 2);
    append_word(&mut bytes, 56, 2);
    append_word(&mut bytes, 2, 2);
    append_word(&mut bytes, 0, 2);
    append_word(&mut bytes, 0, 2);
    append_word(&mut bytes, 0, 2);
    append_word(&mut bytes, 1, 4); // PT_LOAD, read/execute only.
    append_word(&mut bytes, 5, 4);
    append_word(&mut bytes, 0, 8);
    append_word(&mut bytes, base, 8);
    append_word(&mut bytes, base, 8);
    append_word(&mut bytes, size, 8);
    append_word(&mut bytes, size, 8);
    append_word(&mut bytes, 4096, 8);
    append_word(&mut bytes, 0x6474e551, 4); // Nonexecutable stack.
    append_word(&mut bytes, 6, 4);
    for _index in 0usize..5usize { append_word(&mut bytes, 0, 8); }
    append_word(&mut bytes, 16, 8);
    bytes.extend_from_slice(&code);
    bytes
}

fn syscall(a: &mut Assembler, number: u64) {
    let retry = a.label();
    a.bind(retry);
    a.immediate(A, number);
    a.syscall();
    a.immediate(C, (-4i64) as u64);
    a.compare(A, C);
    a.condition(4, retry);
}

fn sleep_image() -> Result<Vec<u8>, i64> {
    let mut a = Assembler::new();
    a.lea(SP, SP, -32);
    a.immediate(A, 0);
    a.store(SP, 0, A);
    a.immediate(A, 1000000);
    a.store(SP, 8, A);
    a.mov(DI, SP);
    a.immediate(SI, 0);
    syscall(&mut a, 35);
    a.immediate(DI, 0);
    a.immediate(A, 60);
    a.syscall();
    let code = a.finish().map_err(|_| 8i64)?;
    Ok(elf(code))
}

const TO_REMOTE: i64 = -24;
const TO_REMOTE_SENT: i64 = -32;
const TO_PARENT: i64 = -40;
const TO_PARENT_SENT: i64 = -48;
const PARENT_EOF: i64 = -56;
const REMOTE_EOF: i64 = -64;
const PARENT_SHUT: i64 = -72;
const REPLY: i64 = -88;
const POLL_PARENT: i64 = -128;
const POLL_REMOTE: i64 = -120;
const PARENT_BUFFER: i64 = -65664;
const REMOTE_BUFFER: i64 = -131200;
const CHUNK: u64 = 65536;

fn interest(
    a: &mut Assembler,
    fd: u8,
    poll: i64,
    input_length: i64,
    eof: i64,
    output_length: i64,
) {
    a.immediate(R8, 0);
    let no_read = a.label();
    let no_write = a.label();
    let enabled = a.label();
    a.load(A, BP, eof);
    a.test(A, true);
    a.condition(5, no_read);
    a.load(A, BP, input_length);
    a.test(A, true);
    a.condition(5, no_read);
    a.immediate(R8, 1);
    a.bind(no_read);
    a.load(A, BP, output_length);
    a.test(A, true);
    a.condition(4, no_write);
    a.immediate(A, 4);
    a.rr(0x09, R8, A, true);
    a.bind(no_write);
    a.mov(A, fd);
    a.test(R8, true);
    a.condition(5, enabled);
    a.immediate(A, !0u64);
    a.bind(enabled);
    a.store(BP, poll, A);
    a.store_width(BP, poll + 4, R8, 2);
    a.immediate(A, 0);
    a.store_width(BP, poll + 6, A, 2);
}

fn receive(
    a: &mut Assembler,
    fd: u8,
    buffer: i64,
    length: i64,
    sent: i64,
    eof: i64,
    failed: Label,
) {
    let done = a.label();
    let got_bytes = a.label();
    a.load(A, BP, eof);
    a.test(A, true);
    a.condition(5, done);
    a.load(A, BP, length);
    a.test(A, true);
    a.condition(5, done);
    a.mov(DI, fd);
    a.lea(SI, BP, buffer);
    a.immediate(D, CHUNK);
    a.immediate(R10, 0x40); // MSG_DONTWAIT
    a.immediate(R8, 0);
    a.immediate(R9, 0);
    syscall(a, 45);
    a.immediate(C, (-11i64) as u64);
    a.compare(A, C);
    a.condition(4, done);
    a.test(A, true);
    a.condition(8, failed);
    a.condition(5, got_bytes);
    a.immediate(A, 1);
    a.store(BP, eof, A);
    a.jump(done);
    a.bind(got_bytes);
    a.store(BP, length, A);
    a.immediate(A, 0);
    a.store(BP, sent, A);
    a.bind(done);
}

fn transmit(
    a: &mut Assembler,
    fd: u8,
    buffer: i64,
    length: i64,
    sent: i64,
    failed: Label,
) {
    let done = a.label();
    a.load(D, BP, length);
    a.test(D, true);
    a.condition(4, done);
    a.load(R9, BP, sent);
    a.rr(0x29, D, R9, true);
    a.lea(SI, BP, buffer);
    a.rr(0x01, SI, R9, true);
    a.mov(DI, fd);
    a.immediate(R10, 0x4040); // MSG_DONTWAIT | MSG_NOSIGNAL
    a.immediate(R8, 0);
    a.immediate(R9, 0);
    syscall(a, 44);
    a.immediate(C, (-11i64) as u64);
    a.compare(A, C);
    a.condition(4, done);
    a.test(A, true);
    a.condition(14, failed);
    a.load(C, BP, sent);
    a.rr(0x01, A, C, true);
    a.store(BP, sent, A);
    a.load(C, BP, length);
    a.compare(A, C);
    a.condition(5, done);
    a.immediate(A, 0);
    a.store(BP, length, A);
    a.store(BP, sent, A);
    a.bind(done);
}

fn relay_image(local: &[u8], remote: &[u8], nonce: &[u8]) -> Result<Vec<u8>, i64> {
    let mut a = Assembler::new();
    let startup_failed = a.label();
    let remote_failed = a.label();
    let handshake = a.label();
    let send_all = a.label();
    let write_all = a.label();
    let failed = a.label();
    let done = a.label();
    let local_address = a.label();
    let remote_address = a.label();
    let secret = a.label();
    let next = a.label();
    a.mov(BP, SP);
    a.lea(SP, SP, -131200);
    a.immediate(A, 0);
    for offset in vec![TO_REMOTE, TO_REMOTE_SENT, TO_PARENT, TO_PARENT_SENT, PARENT_EOF, REMOTE_EOF, PARENT_SHUT] {
        a.store(BP, offset, A);
    }
    // Abrupt interpreter exit must not orphan a blocked relay or target connect.
    a.immediate(DI, 1); // PR_SET_PDEATHSIG
    a.immediate(SI, 9);
    a.immediate(D, 0);
    a.immediate(R10, 0);
    a.immediate(R8, 0);
    syscall(&mut a, 157);
    a.test(A, true);
    a.condition(8, startup_failed);
    // The initial parent channel is established before publishing startup status.
    a.immediate(DI, 2);
    a.immediate(SI, 0x80001);
    a.immediate(D, 0);
    syscall(&mut a, 41);
    a.test(A, true);
    a.condition(8, startup_failed);
    a.mov(R12, A);
    a.mov(DI, R12);
    a.address(SI, local_address);
    a.immediate(D, 16);
    syscall(&mut a, 42);
    a.test(A, true);
    a.condition(8, startup_failed);
    a.immediate(A, 0);
    a.store(BP, REPLY, A);
    a.immediate(DI, 1);
    a.lea(SI, BP, REPLY);
    a.immediate(D, 8);
    a.call(write_all);
    a.mov(DI, R12);
    a.address(SI, secret);
    a.immediate(D, nonce.len() as u64);
    a.call(send_all);
    a.immediate(DI, 2);
    a.immediate(SI, 0x80001);
    a.immediate(D, 0);
    syscall(&mut a, 41);
    a.test(A, true);
    a.condition(8, remote_failed);
    a.mov(R13, A);
    a.mov(DI, R13);
    a.address(SI, remote_address);
    a.immediate(D, 16);
    syscall(&mut a, 42);
    a.test(A, true);
    a.condition(8, remote_failed);
    a.immediate(A, 0);
    a.jump(handshake);
    a.bind(remote_failed);
    a.negate(A);
    a.bind(handshake);
    a.store(BP, REPLY, A);
    a.mov(DI, R12);
    a.lea(SI, BP, REPLY);
    a.immediate(D, 8);
    a.call(send_all);
    a.load(A, BP, REPLY);
    a.test(A, true);
    a.condition(5, done);

    a.bind(next);
    // Parent File has no half-close operation: EOF means its final owner closed.
    // Flush bytes already accepted, then release the remote socket and the child.
    let parent_open = a.label();
    a.load(A, BP, PARENT_EOF);
    a.test(A, true);
    a.condition(4, parent_open);
    a.load(A, BP, TO_REMOTE);
    a.test(A, true);
    a.condition(4, done);
    a.bind(parent_open);
    let no_shutdown = a.label();
    a.load(A, BP, REMOTE_EOF);
    a.test(A, true);
    a.condition(4, no_shutdown);
    a.load(A, BP, TO_PARENT);
    a.test(A, true);
    a.condition(5, no_shutdown);
    a.load(A, BP, PARENT_SHUT);
    a.test(A, true);
    a.condition(5, no_shutdown);
    a.mov(DI, R12);
    a.immediate(SI, 1);
    syscall(&mut a, 48);
    a.immediate(A, 1);
    a.store(BP, PARENT_SHUT, A);
    a.bind(no_shutdown);
    interest(&mut a, R12, POLL_PARENT, TO_REMOTE, PARENT_EOF, TO_PARENT);
    interest(&mut a, R13, POLL_REMOTE, TO_PARENT, REMOTE_EOF, TO_REMOTE);
    a.lea(DI, BP, POLL_PARENT);
    a.immediate(SI, 2);
    a.immediate(D, !0u64);
    syscall(&mut a, 7); // poll, no deadline and no CPU spin.
    a.test(A, true);
    a.condition(8, failed);
    receive(&mut a, R12, PARENT_BUFFER, TO_REMOTE, TO_REMOTE_SENT, PARENT_EOF, failed);
    receive(&mut a, R13, REMOTE_BUFFER, TO_PARENT, TO_PARENT_SENT, REMOTE_EOF, failed);
    transmit(&mut a, R13, PARENT_BUFFER, TO_REMOTE, TO_REMOTE_SENT, failed);
    transmit(&mut a, R12, REMOTE_BUFFER, TO_PARENT, TO_PARENT_SENT, failed);
    a.jump(next);

    a.bind(startup_failed);
    a.negate(A);
    a.store(BP, REPLY, A);
    a.immediate(DI, 1);
    a.lea(SI, BP, REPLY);
    a.immediate(D, 8);
    a.call(write_all);
    a.jump(done);

    a.bind(send_all);
    let send_more = a.label();
    a.bind(send_more);
    a.immediate(R10, 0x4000); // MSG_NOSIGNAL
    a.immediate(R8, 0);
    a.immediate(R9, 0);
    syscall(&mut a, 44);
    a.test(A, true);
    a.condition(14, failed);
    a.rr(0x01, SI, A, true);
    a.rr(0x29, D, A, true);
    a.test(D, true);
    a.condition(5, send_more);
    a.ret();

    a.bind(write_all);
    let write_more = a.label();
    a.bind(write_more);
    syscall(&mut a, 1);
    a.test(A, true);
    a.condition(14, failed);
    a.rr(0x01, SI, A, true);
    a.rr(0x29, D, A, true);
    a.test(D, true);
    a.condition(5, write_more);
    a.ret();

    a.bind(failed);
    // Reset the private stream on relay failures, rather than inventing clean EOF.
    a.immediate(A, 1);
    a.store_width(BP, REPLY, A, 4);
    a.immediate(A, 0);
    a.store_width(BP, REPLY + 4, A, 4);
    a.mov(DI, R12);
    a.immediate(SI, 1);
    a.immediate(D, 13); // SO_LINGER
    a.lea(R10, BP, REPLY);
    a.immediate(R8, 8);
    syscall(&mut a, 54);
    a.bind(done);
    a.immediate(DI, 0);
    a.immediate(A, 60);
    a.syscall();

    a.bind(local_address);
    a.raw(local);
    a.bind(remote_address);
    a.raw(remote);
    a.bind(secret);
    a.raw(nonce);
    let code = a.finish().map_err(|_| 8i64)?;
    Ok(elf(code))
}

pub fn set_nodelay(world: &mut World, file: &File, enabled: bool) -> Result<(), i64> {
    // Passing the File as stdin duplicates its descriptor without reading it.
    // Accepted sockets are direct. A relay-backed outbound File denotes the
    // interpreter's private transport leg, whose buffering this option controls.
    let mut executable = world.temporary_file()?;
    executable.write_all(&nodelay_image(enabled)?)?;
    executable.set_executable()?;
    let mut output = world.temporary_file()?;
    let empty = world.temporary_file()?;
    let arguments: Vec<Vec<u8>> = Vec::new();
    let mut process = world.process();
    let mut child = process.spawn_file(
        &executable, &arguments, file, &output, &empty, &empty,
    )?;
    if child.wait()? != 0 { return Err(5); }
    output.rewind()?;
    let mut bytes = [0u8; 8];
    read_exact(&mut output, &mut bytes)?;
    let error = word(&bytes);
    if error == 0 { return Ok(()); }
    if error > 4095 { return Err(5); }
    Err(error as i64)
}

fn nodelay_image(enabled: bool) -> Result<Vec<u8>, i64> {
    let mut a = Assembler::new();
    let more = a.label();
    let failed = a.label();
    let exit = a.label();
    a.lea(SP, SP, -16);
    a.immediate(A, enabled as u64);
    a.store_width(SP, 0, A, 4);
    a.immediate(DI, 0);
    a.immediate(SI, 6); // IPPROTO_TCP
    a.immediate(D, 1); // TCP_NODELAY
    a.mov(R10, SP);
    a.immediate(R8, 4);
    syscall(&mut a, 54);
    a.negate(A);
    a.store(SP, 0, A);
    a.immediate(DI, 1);
    a.mov(SI, SP);
    a.immediate(D, 8);
    a.bind(more);
    syscall(&mut a, 1);
    a.test(A, true);
    a.condition(14, failed);
    a.rr(0x01, SI, A, true);
    a.rr(0x29, D, A, true);
    a.test(D, true);
    a.condition(5, more);
    a.immediate(DI, 0);
    a.jump(exit);
    a.bind(failed);
    a.immediate(DI, 1);
    a.bind(exit);
    a.immediate(A, 60);
    a.syscall();
    let code = a.finish().map_err(|_| 8i64)?;
    Ok(elf(code))
}
