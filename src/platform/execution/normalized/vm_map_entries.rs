//! Project only actual entries of one live, admitted immutable parent.
//! The cursor never accepts a caller-selected raw child, type or origin.
use super::super::super::map::{Charge, Iter};
use super::super::super::prepare::NormalizedProgram;
use super::super::super::value::{NormalizedValue, projected_value_bytes};
use super::{Class, Value, admission_error};
use crate::platform::execution::ExecutionError;

pub(in super::super) struct Entries<'a> {
    parent: &'a Value,
    cursor: Iter<'a>,
}

impl Value {
    pub(in super::super) fn map_entries(
        &self,
        program: &NormalizedProgram,
    ) -> Result<Entries<'_>, ExecutionError> {
        if self.origin != program.value_origin || self.class != Class::Free {
            return Err(admission_error(
                "entry projection requires this program's admitted immutable map",
            ));
        }
        let NormalizedValue::Map(map) = &self.raw else {
            return Err(admission_error("entry projection requires a map"));
        };
        Ok(Entries {
            parent: self,
            cursor: map.iter(),
        })
    }
}

impl Entries<'_> {
    pub(in super::super) fn next(
        &mut self,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Option<(Value, Value)>, ExecutionError> {
        // Poll before bounded cursor movement, before any owned clone and after
        // construction. A failure aborts the enclosing projection, not its parent.
        reserve(Charge::default())?;
        let Some((key, value)) = self.cursor.next() else {
            return Ok(None);
        };
        reserve(Charge {
            slots: 0,
            bytes: projected_value_bytes(value)?,
        })?;
        let child = Value {
            raw: value.clone(),
            origin: self.parent.origin,
            class: Class::Free,
            borrow: None,
        };
        let key = Value {
            raw: key.to_value(),
            origin: self.parent.origin,
            class: Class::Free,
            borrow: None,
        };
        reserve(Charge::default())?;
        Ok(Some((key, child)))
    }
}
