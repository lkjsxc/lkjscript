// expect: trap R0018
// pending: Preserve the formatted panic explanation, or tell the author to write it through an output capability before trapping.
pub fn probe() { let problem = "lock journal: errno 11"; panic!("journal recovery: {problem}"); }
