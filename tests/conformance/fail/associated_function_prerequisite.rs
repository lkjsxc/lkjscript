trait Eligible {}
trait Build {
    fn make() -> i64;
}
impl<T: Eligible> Build for T {
    fn make() -> i64 {
        42
    }
}
struct Missing {}
pub fn probe() -> i64 {
    Missing::make()
}
