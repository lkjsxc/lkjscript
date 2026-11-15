// expect: reject E_TYPE
// pending: Name the missing enum variants in a non-exhaustive match diagnostic so a new capability does not require searching every type inventory.
enum Authority { Arguments, Environment, ExitStatus }
pub fn probe() -> usize {
    let value = Authority::Environment;
    match value { Authority::Arguments => 1usize }
}
