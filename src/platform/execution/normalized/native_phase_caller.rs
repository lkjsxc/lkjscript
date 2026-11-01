//! Independent test caller for the exported native phase ABI. Production
//! instructions stay unchanged; only ELF entry/extents and appended data differ.
use super::{Check, ensure};
const AX: u8 = 0;
const CX: u8 = 1;
const DX: u8 = 2;
const BX: u8 = 3;
const SP: u8 = 4;
const BP: u8 = 5;
const SI: u8 = 6;
const DI: u8 = 7;
const R11: u8 = 11;
const R12: u8 = 12;
const R13: u8 = 13;
const R14: u8 = 14;
const R15: u8 = 15;
const E: u8 = 4;
const NE: u8 = 5;
const LE: u8 = 14;
const BASE: u64 = 0x400000;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Compiler,
    Echo,
    ObserverLeak,
    AllocationFree,
    RuntimeReject,
}
#[derive(Default)]
struct Code {
    bytes: Vec<u8>,
    labels: Vec<Option<usize>>,
    relative: Vec<(usize, usize)>,
    absolute: Vec<(usize, usize)>,
}
impl Code {
    fn raw(&mut self, b: &[u8]) {
        self.bytes.extend_from_slice(b);
    }
    fn word(&mut self, n: u64) {
        self.raw(&n.to_le_bytes());
    }
    fn small(&mut self, n: i32) {
        self.raw(&n.to_le_bytes());
    }
    fn label(&mut self) -> usize {
        let n = self.labels.len();
        self.labels.push(None);
        n
    }
    fn bind(&mut self, n: usize) -> Check<()> {
        let slot = self.labels.get_mut(n).ok_or("caller label index")?;
        ensure(slot.is_none(), "duplicate caller label")?;
        *slot = Some(self.bytes.len());
        Ok(())
    }
    fn rex(&mut self, r: u8, b: u8) {
        self.raw(&[0x48 | ((r >> 3) << 2) | (b >> 3)]);
    }
    fn immediate(&mut self, r: u8, n: u64) {
        self.rex(0, r);
        self.raw(&[0xb8 + (r & 7)]);
        self.word(n);
    }
    fn address(&mut self, r: u8, n: usize) {
        self.rex(0, r);
        self.raw(&[0xb8 + (r & 7)]);
        self.absolute.push((self.bytes.len(), n));
        self.word(0);
    }
    fn binary(&mut self, op: u8, d: u8, s: u8) {
        self.rex(s, d);
        self.raw(&[op, 0xc0 | ((s & 7) << 3) | (d & 7)]);
    }
    fn mov(&mut self, d: u8, s: u8) {
        self.binary(0x89, d, s);
    }
    fn memory(&mut self, op: u8, r: u8, b: u8, n: i32) {
        self.rex(r, b);
        self.raw(&[op, 0x80 | ((r & 7) << 3) | (b & 7)]);
        if b & 7 == 4 {
            self.raw(&[0x24]);
        }
        self.small(n);
    }
    fn load(&mut self, d: u8, b: u8, n: i32) {
        self.memory(0x8b, d, b, n);
    }
    fn store(&mut self, b: u8, n: i32, s: u8) {
        self.memory(0x89, s, b, n);
    }
    fn lea(&mut self, d: u8, b: u8, n: i32) {
        self.memory(0x8d, d, b, n);
    }
    fn group(&mut self, op: u8, r: u8, n: i32) {
        self.rex(0, r);
        self.raw(&[0x81, 0xc0 | (op << 3) | (r & 7)]);
        self.small(n);
    }
    fn branch(&mut self, prefix: &[u8], n: usize) {
        self.raw(prefix);
        self.relative.push((self.bytes.len(), n));
        self.small(0);
    }
    fn jump(&mut self, n: usize) {
        self.branch(&[0xe9], n);
    }
    fn condition(&mut self, c: u8, n: usize) {
        self.branch(&[0x0f, 0x80 + c], n);
    }
    fn call(&mut self, n: usize) {
        self.branch(&[0xe8], n);
    }
    fn syscall(&mut self) {
        self.raw(&[0x0f, 0x05]);
    }
    fn set(&mut self, n: i32, v: u64) {
        self.immediate(AX, v);
        self.store(R13, n, AX);
    }
    fn checked_call(&mut self, address: u64, fatal: usize) {
        // R14 names the live outer output context. Every callee-saved register
        // and the stack pointer must survive normal, rejected and cancelled calls.
        for (r, v) in [
            (BX, 0x1a2b3c4d),
            (BP, 0x21324354),
            (R12, 0x31425364),
            (R15, 0x41526374),
        ] {
            self.immediate(r, v);
        }
        self.lea(R14, R13, 64);
        self.immediate(R11, address);
        self.raw(&[0x41, 0xff, 0xd3]);
        self.store(R13, 392, AX);
        self.binary(0x39, SP, R13);
        self.condition(NE, fatal);
        for (r, v) in [
            (BX, 0x1a2b3c4d),
            (BP, 0x21324354),
            (R12, 0x31425364),
            (R15, 0x41526374),
        ] {
            self.group(7, r, v);
            self.condition(NE, fatal);
        }
        self.lea(AX, R13, 64);
        self.binary(0x39, R14, AX);
        self.condition(NE, fatal);
    }
    fn invoke(
        &mut self,
        t: &Table,
        offset: i32,
        payload: usize,
        bytes: u64,
        cancel: bool,
        fatal: usize,
    ) {
        self.address(DI, payload);
        self.immediate(SI, bytes);
        self.lea(DX, R13, offset);
        if cancel {
            self.lea(CX, R13, 352);
        } else {
            self.immediate(CX, 0);
        }
        self.checked_call(t.invoke, fatal);
    }
    fn release(&mut self, t: &Table, offset: i32, fatal: usize) {
        self.lea(DI, R13, offset);
        self.checked_call(t.release, fatal);
    }
    fn frame(&mut self, id: u64, offset: i32, write: usize, fatal: usize) -> Check<()> {
        let empty = self.label();
        let ready = self.label();
        let done = self.label();
        self.set(384, id);
        self.store(R13, 416, R13);
        self.load(R12, R13, offset + 120);
        self.group(7, R12, 0);
        self.condition(E, empty);
        self.load(AX, R12, 0);
        self.store(R13, 400, AX);
        self.load(AX, R12, 8);
        self.store(R13, 408, AX);
        self.jump(ready);
        self.bind(empty)?;
        self.set(400, u64::MAX);
        self.set(408, 0);
        self.bind(ready)?;
        self.lea(SI, R13, 384);
        self.immediate(DX, 40);
        self.call(write);
        self.mov(SI, R13);
        self.immediate(DX, 320);
        self.call(write);
        self.load(DX, R13, 408);
        self.group(7, DX, 0);
        self.condition(E, done);
        self.load(SI, R12, 16);
        self.call(write);
        self.bind(done)?;
        self.immediate(AX, 0);
        self.immediate(DI, 0);
        self.lea(SI, R13, 432);
        self.immediate(DX, 1);
        self.syscall();
        self.group(7, AX, 1);
        self.condition(NE, fatal);
        Ok(())
    }
    fn finish(mut self, offset: usize) -> Check<Vec<u8>> {
        for (at, label) in self.relative {
            let destination = self
                .labels
                .get(label)
                .copied()
                .flatten()
                .ok_or("unbound caller label")?;
            let distance = i64::try_from(destination).map_err(|_| "caller label extent")?
                - i64::try_from(at + 4).map_err(|_| "caller patch extent")?;
            let distance = i32::try_from(distance).map_err(|_| "caller rel32 extent")?;
            self.bytes
                .get_mut(at..at + 4)
                .ok_or("caller rel32 patch")?
                .copy_from_slice(&distance.to_le_bytes());
        }
        for (at, label) in self.absolute {
            let destination = self
                .labels
                .get(label)
                .copied()
                .flatten()
                .ok_or("unbound caller address")?;
            let address = BASE
                .checked_add(
                    u64::try_from(
                        offset
                            .checked_add(destination)
                            .ok_or("caller address overflow")?,
                    )
                    .map_err(|_| "caller address domain")?,
                )
                .ok_or("caller address overflow")?;
            self.bytes
                .get_mut(at..at + 8)
                .ok_or("caller address patch")?
                .copy_from_slice(&address.to_le_bytes());
        }
        Ok(self.bytes)
    }
}
pub(super) fn word(bytes: &[u8], offset: usize) -> Check<u64> {
    let value: [u8; 8] = bytes
        .get(offset..offset + 8)
        .ok_or("phase word extent")?
        .try_into()
        .map_err(|_| "phase word width")?;
    Ok(u64::from_le_bytes(value))
}
struct Table {
    invoke: u64,
    release: u64,
}
fn table(image: &[u8]) -> Check<Table> {
    ensure(
        image.get(256..264) == Some(b"LKJPH001".as_slice()),
        "phase ABI identity",
    )?;
    for (offset, expected) in [(264, 64), (272, 160), (280, 24), (304, 1), (312, 0)] {
        ensure(word(image, offset)? == expected, "phase ABI layout/flags")?;
    }
    ensure(word(image, 80)? == BASE, "phase caller static load base")?;
    let end = BASE
        .checked_add(u64::try_from(image.len()).map_err(|_| "image extent")?)
        .ok_or("image address overflow")?;
    let table = Table {
        invoke: word(image, 288)?,
        release: word(image, 296)?,
    };
    ensure(
        (BASE..end).contains(&table.invoke) && (BASE..end).contains(&table.release),
        "phase entry outside original image",
    )?;
    Ok(table)
}
fn nested_invoke(a: &mut Code, t: &Table, cancel: bool, fatal: usize) {
    a.load(AX, R13, 120);
    a.load(DI, AX, 16);
    a.load(SI, AX, 8);
    a.lea(DX, R13, 160);
    if cancel {
        a.lea(CX, R13, 352);
    } else {
        a.immediate(CX, 0);
    }
    a.checked_call(t.invoke, fatal);
}
fn invalid_call(a: &mut Code, table: &Table, case: u64, payload: usize, bytes: u64, fatal: usize) {
    a.address(DI, payload);
    a.immediate(SI, bytes);
    a.lea(DX, R13, 160);
    a.immediate(CX, 0);
    match case {
        40 => {
            a.immediate(DI, u64::MAX - 3);
            a.immediate(SI, 8);
        }
        41 => a.immediate(DX, 1),
        42 => a.immediate(DX, u64::MAX - 7),
        43 => a.immediate(CX, u64::MAX - 7),
        44 => a.immediate(DI, u64::MAX - 7),
        _ => {}
    }
    a.checked_call(
        if case == 44 {
            table.release
        } else {
            table.invoke
        },
        fatal,
    );
}
pub(super) fn wrap(
    image: &[u8],
    request: &[u8],
    alternate: Option<&[u8]>,
    mode: Mode,
) -> Check<Vec<u8>> {
    let table = table(image)?;
    let offset = image
        .len()
        .checked_add(63)
        .ok_or("caller image alignment")?
        & !63;
    let mut a = Code::default();
    let payload = a.label();
    let invalid = a.label();
    let write = a.label();
    let fatal = a.label();
    let bytes = u64::try_from(request.len()).map_err(|_| "phase request extent")?;
    let alternate_payload = a.label();
    let alternate = alternate.unwrap_or(request);
    let alternate_bytes =
        u64::try_from(alternate.len()).map_err(|_| "alternate phase request extent")?;
    a.group(5, SP, 512);
    a.mov(R13, SP);
    a.mov(DI, SP);
    a.immediate(AX, 0);
    a.immediate(CX, 64);
    a.raw(&[0xfc, 0xf3, 0x48, 0xab]);
    a.frame(100, 0, write, fatal)?;
    if mode == Mode::RuntimeReject {
        a.invoke(&table, 160, payload, bytes, false, fatal);
        a.frame(24, 160, write, fatal)?;
        a.invoke(
            &table,
            160,
            alternate_payload,
            alternate_bytes,
            false,
            fatal,
        );
        a.frame(1, 160, write, fatal)?;
        a.release(&table, 160, fatal);
        a.frame(10, 160, write, fatal)?;
        a.frame(11, 0, write, fatal)?;
    } else if mode == Mode::AllocationFree {
        a.frame(110, 160, write, fatal)?;
        a.invoke(&table, 160, payload, bytes, true, fatal);
        a.frame(7, 160, write, fatal)?;
        a.set(352, 0);
        a.invoke(
            &table,
            160,
            alternate_payload,
            alternate_bytes,
            false,
            fatal,
        );
        a.frame(1, 160, write, fatal)?;
        a.release(&table, 160, fatal);
        a.frame(10, 160, write, fatal)?;
        a.frame(11, 0, write, fatal)?;
    } else {
        a.invoke(&table, 0, payload, bytes, false, fatal);
        a.frame(0, 0, write, fatal)?;
        if mode == Mode::Echo {
            for case in 40..=44 {
                invalid_call(&mut a, &table, case, payload, bytes, fatal);
                a.frame(case, 160, write, fatal)?;
            }
        }
        if mode == Mode::ObserverLeak {
            // Invalid caller forgets ownership without unmapping. The observer must
            // reject zero counters with live OS mappings. Production bytes are intact.
            for offset in (0..128).step_by(8) {
                a.set(offset, 0);
            }
            a.frame(300, 0, write, fatal)?;
        } else {
            if mode == Mode::Echo {
                nested_invoke(&mut a, &table, false, fatal);
            } else {
                a.invoke(
                    &table,
                    160,
                    alternate_payload,
                    alternate_bytes,
                    false,
                    fatal,
                );
            }
            a.frame(1, 160, write, fatal)?;
            a.frame(2, 0, write, fatal)?;
            a.release(&table, 160, fatal);
            if mode == Mode::Echo {
                a.frame(203, 160, write, fatal)?;
                a.release(&table, 160, fatal);
            }
            a.frame(3, 160, write, fatal)?;
            a.set(352, 1);
            if mode == Mode::Echo {
                nested_invoke(&mut a, &table, true, fatal);
            } else {
                a.invoke(&table, 160, payload, bytes, true, fatal);
            }
            a.frame(20, 160, write, fatal)?;
            a.set(352, 0);
            if mode == Mode::Compiler {
                a.invoke(&table, 160, invalid, 7, false, fatal);
                a.frame(4, 160, write, fatal)?;
                a.release(&table, 160, fatal);
                a.frame(5, 160, write, fatal)?;
                a.frame(106, 160, write, fatal)?;
                a.invoke(&table, 160, payload, bytes, false, fatal);
                a.frame(6, 160, write, fatal)?;
                a.frame(107, 160, write, fatal)?;
                a.invoke(&table, 160, payload, bytes, true, fatal);
                a.frame(7, 160, write, fatal)?;
                a.set(352, 0);
                a.frame(108, 160, write, fatal)?;
                a.invoke(&table, 160, payload, bytes, false, fatal);
            } else {
                a.frame(200, 160, write, fatal)?;
                nested_invoke(&mut a, &table, false, fatal);
                a.frame(21, 160, write, fatal)?;
                a.frame(201, 160, write, fatal)?;
                nested_invoke(&mut a, &table, false, fatal);
                a.frame(22, 160, write, fatal)?;
                a.frame(202, 160, write, fatal)?;
                nested_invoke(&mut a, &table, false, fatal);
            }
            a.frame(8, 160, write, fatal)?;
            a.frame(9, 0, write, fatal)?;
            if mode == Mode::Echo {
                a.release(&table, 0, fatal);
                a.frame(30, 0, write, fatal)?;
                a.frame(31, 160, write, fatal)?;
                a.release(&table, 160, fatal);
                a.frame(10, 160, write, fatal)?;
                a.frame(11, 0, write, fatal)?;
            } else {
                a.release(&table, 160, fatal);
                a.frame(10, 160, write, fatal)?;
                a.release(&table, 0, fatal);
                a.frame(11, 0, write, fatal)?;
            }
        }
    }
    a.immediate(AX, 60);
    a.immediate(DI, 0);
    a.syscall();
    a.raw(&[0x0f, 0x0b]);
    a.bind(write)?;
    let again = a.label();
    a.bind(again)?;
    a.immediate(AX, 1);
    a.immediate(DI, 1);
    a.syscall();
    a.group(7, AX, -4);
    a.condition(E, again);
    a.group(7, AX, 0);
    a.condition(LE, fatal);
    a.binary(0x01, SI, AX);
    a.binary(0x29, DX, AX);
    a.condition(NE, again);
    a.raw(&[0xc3]);
    a.bind(fatal)?;
    a.immediate(AX, 60);
    a.immediate(DI, 99);
    a.syscall();
    a.raw(&[0x0f, 0x0b]);
    a.bind(payload)?;
    a.raw(request);
    a.bind(alternate_payload)?;
    a.raw(alternate);
    a.bind(invalid)?;
    a.raw(b"invalid");
    let code = a.finish(offset)?;
    let mut wrapper = image.to_vec();
    wrapper.resize(offset, 0);
    wrapper.extend_from_slice(&code);
    let entry = BASE
        .checked_add(u64::try_from(offset).map_err(|_| "caller offset")?)
        .ok_or("caller entry overflow")?;
    let length = u64::try_from(wrapper.len()).map_err(|_| "caller length")?;
    for (at, value) in [(24, entry), (96, length), (104, length)] {
        wrapper
            .get_mut(at..at + 8)
            .ok_or("caller ELF header")?
            .copy_from_slice(&value.to_le_bytes());
    }
    ensure(
        image
            .iter()
            .zip(&wrapper)
            .enumerate()
            .all(|(at, (a, b))| (24..32).contains(&at) || (96..112).contains(&at) || a == b),
        "caller changed production instruction/data bytes",
    )?;
    Ok(wrapper)
}
