// expect: reject E_NAME
// Diagnostic: name the cfg(test) exclusion and suggest an unconditional helper.
#[cfg(test)]
fn fixture() -> usize { 1usize }
pub fn probe() -> usize { fixture() }
