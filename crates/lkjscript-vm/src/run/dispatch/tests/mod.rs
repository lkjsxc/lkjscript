use lkjscript_core::{ExecutionPolicy, Op, Value};

use crate::run::NoTier as NullJit;

use super::dispatch;
use crate::run::data::list_values_equal;
use crate::run::{test_chunk, Vm};

macro_rules! test_vm {
    ($name:ident) => {
        let chunk = test_chunk();
        let mut $name = Vm::new(
            &chunk,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionPolicy::unrestricted(),
        );
    };
}

fn compare(vm: &mut Vm<'_, NullJit>, op: Op, left: Value, right: Value) -> bool {
    vm.push(left);
    vm.push(right);
    dispatch(vm, op as u8).expect("comparison succeeds");
    vm.pop()
        .expect("comparison result")
        .as_bool()
        .expect("Bool result")
}

fn test_i64(_vm: &mut Vm<'_, NullJit>, number: i64) -> Value {
    Value::from_i64(number)
}

fn i64_list(vm: &mut Vm<'_, NullJit>, values: &[i64]) -> Value {
    let mut list = Value::EMPTY_LIST;
    for number in values.iter().rev() {
        let head = test_i64(vm, *number);
        list = vm.list_prepend(head, list).expect("test list prepend");
    }
    list
}

mod equality;
mod identity;
mod lists;
