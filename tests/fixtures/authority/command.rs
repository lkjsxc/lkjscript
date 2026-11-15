struct CommandCleanup { output: Stdout }
impl Drop for CommandCleanup {
    fn drop(&mut self) { self.output.write(b"cleanup\n"); }
}

pub fn main(mut args: Arguments, mut value: Environment, mut input: Stdin, mut output: Stdout, mut status: ExitStatus) {
    for argument in args.args() {
        output.write(b"arg:");
        output.write(&argument);
        output.write(b"\n");
    }
    match value.get() {
        Some(bytes) => { output.write(b"value:"); output.write(&bytes); output.write(b"\n"); }
        None => output.write(b"absent\n"),
    }
    let bytes = input.read_to_end().unwrap();
    output.write(&bytes);
    let cleanup = CommandCleanup { output };
    status.set_status(7u8);
    drop(cleanup);
}
