impl World {
    fn __lkj_arg_count(&mut self) -> usize {
        999
    }
}
pub fn main(mut world: World) {
    assert_eq!(world.__lkj_arg_count(), world.args().len());
}
