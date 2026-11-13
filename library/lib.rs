//! Source standard library, also buildable by rustc during bootstrap.
mod prelude;
pub use prelude::{Option, Result};
pub mod bytes;
pub mod char;
pub mod fmt;
mod ordered_map;
mod ordered_set;
pub mod str;
pub use ordered_map::BTreeMap;
pub use ordered_set::BTreeSet;
mod string;
pub use string::{FromUtf8Error, String};
mod sequence;
pub use sequence::{__lkj_vec_clone, __lkj_vec_compare, __lkj_vec_equal, __lkj_vec_repeat};

pub mod collections {
    pub use crate::{BTreeMap, BTreeSet};
}

pub mod option {
    pub use crate::prelude::Option;
    pub use crate::prelude::Option::{None, Some};
}
pub mod result {
    pub use crate::prelude::Result;
    pub use crate::prelude::Result::{Err, Ok};
}
