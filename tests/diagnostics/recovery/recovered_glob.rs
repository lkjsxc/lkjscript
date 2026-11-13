mod declarations {
    pub fn unavailable() -> impl Iterator<Item = i64> { unreachable!() }
}
mod first {
    pub use crate::declarations::*;
    pub fn dependent() -> i64 { unavailable() }
}
mod second {
    use crate::first::*;
    fn dependent() -> i64 { unavailable() }
}
mod shadow {
    use crate::declarations::*;
    fn unavailable() -> i64 { 42 }
    fn independent() -> bool { unavailable() }
}
