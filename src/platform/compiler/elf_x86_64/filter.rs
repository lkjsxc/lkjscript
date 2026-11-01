//! Explicit typed command boundary for binary compiler targets. A selected
//! (Bytes) -> {diagnostic: I64, image: Bytes} entry is a stdin/stdout filter.
use super::asm::{R::*, *};
use super::{Emitter, NormalizedProgram, Result, TypeForm};

pub(super) fn signature(program: &NormalizedProgram, entry: usize) -> bool {
    let function = &program.functions[entry];
    let [parameter] = function.parameters.as_ref() else {
        return false;
    };
    if !matches!(
        program.types.get(&parameter.ty).map(|t| &t.form),
        Some(TypeForm::Bytes)
    ) {
        return false;
    }
    let Some(TypeForm::StructuralRecord { fields }) =
        program.types.get(&function.result.ty).map(|t| &t.form)
    else {
        return false;
    };
    fields.len() == 2
        && fields.iter().all(|field| {
            matches!(
                (
                    field.name.as_str(),
                    program.types.get(&field.ty).map(|t| &t.form)
                ),
                ("diagnostic", Some(TypeForm::I64)) | ("image", Some(TypeForm::Bytes))
            )
        })
}

impl Emitter<'_> {
    pub(super) fn start_filter(&mut self, entry: usize) -> Result<()> {
        self.a.imm(R15, 0);
        self.a.load(Ax, Sp, 0);
        self.a.group(7, Ax, 1);
        self.a.when(NE, self.runtime.argument);
        super::phase::process_context(&mut self.a);
        // Input storage is private to this reader until EOF. Growth may move its
        // mapping because no Bytes descriptor or view exists yet. The mapping
        // then stays live through the result write, which may alias the input.
        self.a.imm(Bx, 65536);
        self.a.imm(Ax, 9);
        self.a.imm(Di, 0);
        self.a.mov(Si, Bx);
        self.a.imm(Dx, 3);
        self.a.imm(R10, 0x22);
        self.a.imm(R8, u64::MAX);
        self.a.imm(R9, 0);
        self.a.syscall();
        self.a.group(7, Ax, -4095);
        self.a.when(AE, self.runtime.capacity);
        self.a.mov(R12, Ax);
        self.a.imm(R13, 0);
        let read = self.a.label();
        let probe = self.a.label();
        let probed = self.a.label();
        let available = self.a.label();
        let eof = self.a.label();
        self.a.bind(read);
        self.a.bin(0x39, R13, Bx);
        self.a.when(B, available);
        // Probe EOF before growing a full mapping. The scratch byte cannot alias
        // input storage and R9 survives the remap syscall.
        self.a.group(5, Sp, 8);
        self.a.bind(probe);
        self.a.imm(Di, 0);
        self.a.mov(Si, Sp);
        self.a.imm(Dx, 1);
        self.a.imm(Ax, 0);
        self.a.syscall();
        self.a.group(7, Ax, -4);
        self.a.when(E, probe);
        self.a.group(7, Ax, 0);
        self.a.when(LE, probed);
        self.a.load_byte(R9, Sp);
        self.a.bind(probed);
        // MOVZX and LEA preserve the read-result comparison flags.
        self.a.lea(Sp, Sp, 8);
        self.a.when(L, self.runtime.input);
        self.a.when(E, eof);
        // Capacity stays positive and length stays nonnegative within I64.
        // Representation arithmetic is independent of caller resource policy.
        self.a.mov(Dx, Bx);
        self.a.bin(0x01, Dx, Bx);
        self.a.when(O, self.runtime.capacity);
        self.a.imm(Ax, 25);
        self.a.mov(Di, R12);
        self.a.mov(Si, Bx);
        self.a.imm(R10, 1);
        self.a.syscall();
        self.a.group(7, Ax, -4095);
        self.a.when(AE, self.runtime.capacity);
        self.a.mov(R12, Ax);
        self.a.mov(Bx, Dx);
        self.a.mov(Si, R12);
        self.a.bin(0x01, Si, R13);
        self.a.store_byte(Si, R9);
        self.a.group(0, R13, 1);
        self.a.bind(available);
        self.a.imm(Di, 0);
        self.a.mov(Si, R12);
        self.a.bin(0x01, Si, R13);
        self.a.mov(Dx, Bx);
        self.a.bin(0x29, Dx, R13);
        self.a.imm(Ax, 0);
        self.a.syscall();
        self.a.group(7, Ax, -4);
        self.a.when(E, read);
        self.a.group(7, Ax, 0);
        self.a.when(L, self.runtime.input);
        self.a.when(E, eof);
        // read returned at most capacity - length bytes, so this cannot overflow.
        self.a.bin(0x01, R13, Ax);
        self.a.jump(read);
        self.a.bind(eof);
        self.a.imm(Di, 24);
        self.a.call(self.runtime.alloc);
        self.a.imm(Cx, 7);
        self.a.store(Ax, 0, Cx);
        self.a.store(Ax, 8, R13);
        self.a.store(Ax, 16, R12);
        self.a.push(Ax);
        self.a.call(self.label(entry)?);
        self.a.group(0, Sp, 8);
        // Structural record fields are emitted in canonical name order.
        self.a.load(Cx, Ax, 24);
        self.a.load(Di, Cx, 8);
        self.a.bin(0x85, Di, Di);
        let success = self.a.label();
        self.a.when(E, success);
        self.a.call(self.runtime.integer_text);
        self.a.mov(R13, Ax);
        let prefix = b"compiler_rejected:";
        let prefix_label = super::runtime::data(&mut self.a, &mut self.data, prefix);
        self.a.imm(Di, 2);
        self.a.address(Si, prefix_label);
        self.a.imm(Dx, prefix.len() as u64);
        self.a.call(self.runtime.write);
        self.a.imm(Di, 2);
        self.a.load(Si, R13, 32);
        self.a.load(Dx, R13, 24);
        self.a.call(self.runtime.write);
        let newline = super::runtime::data(&mut self.a, &mut self.data, b"\n");
        self.a.imm(Di, 2);
        self.a.address(Si, newline);
        self.a.imm(Dx, 1);
        self.a.call(self.runtime.write);
        self.filter_exit(65);
        self.a.bind(success);
        self.a.load(Ax, Ax, 40);
        self.a.load(Si, Ax, 16);
        self.a.load(Dx, Ax, 8);
        self.a.imm(Di, 1);
        self.a.call(self.runtime.write);
        self.filter_exit(0);
        Ok(())
    }

    fn filter_exit(&mut self, status: u64) {
        self.a.imm(Di, status);
        self.a.imm(Ax, 60);
        self.a.syscall();
        self.a.bytes(&[0x0f, 0x0b]);
    }
}
