// expect: trap R0018
// Unique immutable literal captures retain their exact static trap explanation.
pub fn probe() { let problem = "lock journal: errno 11"; panic!("journal recovery: {problem}"); }
