// expect: check
// authority-expect: Notice::drop [custom Drop]; stdout_write; directory_close
struct Notice {
    output: Stdout,
}
impl Drop for Notice {
    fn drop(&mut self) {
        self.output.write(b"closed\n");
    }
}
struct Held {
    notices: Vec<Option<Box<Notice>>>,
    tree: Directory,
}
pub fn main(send: Stdout, tree: Directory) {
    let held = Held {
        notices: vec![Some(Box::new(Notice { output: send }))],
        tree,
    };
    drop(held);
}
