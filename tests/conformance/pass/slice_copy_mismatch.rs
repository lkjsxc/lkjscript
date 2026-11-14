pub fn probe() {
    let mut values = vec![0u8, 0u8];
    let source = vec![1u8];
    values.as_mut_slice().copy_from_slice(&source);
}
