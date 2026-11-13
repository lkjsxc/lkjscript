//! Lower checked authority operations to the narrow Linux host interface.
use super::*;

pub(super) fn host_for(name: &str) -> Option<HostFunction> {
    Some(match name {
        "stdout_write" => HostFunction::StdoutWrite,
        "stderr_write" => HostFunction::StderrWrite,
        "world_arg_count" => HostFunction::ArgumentCount,
        "world_arg_len" => HostFunction::ArgumentLength,
        "world_arg_copy" => HostFunction::ArgumentCopy,
        "world_exit_status" => HostFunction::SetExitStatus,
        "world_directory" => HostFunction::DirectoryOpen,
        "directory_open" => HostFunction::FileOpen,
        "stdin_read" => HostFunction::StdinRead,
        "file_read" => HostFunction::FileRead,
        "file_write" => HostFunction::FileWrite,
        _ => return None,
    })
}
pub(super) fn import(host: HostFunction) -> Import {
    let parameters = match host {
        HostFunction::ArgumentCount => 0,
        HostFunction::ArgumentLength | HostFunction::SetExitStatus | HostFunction::HandleClose => 1,
        HostFunction::StdinRead | HostFunction::DirectoryOpen => 2,
        HostFunction::StdoutWrite
        | HostFunction::StderrWrite
        | HostFunction::ArgumentCopy
        | HostFunction::FileRead
        | HostFunction::FileWrite => 3,
        HostFunction::FileOpen => 4,
    };
    Import {
        host,
        signature: Signature::words(parameters),
    }
}
impl Body<'_> {
    fn host_call(&mut self, host: HostFunction) -> Result<(), String> {
        for index in 0..self.hosts.len() {
            if self.hosts[index] == host {
                call(&mut self.code, index);
                return Ok(());
            }
        }
        Err("lowering omitted a required host operation".to_string())
    }
    fn open_result(&mut self, dst: usize) {
        let ty = self.function.locals[dst].ty;
        // Syscalls return either a nonnegative handle or a negative errno.
        set(&mut self.code, self.scratch);
        get(&mut self.code, self.scratch);
        constant(&mut self.code, 0);
        self.code.push(Instruction::Scalar(ScalarOp::I64LtS));
        begin_if(&mut self.code);
        let mut remaining = 2usize;
        while remaining != 0 {
            remaining = remaining - 1usize;
            let tag = remaining;
            let payload = self.layout.payload(ty, Some(tag));
            constant(&mut self.code, payload.bytes as i64);
            call(&mut self.code, ALLOC + self.runtime);
            set(&mut self.code, self.scratch + 1);
            get(&mut self.code, self.scratch + 1);
            constant(&mut self.code, tag as i64);
            store(&mut self.code, 0);
            get(&mut self.code, self.scratch + 1);
            if tag == 1 {
                constant(&mut self.code, 0);
                get(&mut self.code, self.scratch);
                self.code.push(Instruction::Scalar(ScalarOp::I64Sub));
            } else {
                get(&mut self.code, self.scratch);
            }
            store(&mut self.code, payload.fields[0]);
            if tag == 1 {
                self.code.push(Instruction::Else);
            }
        }
        self.code.push(Instruction::End);
        get(&mut self.code, self.scratch + 1);
        self.write(dst);
    }
    pub(super) fn io_intrinsic(
        &mut self,
        dst: usize,
        name: &str,
        args: &[usize],
        line: usize,
    ) -> Result<bool, String> {
        if name == "world_stdout" || name == "world_stdin" || name == "world_stderr" {
            // Authority is checked in K. No scalar construction can produce it.
            constant(&mut self.code, 1);
            self.write(dst);
            return Ok(true);
        }
        let host = match host_for(name) {
            Some(host) => host,
            None => return Ok(false),
        };
        match name {
            "stdout_write" | "stderr_write" => {
                self.read(args[1]);
                constant(&mut self.code, line as i64);
            }
            "world_arg_count" => {}
            "world_arg_len" | "world_exit_status" => self.read(args[1]),
            "world_arg_copy" => {
                self.read(args[1]);
                self.read(args[2]);
            }
            "world_directory" | "stdin_read" => self.read(args[1]),
            "directory_open" | "file_read" | "file_write" => {
                self.read(args[0]);
                load(&mut self.code, 0);
                self.read(args[1]);
                if name == "directory_open" {
                    self.read(args[2]);
                }
            }
            _ => unreachable!(),
        }
        self.host_call(host)?;
        if name == "world_directory" || name == "directory_open" {
            self.open_result(dst);
        } else {
            if name == "world_arg_len" || name == "world_arg_copy" {
                tee(&mut self.code, self.scratch);
                constant(&mut self.code, 0);
                self.code.push(Instruction::Scalar(ScalarOp::I64LtS));
                trap_if(&mut self.code, 3, line);
                get(&mut self.code, self.scratch);
            }
            self.write(dst);
        }
        Ok(true)
    }
}
