// expect: reject E_NAME
// pending: Name fixture's exclusion by cfg(test) (also excluded by lkjscript test today) and suggest an unconditional helper, instead of reporting an unresolved declaration without its cause.
#[cfg(test)]
fn fixture() -> usize { 1usize }
pub fn probe() -> usize { fixture() }
