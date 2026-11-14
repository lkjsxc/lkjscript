//! Capability convenience methods are source code. Raw operations require a
//! checked authority value and return Linux errors as negative i64 values.
use crate::Result;
use crate::bytes::append_bytes;

pub fn __lkjscript_args(world: &mut World) -> Vec<Vec<u8>> {
    let count = world.__lkjscript_arg_count();
    let mut values: Vec<Vec<u8>> = Vec::new();
    let mut index = 0usize;
    while index < count {
        let length = world.__lkjscript_arg_len(index);
        let mut bytes = vec![0u8; length];
        let copied = world.__lkjscript_arg_copy(index, &mut bytes);
        assert_eq!(copied, length);
        values.push(bytes);
        index = index + 1;
    }
    values
}

fn path_bytes(path: &str) -> Result<Vec<u8>, i64> {
    let input = path.as_bytes();
    let mut bytes = Vec::new();
    let mut index = 0usize;
    while index < input.len() {
        if input[index] == 0u8 {
            return Result::Err(22);
        }
        bytes.push(input[index]);
        index = index + 1;
    }
    bytes.push(0u8);
    Result::Ok(bytes)
}

pub fn __lkjscript_directory(world: &mut World, path: &str) -> Result<Directory, i64> {
    let bytes = path_bytes(path)?;
    world.__lkjscript_directory(&bytes)
}

pub fn __lkjscript_open(directory: &mut Directory, path: &str) -> Result<File, i64> {
    let bytes = path_bytes(path)?;
    directory.__lkjscript_open(&bytes, false)
}

pub fn __lkjscript_create(directory: &mut Directory, path: &str) -> Result<File, i64> {
    let bytes = path_bytes(path)?;
    directory.__lkjscript_open(&bytes, true)
}

fn count_result(count: i64) -> Result<usize, i64> {
    if count < 0 {
        Result::Err(-count)
    } else {
        Result::Ok(count as usize)
    }
}

pub fn __lkjscript_stdin_read(input: &mut Stdin, bytes: &mut [u8]) -> Result<usize, i64> {
    count_result(input.__lkjscript_read(bytes))
}

pub fn __lkjscript_file_read(file: &mut File, bytes: &mut [u8]) -> Result<usize, i64> {
    count_result(file.__lkjscript_read(bytes))
}

pub fn __lkjscript_file_write(file: &mut File, bytes: &[u8]) -> Result<usize, i64> {
    count_result(file.__lkjscript_write(bytes))
}

pub fn __lkjscript_stdin_read_to_end(input: &mut Stdin) -> Result<Vec<u8>, i64> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = __lkjscript_stdin_read(input, &mut buffer)?;
        if count == 0 {
            break;
        }
        append_bytes(&mut bytes, &buffer[..count]);
    }
    Result::Ok(bytes)
}

pub fn __lkjscript_file_read_to_end(file: &mut File) -> Result<Vec<u8>, i64> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = __lkjscript_file_read(file, &mut buffer)?;
        if count == 0 {
            break;
        }
        append_bytes(&mut bytes, &buffer[..count]);
    }
    Result::Ok(bytes)
}

pub fn __lkjscript_file_write_all(file: &mut File, bytes: &[u8]) -> Result<(), i64> {
    let mut position = 0usize;
    while position < bytes.len() {
        let count = __lkjscript_file_write(file, &bytes[position..])?;
        if count == 0 {
            return Result::Err(5);
        }
        position = position + count;
    }
    Result::Ok(())
}
