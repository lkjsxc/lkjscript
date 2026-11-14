//! Scalar instruction selection. Overflow and conversion semantics are explicit.
use super::*;

impl Body<'_> {
    pub(super) fn scalar_unary(
        &mut self,
        dst: usize,
        op: UnaryOp,
        src: usize,
    ) -> Result<(), Diagnostic> {
        let register = self.registers[dst].unwrap_or(A);
        self.read(src, register);
        match self.program.types[self.ty(src)].kind {
            TypeKind::F64 => {
                if op != UnaryOp::Neg {
                    return Err(self.problem("unsupported floating unary operation"));
                }
                self.a.immediate(C, 1u64 << 63);
                self.a.rr(0x31, register, C, true);
            }
            TypeKind::Bool => {
                self.a.immediate(C, 1);
                self.a.rr(0x31, register, C, true);
            }
            TypeKind::Int { bits, signed } => {
                if op == UnaryOp::Not {
                    self.a.not(register);
                } else {
                    let minimum = if signed { 1u64 << (bits - 1) } else { 0 };
                    if signed {
                        let minimum = if bits == 64 {
                            minimum
                        } else {
                            0u64.wrapping_sub(minimum)
                        };
                        self.a.immediate(C, minimum);
                        self.a.compare(register, C);
                        self.trap_if(4, 1);
                    } else {
                        self.a.test(register, true);
                        self.trap_if(5, 1);
                    }
                    self.a.negate(register);
                }
            }
            _ => return Err(self.problem("unsupported scalar unary type")),
        }
        self.write(dst, register);
        Ok(())
    }

    fn scalar_narrow(&mut self, register: u8, bits: u8, signed: bool) {
        if bits == 64 {
            return;
        }
        if signed {
            self.a.immediate(D, 0u64.wrapping_sub(1u64 << (bits - 1)));
            self.a.compare(register, D);
            self.trap_if(12, 1);
            self.a.immediate(D, (1u64 << (bits - 1)) - 1);
            self.a.compare(register, D);
            self.trap_if(15, 1);
        } else {
            self.a.immediate(D, (1u64 << bits) - 1);
            self.a.compare(register, D);
            self.trap_if(7, 1);
        }
    }

    fn floating_binary(
        &mut self,
        dst: usize,
        op: BinaryOp,
        left: usize,
        right: usize,
    ) -> Result<(), Diagnostic> {
        self.read(left, A);
        self.read(right, C);
        self.a.mov_to_xmm(0, A);
        self.a.mov_to_xmm(1, C);
        let arithmetic = match op {
            BinaryOp::Add => Some(0x58),
            BinaryOp::Sub => Some(0x5c),
            BinaryOp::Mul => Some(0x59),
            BinaryOp::Div => Some(0x5e),
            _ => None,
        };
        match arithmetic {
            Some(opcode) => {
                self.a.f64_op(opcode, 0, 1);
                self.a.mov_from_xmm(A, 0);
                self.a.f64_compare(0, 0);
                let ordinary = self.a.label();
                self.a.condition(11, ordinary);
                self.a.immediate(A, 0x7ff8000000000000);
                self.a.bind(ordinary);
            }
            None => {
                let condition = match op {
                    BinaryOp::Eq => 4,
                    BinaryOp::Ne => 5,
                    BinaryOp::Lt => 2,
                    BinaryOp::Gt => 7,
                    BinaryOp::Le => 6,
                    BinaryOp::Ge => 3,
                    _ => return Err(self.problem("unsupported floating binary operation")),
                };
                self.a.f64_compare(0, 1);
                self.a.setcc(A, condition);
                self.a.setcc(C, if op == BinaryOp::Ne { 10 } else { 11 });
                self.a
                    .rr(if op == BinaryOp::Ne { 0x09 } else { 0x21 }, A, C, true);
            }
        }
        self.write(dst, A);
        Ok(())
    }

    pub(super) fn scalar_binary(
        &mut self,
        dst: usize,
        op: BinaryOp,
        left: usize,
        right: usize,
    ) -> Result<(), Diagnostic> {
        if self.program.types[self.ty(left)].kind == TypeKind::F64 {
            return self.floating_binary(dst, op, left, right);
        }
        let (bits, signed) = match self.program.types[self.ty(left)].kind {
            TypeKind::Int { bits, signed } => (bits, signed),
            TypeKind::Bool => (1, false),
            TypeKind::Char => (32, false),
            _ => return Err(self.problem("unsupported scalar binary type")),
        };
        let assigned = self.registers[dst].unwrap_or(A);
        let fixed = matches!(op, BinaryOp::Div | BinaryOp::Rem)
            || (!signed && bits == 64 && op == BinaryOp::Mul);
        let register = if fixed || self.registers[right] == Some(assigned) {
            A
        } else {
            assigned
        };
        self.read(left, register);
        self.read(right, C);
        match op {
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Gt
            | BinaryOp::Le
            | BinaryOp::Ge => {
                let condition = match op {
                    BinaryOp::Eq => 4,
                    BinaryOp::Ne => 5,
                    BinaryOp::Lt => {
                        if signed {
                            12
                        } else {
                            2
                        }
                    }
                    BinaryOp::Gt => {
                        if signed {
                            15
                        } else {
                            7
                        }
                    }
                    BinaryOp::Le => {
                        if signed {
                            14
                        } else {
                            6
                        }
                    }
                    _ => {
                        if signed {
                            13
                        } else {
                            3
                        }
                    }
                };
                self.a.compare(register, C);
                self.a.setcc(register, condition);
            }
            BinaryOp::Add | BinaryOp::WrappingAdd => {
                self.a.rr(0x01, register, C, true);
                if op == BinaryOp::Add {
                    if bits == 64 {
                        self.trap_if(if signed { 0 } else { 2 }, 1);
                    }
                    self.scalar_narrow(register, bits, signed);
                }
            }
            BinaryOp::Sub | BinaryOp::WrappingSub => {
                self.a.rr(0x29, register, C, true);
                if op == BinaryOp::Sub {
                    if bits == 64 {
                        self.trap_if(if signed { 0 } else { 2 }, 1);
                    }
                    self.scalar_narrow(register, bits, signed);
                }
            }
            BinaryOp::Mul | BinaryOp::WrappingMul => {
                if !signed && bits == 64 && op == BinaryOp::Mul {
                    self.a.mul(C);
                    self.a.test(D, true);
                    self.trap_if(5, 1);
                } else {
                    self.a.imul(register, C);
                    if op == BinaryOp::Mul {
                        if bits == 64 {
                            self.trap_if(0, 1);
                        }
                        self.scalar_narrow(register, bits, signed);
                    }
                }
            }
            BinaryOp::Div | BinaryOp::Rem => {
                self.a.test(C, true);
                self.trap_if(4, 2);
                if signed {
                    let ordinary = self.a.label();
                    self.a.immediate(D, 0u64.wrapping_sub(1u64 << (bits - 1)));
                    self.a.compare(register, D);
                    self.a.condition(5, ordinary);
                    self.a.immediate(D, !0u64);
                    self.a.compare(C, D);
                    self.trap_if(4, 1);
                    self.a.bind(ordinary);
                    self.a.raw(&[0x48, 0x99]);
                } else {
                    self.a.rr(0x31, D, D, true);
                }
                self.a.divide(C, signed);
                if op == BinaryOp::Rem {
                    self.a.mov(A, D);
                }
            }
            BinaryOp::BitAnd | BinaryOp::And => self.a.rr(0x21, register, C, true),
            BinaryOp::BitOr | BinaryOp::Or => self.a.rr(0x09, register, C, true),
            BinaryOp::BitXor => self.a.rr(0x31, register, C, true),
            BinaryOp::Shl | BinaryOp::Shr => {
                self.a.immediate(D, bits as u64);
                self.a.compare(C, D);
                self.trap_if(3, 1);
                self.a.shift_cl(
                    register,
                    if op == BinaryOp::Shl {
                        4
                    } else if signed {
                        7
                    } else {
                        5
                    },
                );
            }
        }
        self.write(dst, register);
        Ok(())
    }

    fn floating_to_integer(&mut self, bits: u8, signed: bool) {
        let zero = self.a.label();
        let low = self.a.label();
        let high = self.a.label();
        let done = self.a.label();
        self.a.mov_to_xmm(0, A);
        self.a.f64_compare(0, 0);
        self.a.condition(10, zero);
        let exponent = if signed { bits - 1 } else { bits };
        let bound = (1023u64 + exponent as u64) << 52;
        self.a
            .immediate(C, if signed { bound | (1u64 << 63) } else { 0 });
        self.a.mov_to_xmm(1, C);
        self.a.f64_compare(0, 1);
        self.a.condition(6, low);
        self.a.immediate(C, bound);
        self.a.mov_to_xmm(1, C);
        self.a.f64_compare(0, 1);
        self.a.condition(3, high);
        if !signed && bits == 64 {
            let ordinary = self.a.label();
            self.a.immediate(C, (1023u64 + 63) << 52);
            self.a.mov_to_xmm(1, C);
            self.a.f64_compare(0, 1);
            self.a.condition(2, ordinary);
            self.a.f64_op(0x5c, 0, 1);
            self.a.f64_to_int(A, 0);
            self.a.immediate(C, 1u64 << 63);
            self.a.rr(0x31, A, C, true);
            self.a.jump(done);
            self.a.bind(ordinary);
        }
        self.a.f64_to_int(A, 0);
        self.a.jump(done);
        self.a.bind(low);
        self.a.immediate(
            A,
            if signed {
                0u64.wrapping_sub(1u64 << (bits - 1))
            } else {
                0
            },
        );
        self.a.jump(done);
        self.a.bind(high);
        let maximum = if bits == 64 && !signed {
            !0u64
        } else {
            (1u64 << exponent) - 1
        };
        self.a.immediate(A, maximum);
        self.a.jump(done);
        self.a.bind(zero);
        self.a.immediate(A, 0);
        self.a.bind(done);
    }

    pub(super) fn scalar_cast(&mut self, dst: usize, src: usize) -> Result<(), Diagnostic> {
        self.read(src, A);
        let from = self.program.types[self.ty(src)].kind.clone();
        let to = self.program.types[self.ty(dst)].kind.clone();
        match (from, to) {
            (TypeKind::F64, TypeKind::Int { bits, signed }) => {
                self.floating_to_integer(bits, signed)
            }
            (TypeKind::Int { signed, .. }, TypeKind::F64) => {
                if signed {
                    self.a.int_to_f64(0, A);
                } else {
                    let ordinary = self.a.label();
                    let done = self.a.label();
                    self.a.test(A, true);
                    self.a.condition(9, ordinary);
                    self.a.mov(C, A);
                    self.a.immediate(D, 1);
                    self.a.rr(0x21, C, D, true);
                    self.a.shift(A, 5, 1);
                    self.a.rr(0x09, A, C, true);
                    self.a.int_to_f64(0, A);
                    self.a.f64_op(0x58, 0, 0);
                    self.a.jump(done);
                    self.a.bind(ordinary);
                    self.a.int_to_f64(0, A);
                    self.a.bind(done);
                }
                self.a.mov_from_xmm(A, 0);
            }
            _ => {}
        }
        self.write(dst, A);
        Ok(())
    }
}
