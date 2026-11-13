pub fn main(mut world: World) {
    let arguments = world.args();
    assert_eq!(arguments.len(), 1usize);
    let payload = arguments[0].clone();
    assert_eq!(world.__lkj_arg_count(), 1usize);
    assert_eq!(world.__lkj_arg_len(0usize), payload.len());
    let mut bytes = [0u8; 2];
    let count = world.__lkj_arg_copy(0usize, &mut bytes);
    let expected = if payload.len() < 2usize {
        payload.len()
    } else {
        2usize
    };
    assert_eq!(count, expected);
    let mut index = 0usize;
    while index < count {
        assert_eq!(bytes[index], payload[index]);
        index = index + 1;
    }
    let mut empty: [u8; 0] = [];
    assert_eq!(world.__lkj_arg_copy(0usize, &mut empty), 0usize);
    let mut out = world.stdout();
    out.write(b"arguments ok\n");
}
