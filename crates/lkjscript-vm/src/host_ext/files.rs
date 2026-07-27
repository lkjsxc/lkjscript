use super::*;

impl ResourceTable {
    pub fn sys_open_read(&mut self, path: &[u8]) -> Result<Value> {
        self.ensure_capacity()?;
        let file = lkjscript_sys::open_read(path)
            .map_err(|error| Error::msg(format!("open-file-reader: {error}")))?;
        self.push(OwnedResource::File {
            descriptor: file,
            kind: ResourceKind::FileReader,
        })
    }

    pub fn sys_open_write(&mut self, path: &[u8]) -> Result<Value> {
        self.ensure_capacity()?;
        let file = lkjscript_sys::open_write(path)
            .map_err(|error| Error::msg(format!("open-file-writer: {error}")))?;
        self.push(OwnedResource::File {
            descriptor: file,
            kind: ResourceKind::FileWriter,
        })
    }

    pub fn sys_open_append(&mut self, path: &[u8]) -> Result<Value> {
        self.ensure_capacity()?;
        let file = lkjscript_sys::open_append(path)
            .map_err(|error| Error::msg(format!("open-file-appender: {error}")))?;
        self.push(OwnedResource::File {
            descriptor: file,
            kind: ResourceKind::FileAppender,
        })
    }

    pub fn sys_open_create_new(&mut self, path: &[u8]) -> Result<Value> {
        self.ensure_capacity()?;
        let file = lkjscript_sys::open_create_new(path)
            .map_err(|error| Error::msg(format!("create-file: {error}")))?;
        self.push(OwnedResource::File {
            descriptor: file,
            kind: ResourceKind::FileWriter,
        })
    }

    pub fn sys_open_dir(&mut self, path: &[u8]) -> Result<Value> {
        self.ensure_capacity()?;
        let directory = lkjscript_sys::open_dir(path)
            .map_err(|error| Error::msg(format!("sys-open-dir: {error}")))?;
        self.push(OwnedResource::Directory(directory))
    }

    /// Files and directory handles may be synced; directories make a prior
    /// same-filesystem rename durable. Sockets and stale handles are rejected.
    pub fn sys_fsync(&self, handle: Value) -> Result<Value> {
        let raw = self.sync_raw(handle, "sync-file")?;
        lkjscript_sys::fsync_fd(raw).map_err(|error| Error::msg(format!("sys-fsync: {error}")))?;
        Ok(Value::UNIT)
    }

    /// Only regular file capabilities may be truncated; directory and socket
    /// handles are rejected before the OS call.
    pub fn sys_truncate(&self, handle: Value, length: i64) -> Result<Value> {
        if length < 0 {
            return Err(Error::msg("sys-truncate length out of range"));
        }
        let raw = self.file_raw(handle, "truncate-file")?;
        lkjscript_sys::truncate_fd(raw, length)
            .map_err(|error| Error::msg(format!("sys-truncate: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sys_rename(from: &[u8], to: &[u8]) -> Result<Value> {
        lkjscript_sys::rename_path(from, to)
            .map_err(|error| Error::msg(format!("sys-rename: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sys_path_exists(path: &[u8]) -> Result<Value> {
        let exists = lkjscript_sys::path_exists(path)
            .map_err(|error| Error::msg(format!("sys-path-exists: {error}")))?;
        Ok(Value::from_bool(exists))
    }
}
