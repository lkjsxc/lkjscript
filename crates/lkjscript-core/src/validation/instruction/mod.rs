mod algebraic;
mod byte_data;
mod bytes;
mod calls;
mod collections;
mod data;
mod enums;
mod io;
mod numeric;
#[path = "routing/required.rs"]
mod required;
mod sqlite;
mod structural;
mod system;
#[path = "system/types.rs"]
mod system_types;
mod types;
mod unique;

use super::{decode::instruction_error, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

include!("routing/dispatch.rs");
