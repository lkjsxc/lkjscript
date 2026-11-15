//! Native startup for explicitly granted owned entry capabilities.
use super::platform::Platform;
use crate::diagnostic::Span;
use crate::ir::EntryGrant;
use crate::x86::*;

pub struct Plan {
    values: Label,
    paths: Vec<Label>,
    grants: Vec<EntryGrant>,
    site: Span,
    failure_buffer: Label,
}
impl Plan {
    pub(super) fn fallible(grant: &EntryGrant) -> bool {
        !matches!(grant, EntryGrant::World | EntryGrant::Stdout | EntryGrant::Stdin
            | EntryGrant::Stderr | EntryGrant::Clock | EntryGrant::Process | EntryGrant::Entropy)
    }
    pub fn new(a: &mut Assembler, grants: &[EntryGrant], site: Span) -> Self {
        let values = a.label();
        let mut paths = Vec::new();
        for _ in grants { paths.push(a.label()); }
        Self { values, paths, grants: grants.to_vec(), site, failure_buffer: a.label() }
    }
    fn offset(&self, index: usize) -> i64 {
        let mut offset = 0i64;
        for at in 0..index { offset += if matches!(self.grants[at], EntryGrant::Stop) { 16i64 } else { 8i64 }; }
        offset
    }
    pub fn prepare(&self, a: &mut Assembler, p: Platform) {
        for (index, grant) in self.grants.iter().enumerate() {
            match grant {
                EntryGrant::Directory(path) => {
                    a.address(DI, self.paths[index]);
                    a.immediate(SI, path.len() as u64);
                    a.call(p.directory_open);
                }
                EntryGrant::Connector(address) => {
                    a.address(DI, self.paths[index]);
                    a.immediate(SI, address.len() as u64);
                    a.call(p.world_connector);
                }
                EntryGrant::Listener(address) => {
                    a.address(DI, self.paths[index]);
                    a.immediate(SI, address.len() as u64);
                    a.call(p.world_listen);
                }
                EntryGrant::Readiness => { a.call(p.world_readiness); },
                EntryGrant::Stop => { a.call(p.world_stop); },
                _ => { a.immediate(A, 1); },
            }
            let ready = a.label();
            a.test(A, true);
            a.condition(9, ready);
            if Self::fallible(grant) {
                // Preserve the host errno and name the failed grant before the
                // generic host trap; a bind failure is not a stdout failure.
                let name = match grant {
                    EntryGrant::Directory(_) => "Directory",
                    EntryGrant::Listener(_) => "Listener",
                    EntryGrant::Connector(_) => "Connector",
                    EntryGrant::Readiness => "Readiness",
                    EntryGrant::Stop => "Stop",
                    _ => "capability",
                };
                a.push(A);
                a.address(R8, self.failure_buffer);
                a.lea(R8, R8, 128);
                super::platform::diagnostic_text(a, "\n");
                a.pop(A);
                a.negate(A);
                super::platform::diagnostic_decimal(a, 1);
                super::platform::diagnostic_text(a, &format!("entry grant {name} failed: errno "));
                a.mov(SI, R8);
                a.address(D, self.failure_buffer);
                a.lea(D, D, 128);
                a.rr(0x29, D, SI, true);
                super::platform::diagnostic_write(a, 2);
            }
            a.immediate(LINE, self.site.line as u64);
            a.immediate(FILE, self.site.file as u64);
            a.immediate(A, 7);
            a.jump(p.trap);
            a.bind(ready);
            a.address(R11, self.values);
            a.store(R11, self.offset(index), A);
            if matches!(grant, EntryGrant::Stop) { a.immediate(A, 0); a.store(R11, self.offset(index) + 8, A); }
        }
    }
    pub fn load(&self, a: &mut Assembler, index: usize, register: u8) {
        a.address(R11, self.values);
        if matches!(self.grants[index], EntryGrant::Stop) { a.lea(register, R11, self.offset(index)); }
        else { a.load(register, R11, self.offset(index)); }
    }
    pub fn data(&self, a: &mut Assembler, bytes: &mut Vec<u8>, offset: usize) {
        a.data_label(self.values, offset + bytes.len());
        for grant in &self.grants {
            bytes.extend_from_slice(&[0u8; 8]);
            if matches!(grant, EntryGrant::Stop) { bytes.extend_from_slice(&[0u8; 8]); }
        }
        a.data_label(self.failure_buffer, offset + bytes.len());
        if self.grants.iter().any(|grant| Self::fallible(grant)) { bytes.extend_from_slice(&[0u8; 128]); }
        for (index, grant) in self.grants.iter().enumerate() {
            a.data_label(self.paths[index], offset + bytes.len());
            match grant {
                EntryGrant::Directory(path) => bytes.extend_from_slice(path),
                EntryGrant::Listener(address) | EntryGrant::Connector(address) => bytes.extend_from_slice(address),
                _ => {}
            }
        }
    }
}
