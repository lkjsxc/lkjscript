struct Task<F> {
    call: F,
}
fn run<F: FnOnce() -> i64>(task: Task<F>) -> i64 {
    (task.call)()
}
pub fn main(mut world: World) {
    let mut output: Stdout = world.stdout();
    let task = Task {
        call: move || {
            output.write(&[52u8, 50u8, 10u8]);
            42i64
        },
    };
    assert_eq!(run(task), 42);
}
