//! Source standard library, also buildable by rustc during bootstrap.
mod prelude;
pub use prelude::{Option, Result};
pub mod bytes;
mod ordered_map;
pub mod str;
pub use ordered_map::BTreeMap;
mod string;
pub use string::{FromUtf8Error, String};
mod sequence;
pub use sequence::__lkj_vec_compare;

pub mod collections {
    pub use crate::BTreeMap;
}
