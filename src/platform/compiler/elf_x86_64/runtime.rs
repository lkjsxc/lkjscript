//! Standalone Linux routines emitted directly into the image.
use super::asm::{Asm, Label, R::*, *};

pub(super) struct Runtime {
    pub alloc: Label,
    pub integer: Label,
    pub boolean: Label,
    pub integer_text: Label,
    pub print: Label,
    pub write: Label,
    pub parse: Label,
    pub overflow: Label,
    pub division: Label,
    pub bounds: Label,
    pub bytes_index: Label,
    pub bytes_octet: Label,
    pub bytes_range: Label,
    pub bytes_capacity: Label,
    pub buffer_index: Label,
    pub buffer_octet: Label,
    pub buffer_capacity: Label,
    pub blake3: Label,
    pub input: Label,
    pub capacity: Label,
    pub argument: Label,
}

pub(super) fn data(a: &mut Asm, pool: &mut Vec<(Label, Vec<u8>)>, bytes: &[u8]) -> Label {
    let label = a.label();
    pool.push((label, bytes.to_vec()));
    label
}

fn save(a: &mut Asm) {
    a.push(Bx);
    a.push(R12);
    a.push(R13);
}
fn restore(a: &mut Asm) {
    a.pop(R13);
    a.pop(R12);
    a.pop(Bx);
    a.ret();
}
fn literal(a: &mut Asm, label: Label, length: u64, write: Label) {
    a.imm(Di, 1);
    a.address(Si, label);
    a.imm(Dx, length);
    a.call(write);
}

impl Runtime {
    pub fn new(a: &mut Asm) -> Self {
        Self {
            alloc: a.label(),
            integer: a.label(),
            boolean: a.label(),
            integer_text: a.label(),
            print: a.label(),
            write: a.label(),
            parse: a.label(),
            overflow: a.label(),
            division: a.label(),
            bounds: a.label(),
            bytes_index: a.label(),
            bytes_octet: a.label(),
            bytes_range: a.label(),
            bytes_capacity: a.label(),
            buffer_index: a.label(),
            buffer_octet: a.label(),
            buffer_capacity: a.label(),
            blake3: a.label(),
            input: a.label(),
            capacity: a.label(),
            argument: a.label(),
        }
    }

    pub fn emit(&self, a: &mut Asm, pool: &mut Vec<(Label, Vec<u8>)>) {
        // Bump allocation uses anonymous read/write mappings. Executable image pages
        // remain read/execute. Every old mapping remains live until sys_exit.
        a.bind(self.alloc);
        a.group(0, Di, 7);
        a.when(B, self.capacity);
        a.group(4, Di, -8);
        a.mov(Ax, R15);
        a.bin(0x01, Ax, Di);
        a.when(B, self.capacity);
        a.bin(0x39, Ax, R14);
        let available = a.label();
        a.when(B, available);
        a.push(Di);
        a.mov(Si, Di);
        a.group(7, Si, 65536);
        let size = a.label();
        a.when(AE, size);
        a.imm(Si, 65536);
        a.bind(size);
        a.imm(Ax, 9);
        a.imm(Di, 0);
        a.imm(Dx, 3);
        a.imm(R10, 0x22);
        a.imm(R8, u64::MAX);
        a.imm(R9, 0);
        a.syscall();
        a.group(7, Ax, -4095);
        a.when(AE, self.capacity);
        a.mov(R15, Ax);
        a.mov(R14, Ax);
        a.bin(0x01, R14, Si);
        a.when(B, self.capacity);
        a.pop(Di);
        a.bind(available);
        a.mov(Ax, R15);
        a.bin(0x01, R15, Di);
        a.ret();

        for (label, tag) in [(self.integer, 2), (self.boolean, 1)] {
            a.bind(label);
            a.push(Ax);
            a.imm(Di, 16);
            a.call(self.alloc);
            a.pop(Cx);
            a.store(Ax, 8, Cx);
            a.imm(Cx, tag);
            a.store(Ax, 0, Cx);
            a.ret();
        }

        a.bind(self.write);
        let again = a.label();
        let done = a.label();
        let io_error = a.label();
        a.bind(again);
        a.bin(0x85, Dx, Dx);
        a.when(E, done);
        a.imm(Ax, 1);
        a.syscall();
        a.group(7, Ax, -4);
        a.when(E, again);
        a.group(7, Ax, 0);
        a.when(LE, io_error);
        a.bin(0x01, Si, Ax);
        a.bin(0x29, Dx, Ax);
        a.jump(again);
        a.bind(done);
        a.ret();
        a.bind(io_error);
        a.imm(Di, 74);
        a.imm(Ax, 60);
        a.syscall();
        a.bytes(&[0x0f, 0x0b]);

        // Negative accumulation admits i64::MIN without a positive intermediate.
        a.bind(self.parse);
        save(a);
        a.mov(Bx, Di);
        a.imm(R13, 0);
        a.imm(Ax, 0);
        a.load_byte(Dx, Bx);
        a.group(7, Dx, 45);
        let plus = a.label();
        let digits = a.label();
        a.when(NE, plus);
        a.imm(R13, 1);
        a.group(0, Bx, 1);
        a.jump(digits);
        a.bind(plus);
        a.group(7, Dx, 43);
        a.when(NE, digits);
        a.group(0, Bx, 1);
        a.bind(digits);
        a.imm(R12, 0);
        let next = a.label();
        let parsed = a.label();
        a.bind(next);
        a.load_byte(Dx, Bx);
        a.bin(0x85, Dx, Dx);
        a.when(E, parsed);
        a.group(5, Dx, 48);
        a.group(7, Dx, 10);
        a.when(AE, self.argument);
        a.imm(Cx, 10);
        a.imul(Ax, Cx);
        a.when(O, self.argument);
        a.bin(0x29, Ax, Dx);
        a.when(O, self.argument);
        a.group(0, Bx, 1);
        a.group(0, R12, 1);
        a.jump(next);
        a.bind(parsed);
        a.bin(0x85, R12, R12);
        a.when(E, self.argument);
        a.bin(0x85, R13, R13);
        let signed = a.label();
        a.when(NE, signed);
        a.unary(3, Ax);
        a.when(O, self.argument);
        a.bind(signed);
        a.call(self.integer);
        restore(a);

        // Produce a JSON string object for decimal conversion. Printing an integer
        // uses the same digits excluding the two quotes.
        a.bind(self.integer_text);
        save(a);
        a.push(Di);
        a.imm(Di, 64);
        a.call(self.alloc);
        a.mov(R12, Ax);
        a.pop(Ax);
        a.imm(R13, 0);
        a.group(7, Ax, 0);
        let positive = a.label();
        a.when(GE, positive);
        a.imm(R13, 1);
        a.unary(3, Ax);
        a.bind(positive);
        a.lea(Bx, R12, 63);
        a.imm(Dx, 34);
        a.store_byte(Bx, Dx);
        a.group(5, Bx, 1);
        let digit = a.label();
        a.bind(digit);
        a.imm(Dx, 0);
        a.imm(Cx, 10);
        a.unary(6, Cx);
        a.group(0, Dx, 48);
        a.store_byte(Bx, Dx);
        a.group(5, Bx, 1);
        a.bin(0x85, Ax, Ax);
        a.when(NE, digit);
        a.bin(0x85, R13, R13);
        let quote = a.label();
        a.when(E, quote);
        a.imm(Dx, 45);
        a.store_byte(Bx, Dx);
        a.group(5, Bx, 1);
        a.bind(quote);
        a.imm(Dx, 34);
        a.store_byte(Bx, Dx);
        a.lea(Dx, R12, 64);
        a.bin(0x29, Dx, Bx);
        a.imm(Cx, 3);
        a.store(R12, 0, Cx);
        a.store(R12, 8, Dx);
        a.store(R12, 16, Bx);
        // The JSON text bytes and raw UTF-8 view have distinct explicit lengths.
        // Even i64::MIN leaves the first 40 descriptor bytes clear of the digits.
        a.group(5, Dx, 2);
        a.store(R12, 24, Dx);
        a.lea(Dx, Bx, 1);
        a.store(R12, 32, Dx);
        a.mov(Ax, R12);
        restore(a);

        let null = data(a, pool, b"null");
        let yes = data(a, pool, b"true");
        let no = data(a, pool, b"false");
        let comma = data(a, pool, b",");
        let colon = data(a, pool, b":");
        let open_list = data(a, pool, b"[");
        let close_list = data(a, pool, b"]");
        let open_record = data(a, pool, b"{");
        let close_record = data(a, pool, b"}");
        a.bind(self.print);
        save(a);
        a.mov(Bx, Di);
        a.load(Ax, Bx, 0);
        let cases: Vec<_> = (0..6).map(|_| a.label()).collect();
        let end = a.label();
        for (tag, label) in cases.iter().enumerate() {
            a.group(7, Ax, tag as i32);
            a.when(E, *label);
        }
        a.jump(self.bounds);
        a.bind(cases[0]);
        literal(a, null, 4, self.write);
        a.jump(end);
        a.bind(cases[1]);
        a.load(Ax, Bx, 8);
        a.bin(0x85, Ax, Ax);
        let false_value = a.label();
        a.when(E, false_value);
        literal(a, yes, 4, self.write);
        a.jump(end);
        a.bind(false_value);
        literal(a, no, 5, self.write);
        a.jump(end);
        a.bind(cases[2]);
        a.load(Di, Bx, 8);
        a.call(self.integer_text);
        a.load(Si, Ax, 16);
        a.group(0, Si, 1);
        a.load(Dx, Ax, 8);
        a.group(5, Dx, 2);
        a.imm(Di, 1);
        a.call(self.write);
        a.jump(end);
        a.bind(cases[3]);
        a.load(Si, Bx, 16);
        a.load(Dx, Bx, 8);
        a.imm(Di, 1);
        a.call(self.write);
        a.jump(end);
        for (record, case, open, close) in [
            (false, cases[4], open_list, close_list),
            (true, cases[5], open_record, close_record),
        ] {
            a.bind(case);
            literal(a, open, 1, self.write);
            a.imm(R12, 0);
            a.load(R13, Bx, 8);
            let repeat = a.label();
            let finish = a.label();
            let element = a.label();
            a.bind(repeat);
            a.bin(0x39, R12, R13);
            a.when(AE, finish);
            a.bin(0x85, R12, R12);
            a.when(E, element);
            literal(a, comma, 1, self.write);
            a.bind(element);
            a.mov(Ax, R12);
            a.shift(4, Ax, if record { 4 } else { 3 });
            a.bin(0x01, Ax, Bx);
            a.load(Di, Ax, 16);
            a.call(self.print);
            if record {
                literal(a, colon, 1, self.write);
                a.mov(Ax, R12);
                a.shift(4, Ax, 4);
                a.bin(0x01, Ax, Bx);
                a.load(Di, Ax, 24);
                a.call(self.print);
            }
            a.group(0, R12, 1);
            a.jump(repeat);
            a.bind(finish);
            literal(a, close, 1, self.write);
            a.jump(end);
        }
        a.bind(end);
        restore(a);

        for (label, code, status) in [
            (self.overflow, "normalized_integer_overflow", 70),
            (self.division, "normalized_integer_division", 70),
            (self.bounds, "normalized_list_index", 70),
            (self.bytes_index, "normalized_bytes_index", 70),
            (self.bytes_octet, "normalized_bytes_octet", 70),
            (self.bytes_range, "normalized_bytes_range", 70),
            (self.bytes_capacity, "normalized_allocation", 70),
            (self.buffer_index, "normalized_buffer_index", 70),
            (self.buffer_octet, "normalized_buffer_octet", 70),
            (self.buffer_capacity, "normalized_buffer_storage", 70),
            (self.capacity, "native_allocation", 70),
            (self.argument, "native_argument", 64),
            (self.input, "native_input", 65),
        ] {
            let message = format!("{code}\n");
            let message_label = data(a, pool, message.as_bytes());
            a.bind(label);
            a.imm(Di, 2);
            a.address(Si, message_label);
            a.imm(Dx, message.len() as u64);
            a.call(self.write);
            a.imm(Di, status);
            a.imm(Ax, 60);
            a.syscall();
            a.bytes(&[0x0f, 0x0b]);
        }
        super::blake3::emit(a, self.blake3);
    }
}
