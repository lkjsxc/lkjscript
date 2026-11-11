struct Task<F> { call: F }
fn run<F: FnOnce() -> i64>(task: Task<F>) -> i64 { (task.call)() }
pub fn main(world: World) -> i64 {
    let task = Task { call: move || { world.write_i64(42); 42 } };
    run(task)
}
