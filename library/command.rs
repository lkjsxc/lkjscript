//! Typed command helpers. The current loader mounts this optional library module.
//! Keeping it outside lib.rs lets the stored seed compile the compiler without
//! resolving capability types that were introduced after that seed.
pub fn __lkjscript_command_args(arguments: &mut Arguments) -> Vec<Vec<u8>> {
    let count = arguments.__lkjscript_arg_count();
    let mut values: Vec<Vec<u8>> = Vec::new();
    let mut index = 0usize;
    while index < count {
        let length = arguments.__lkjscript_arg_len(index);
        let mut bytes = vec![0u8; length];
        let copied = arguments.__lkjscript_arg_copy(index, &mut bytes);
        assert_eq!(copied, length);
        values.push(bytes);
        index += 1;
    }
    values
}

pub fn __lkjscript_environment_get(environment: &mut Environment) -> Option<Vec<u8>> {
    let length = environment.__lkjscript_len();
    if length < 0 { return Option::None; }
    let mut bytes = vec![0u8; length as usize];
    let copied = environment.__lkjscript_copy(&mut bytes);
    assert_eq!(copied, length as usize);
    Option::Some(bytes)
}
