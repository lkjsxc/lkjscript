use calls::{indirect_call, lower_direct_call};
use constants::*;
use failure_cleanup::lower_failure_cleanup;
pub(in crate::lower) use failure_cleanup::lower_failure_cleanup_id;
use output::write_instruction_output;
pub(in crate::lower) use structural::{
    consuming_operand, copy_call_argument as copy_structural_call_argument,
    lower_drop as lower_structural_drop, lower_terminal_cleanup as lower_structural_terminal_cleanup,
    lower_trap_message as lower_structural_trap_message, structural_call,
};
pub(in crate::lower) use runtime_bytes::lower_bytes_runtime;
use unique::*;
