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
}
impl Plan {
    pub fn new(a: &mut Assembler, grants: &[EntryGrant], site: Span) -> Self {
        let values = a.label();
        let mut paths = Vec::new();
        for _ in grants { paths.push(a.label()); }
        Self { values, paths, grants: grants.to_vec(), site }
    }
    pub fn prepare(&self, a: &mut Assembler, p: Platform) {
        for (index, grant) in self.grants.iter().enumerate() {
            match grant {
                EntryGrant::Directory(path) => {
                    a.address(DI, self.paths[index]);
                    a.immediate(SI, path.len() as u64);
                    a.call(p.directory_open);
                }
                EntryGrant::Listener(address) => {
                    a.address(DI, self.paths[index]);
                    a.immediate(SI, address.len() as u64);
                    a.call(p.world_listen);
                }
                _ => { a.immediate(A, 1); },
            }
            let ready = a.label();
            a.test(A, true);
            a.condition(9, ready);
            a.immediate(LINE, self.site.line as u64);
            a.immediate(FILE, self.site.file as u64);
            a.immediate(A, 7);
            a.jump(p.trap);
            a.bind(ready);
            a.address(R11, self.values);
            a.store(R11, index as i64 * 8, A);
        }
    }
    pub fn load(&self, a: &mut Assembler, index: usize, register: u8) {
        a.address(R11, self.values);
        a.load(register, R11, index as i64 * 8);
    }
    pub fn data(&self, a: &mut Assembler, bytes: &mut Vec<u8>, offset: usize) {
        a.data_label(self.values, offset + bytes.len());
        for _ in &self.grants { bytes.extend_from_slice(&[0u8; 8]); }
        for (index, grant) in self.grants.iter().enumerate() {
            a.data_label(self.paths[index], offset + bytes.len());
            match grant {
                EntryGrant::Directory(path) => bytes.extend_from_slice(path),
                EntryGrant::Listener(address) => bytes.extend_from_slice(address),
                _ => {}
            }
        }
    }
}
