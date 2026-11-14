// expect: check
// authority-expect: declared input:Stdin errors:Stderr time:Clock launch:Process
// authority-expect: reachable operations none
pub fn main(input: Stdin, errors: Stderr, time: Clock, launch: Process) {
    drop(input);
    drop(errors);
    drop(time);
    drop(launch);
}
