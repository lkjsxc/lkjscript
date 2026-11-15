// expect: reject E_TYPE
// Diagnostic: A non-exhaustive enum match names the entirely missing variants; guarded arms do not establish coverage.
enum Authority { Arguments, Environment, ExitStatus }
pub fn probe() -> usize {
    let value = Authority::Environment;
    match value { Authority::Arguments => 1usize }
}
