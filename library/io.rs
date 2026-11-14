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

fn unit_result(value: i64) -> Result<(), i64> {
    if value < 0 {
        Result::Err(-value)
    } else {
        Result::Ok(())
    }
}

pub fn __lkjscript_temporary_file(world: &mut World) -> Result<File, i64> {
    world.__lkjscript_file(0u8)
}
pub fn __lkjscript_stdin_file(world: &mut World) -> Result<File, i64> {
    world.__lkjscript_file(1u8)
}
pub fn __lkjscript_stdout_file(world: &mut World) -> Result<File, i64> {
    world.__lkjscript_file(2u8)
}
pub fn __lkjscript_stderr_file(world: &mut World) -> Result<File, i64> {
    world.__lkjscript_file(3u8)
}
pub fn __lkjscript_executable(world: &mut World) -> Result<File, i64> {
    world.__lkjscript_file(4u8)
}
pub fn __lkjscript_file_readonly(file: &mut File) -> Result<File, i64> {
    file.__lkjscript_readonly()
}
pub fn __lkjscript_file_rewind(file: &mut File) -> Result<(), i64> {
    unit_result(file.__lkjscript_rewind())
}
pub fn __lkjscript_file_executable(file: &mut File) -> Result<(), i64> {
    unit_result(file.__lkjscript_executable())
}

fn word(bytes: &[u8], offset: usize) -> usize {
    let mut value = 0usize;
    let mut index = 0usize;
    while index < 8 {
        value = value | ((bytes[offset + index] as usize) << ((index * 8) as u64));
        index += 1;
    }
    value
}
pub fn __lkjscript_clock_nanoseconds(clock: &mut Clock) -> Result<usize, i64> {
    let mut time = [0u8; 16];
    unit_result(clock.__lkjscript_read(&mut time))?;
    Result::Ok(word(&time, 0) * 1_000_000_000 + word(&time, 8))
}

pub fn __lkjscript_process_spawn(
    process: &mut Process,
    executable: &File,
    arguments: &[Vec<u8>],
    input: &File,
    output: &File,
    errors: &File,
    result: &File,
) -> Result<Child, i64> {
    let mut bytes = Vec::new();
    append_bytes(&mut bytes, b"lkjscript\0");
    for argument in arguments {
        for byte in argument {
            if *byte == 0u8 {
                return Result::Err(22);
            }
            bytes.push(*byte);
        }
        bytes.push(0u8);
    }
    process.__lkjscript_spawn(executable, &bytes, input, output, errors, result)
}

fn process_status(status: i64) -> i64 {
    let signal = status & 127;
    if signal == 0 {
        (status >> 8u64) & 255
    } else {
        128 + signal
    }
}
pub fn __lkjscript_child_wait(child: &mut Child) -> Result<i64, i64> {
    let status = child.__lkjscript_wait(false);
    if status < 0 {
        Result::Err(-status)
    } else {
        Result::Ok(process_status(status - 1))
    }
}
pub fn __lkjscript_child_try_wait(child: &mut Child) -> Result<crate::Option<i64>, i64> {
    let status = child.__lkjscript_wait(true);
    if status < 0 {
        Result::Err(-status)
    } else if status == 0 {
        Result::Ok(crate::Option::None)
    } else {
        Result::Ok(crate::Option::Some(process_status(status - 1)))
    }
}
pub fn __lkjscript_child_terminate(child: &mut Child) -> Result<(), i64> {
    unit_result(child.__lkjscript_terminate())
}

pub fn __lkjscript_open_directory(directory: &mut Directory, path: &str) -> Result<Directory, i64> {
    directory.__lkjscript_open_directory(&path_bytes(path)?)
}
fn parent(directory: &mut Directory, path: &str) -> Result<(Directory, Vec<u8>), i64> {
    let mut split = 0usize;
    let mut index = 0usize;
    let bytes = path.as_bytes();
    while index < bytes.len() {
        if bytes[index] == b'/' {
            split = index + 1;
        }
        index += 1;
    }
    let parent = if split == 0 { "." } else { &path[..split] };
    let name = &path[split..];
    if name.is_empty() || name == "." || name == ".." {
        return Result::Err(22);
    }
    let owner = __lkjscript_open_directory(directory, parent)?;
    Result::Ok((owner, path_bytes(name)?))
}
pub fn __lkjscript_create_directory(directory: &mut Directory, path: &str) -> Result<(), i64> {
    let (mut owner, name) = parent(directory, path)?;
    unit_result(owner.__lkjscript_create_directory(&name))
}
pub fn __lkjscript_directory_remove(directory: &mut Directory, path: &str) -> Result<(), i64> {
    let (mut owner, name) = parent(directory, path)?;
    unit_result(owner.__lkjscript_remove(&name, false))
}
pub fn __lkjscript_directory_remove_directory(
    directory: &mut Directory,
    path: &str,
) -> Result<(), i64> {
    let (mut owner, name) = parent(directory, path)?;
    unit_result(owner.__lkjscript_remove(&name, true))
}
pub fn __lkjscript_directory_rename(
    directory: &mut Directory,
    old: &str,
    new: &str,
) -> Result<(), i64> {
    let (mut from, old_name) = parent(directory, old)?;
    let (mut to, new_name) = parent(directory, new)?;
    unit_result(from.__lkjscript_rename(&old_name, &mut to, &new_name))
}
pub fn __lkjscript_directory_list(directory: &mut Directory) -> Result<Vec<Vec<u8>>, i64> {
    let mut opened = __lkjscript_open_directory(directory, ".")?;
    let mut names = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = count_result(opened.__lkjscript_read(&mut buffer))?;
        if count == 0 {
            break;
        }
        let mut at = 0usize;
        while at < count {
            if count - at < 20 {
                return Result::Err(5);
            }
            let record = buffer[at + 16] as usize | ((buffer[at + 17] as usize) << 8u64);
            if record < 20 || record > count - at {
                return Result::Err(5);
            }
            let mut end = at + 19;
            while end < at + record && buffer[end] != 0u8 {
                end += 1;
            }
            let name = &buffer[at + 19..end];
            if name != b"." && name != b".." {
                names.push(name.to_vec());
            }
            at += record;
        }
    }
    Result::Ok(names)
}
