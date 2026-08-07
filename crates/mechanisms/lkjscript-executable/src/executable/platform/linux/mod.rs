#![allow(unsafe_code)]

use std::ffi::c_void;
use std::ptr::NonNull;

use crate::executable::{
    machine_arguments, InstallError, InvocationError, IslandCallState, MachineArgument,
    MappingPermissions, NativeCallState, NativeStackBoundary, NativeValue, PermissionProbeError,
    RawReturn, Signature, ValueType,
};

const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const PROT_EXEC: i32 = 0x4;
const MAP_PRIVATE: i32 = 0x2;
const MAP_ANONYMOUS: i32 = 0x20;

unsafe extern "C" {
    fn mmap(
        address: *mut c_void,
        length: usize,
        protection: i32,
        flags: i32,
        descriptor: i32,
        offset: isize,
    ) -> *mut c_void;
    fn mprotect(address: *mut c_void, length: usize, protection: i32) -> i32;
    fn munmap(address: *mut c_void, length: usize) -> i32;
    fn sysconf(name: i32) -> isize;
    fn pthread_self() -> usize;
    fn pthread_getattr_np(thread: usize, attributes: *mut c_void) -> i32;
    fn pthread_attr_getstack(
        attributes: *const c_void,
        stack_address: *mut *mut c_void,
        stack_size: *mut usize,
    ) -> i32;
    fn pthread_attr_getguardsize(attributes: *const c_void, guard_size: *mut usize) -> i32;
    fn pthread_attr_destroy(attributes: *mut c_void) -> i32;
}

const SC_PAGESIZE: i32 = 30;

#[derive(Debug)]
pub(in crate::executable) struct Mapping {
    base: NonNull<u8>,
    length: usize,
    allocation_length: usize,
    sealed: bool,
    wx_transition_verified: bool,
}

// SAFETY: Mapping uniquely owns one mmap allocation. Moving ownership does not
// invalidate the address, and Drop unmaps exactly once after ownership ends.
unsafe impl Send for Mapping {}
// SAFETY: Mapping is exposed across threads only through InstalledImage after
// seal_rx succeeds. The mapping is then immutable RX memory; invocation creates
// per-call state and performs no writes through Mapping.
unsafe impl Sync for Mapping {}

mod abi_call;
mod invocation;
mod mapping;
mod stack;

use abi_call::invoke_typed;

pub(in crate::executable) use stack::{
    native_stack_bounds, native_stack_reservation_fits, NativeStackBounds,
};
