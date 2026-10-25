//! Unkeyed and derive-key 32-byte BLAKE3 emitted as native integer instructions.
//! The left-complete chunk tree has at most 54 live frames for any nonnegative
//! I64 byte length. No host hashing runs in the executable.
//! Algorithm and independent vectors: https://github.com/BLAKE3-team/BLAKE3
use super::asm::{R::*, *};

const IV: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];
const PERMUTE: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
// Each frame holds metadata, CV, padded block, compression state and child CVs.
const CV: i32 = 64;
const BLOCK: i32 = 96;
const STATE: i32 = 160;
const LEFT: i32 = 224;
const RIGHT: i32 = 256;
const FRAME: i32 = 288;

fn initial_cv(a: &mut Asm) {
    let iv = a.label();
    let done = a.label();
    a.load(R9, R12, 48);
    a.bin(0x85, R9, R9);
    a.when(E, iv);
    for i in 0..4 {
        a.load(Ax, R9, i * 8);
        a.store(R12, CV + i * 8, Ax);
    }
    a.jump(done);
    a.bind(iv);
    for (i, word) in IV.into_iter().enumerate() {
        a.imm(Ax, u64::from(word));
        a.store32(R12, CV + i as i32 * 4, Ax);
    }
    a.bind(done);
}

fn child_parameters(a: &mut Asm) {
    a.load(Cx, R12, 24);
    a.group(4, Cx, !8); // Preserve derivation flags while clearing ROOT.
    a.load(R9, R12, 48);
}

pub(super) fn emit(a: &mut Asm, tree: Label) {
    let compress = a.label();
    let leaf = a.label();
    let finish = a.label();
    a.bind(tree);
    a.push(Bx);
    a.push(R12);
    a.push(R13);
    a.group(5, Sp, FRAME);
    a.mov(R12, Sp);
    // Di data, Si length, Dx chunk counter, Cx mode/root flags, R8 output.
    // R9 points to 32 key bytes, or is zero for the IV. It stays live in the caller.
    for (offset, register) in [(0, Di), (8, Si), (16, Dx), (24, Cx), (32, R8), (48, R9)] {
        a.store(R12, offset, register);
    }
    a.group(7, Si, 1025);
    a.when(B, leaf);
    a.imm(Bx, 1024);
    let split = a.label();
    let divided = a.label();
    a.bind(split);
    a.mov(Ax, Bx);
    a.shift(4, Ax, 1);
    a.bin(0x39, Ax, Si);
    a.when(AE, divided);
    a.mov(Bx, Ax);
    a.jump(split);
    a.bind(divided);
    a.store(R12, 40, Bx);
    a.mov(Si, Bx);
    child_parameters(a);
    a.lea(R8, R12, LEFT);
    a.call(tree);
    a.load(Bx, R12, 40);
    a.load(Di, R12, 0);
    a.bin(0x01, Di, Bx);
    a.load(Si, R12, 8);
    a.bin(0x29, Si, Bx);
    a.shift(5, Bx, 10);
    a.load(Dx, R12, 16);
    a.bin(0x01, Dx, Bx);
    child_parameters(a);
    a.lea(R8, R12, RIGHT);
    a.call(tree);
    for i in 0..8 {
        a.load(Ax, R12, LEFT + i * 8);
        a.store(R12, BLOCK + i * 8, Ax);
    }
    initial_cv(a);
    a.imm(Ax, 0);
    a.store(R12, 16, Ax);
    a.imm(Ax, 64);
    a.store(R12, 56, Ax);
    a.load(R8, R12, 24);
    a.group(1, R8, 4); // PARENT | (ROOT for the top node).
    a.call(compress);
    a.jump(finish);

    a.bind(leaf);
    initial_cv(a);
    a.imm(R13, 0); // Number of blocks already compressed in this chunk.
    let block = a.label();
    a.bind(block);
    a.imm(Ax, 0);
    for i in 0..8 {
        a.store(R12, BLOCK + i * 8, Ax);
    }
    a.load(Bx, R12, 8);
    a.group(7, Bx, 65);
    let short = a.label();
    a.when(B, short);
    a.imm(Bx, 64);
    a.bind(short);
    a.store(R12, 56, Bx);
    a.load(Si, R12, 0);
    a.lea(Di, R12, BLOCK);
    a.imm(Cx, 0);
    let copy = a.label();
    let copied = a.label();
    a.bind(copy);
    a.bin(0x39, Cx, Bx);
    a.when(AE, copied);
    a.load_byte(Ax, Si);
    a.store_byte(Di, Ax);
    a.group(0, Si, 1);
    a.group(0, Di, 1);
    a.group(0, Cx, 1);
    a.jump(copy);
    a.bind(copied);
    a.imm(R8, 0);
    a.bin(0x85, R13, R13);
    let flags = a.label();
    a.when(NE, flags);
    a.imm(R8, 1); // CHUNK_START.
    a.bind(flags);
    a.load(Ax, R12, 8);
    a.group(7, Ax, 65);
    let middle = a.label();
    a.when(AE, middle);
    a.group(1, R8, 2); // CHUNK_END, also for the one empty block.
    a.load(Ax, R12, 24);
    a.bin(0x09, R8, Ax);
    a.bind(middle);
    a.load(Ax, R12, 24);
    a.group(4, Ax, !8);
    a.bin(0x09, R8, Ax); // Every chunk block carries the derivation mode.
    a.call(compress);
    a.load(Ax, R12, 8);
    a.group(7, Ax, 65);
    a.when(B, finish);
    a.group(5, Ax, 64);
    a.store(R12, 8, Ax);
    a.load(Ax, R12, 0);
    a.group(0, Ax, 64);
    a.store(R12, 0, Ax);
    a.group(0, R13, 1);
    a.jump(block);

    a.bind(finish);
    a.load(Di, R12, 32);
    for i in 0..4 {
        a.load(Ax, R12, CV + i * 8);
        a.store(Di, i * 8, Ax);
    }
    a.group(0, Sp, FRAME);
    a.pop(R13);
    a.pop(R12);
    a.pop(Bx);
    a.ret();
    emit_compress(a, compress);
}

fn emit_compress(a: &mut Asm, label: Label) {
    a.bind(label);
    for i in 0..8 {
        a.load32(Ax, R12, CV + i * 4);
        a.store32(R12, STATE + i * 4, Ax);
    }
    for (i, word) in IV[..4].iter().enumerate() {
        a.imm(Ax, u64::from(*word));
        a.store32(R12, STATE + 32 + i as i32 * 4, Ax);
    }
    a.load(Ax, R12, 16);
    a.store32(R12, STATE + 48, Ax);
    a.shift(5, Ax, 32);
    a.store32(R12, STATE + 52, Ax);
    a.load(Ax, R12, 56);
    a.store32(R12, STATE + 56, Ax);
    a.store32(R12, STATE + 60, R8);
    let mut words: [usize; 16] = std::array::from_fn(|i| i);
    for _ in 0..7 {
        for (column, [va, vb, vc, vd]) in [
            [0, 4, 8, 12],
            [1, 5, 9, 13],
            [2, 6, 10, 14],
            [3, 7, 11, 15],
            [0, 5, 10, 15],
            [1, 6, 11, 12],
            [2, 7, 8, 13],
            [3, 4, 9, 14],
        ]
        .into_iter()
        .enumerate()
        {
            for (register, index) in [(Ax, va), (Cx, vb), (Dx, vc), (Si, vd)] {
                a.load32(register, R12, STATE + index * 4);
            }
            for (word, rotate_d, rotate_b) in
                [(words[column * 2], 16, 12), (words[column * 2 + 1], 8, 7)]
            {
                a.load32(Di, R12, BLOCK + word as i32 * 4);
                a.bin32(0x01, Ax, Cx);
                a.bin32(0x01, Ax, Di);
                a.bin32(0x31, Si, Ax);
                a.rotate32(Si, rotate_d);
                a.bin32(0x01, Dx, Si);
                a.bin32(0x31, Cx, Dx);
                a.rotate32(Cx, rotate_b);
            }
            for (register, index) in [(Ax, va), (Cx, vb), (Dx, vc), (Si, vd)] {
                a.store32(R12, STATE + index * 4, register);
            }
        }
        words = PERMUTE.map(|index| words[index]);
    }
    for i in 0..8 {
        a.load32(Ax, R12, STATE + i * 4);
        a.load32(Cx, R12, STATE + 32 + i * 4);
        a.bin32(0x31, Ax, Cx);
        a.store32(R12, CV + i * 4, Ax);
    }
    a.ret();
}
