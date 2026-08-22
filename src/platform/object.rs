//! Generic named object storage with local and S3-compatible production bindings.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{CallPolicy, CapabilityAdapter, ExecutionError, ExecutionFailureClass};
use super::semantic::OwnerId;
use super::stream::StreamRegistry;
use super::value::Value;
use bytes::Bytes;
use futures_util::TryStreamExt;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
use object_store::path::Path;
use object_store::{
    Attribute, Attributes, ObjectStore, ObjectStoreExt, PutMode, PutMultipartOptions, PutOptions,
    PutPayload,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path as FilePath;
use std::sync::Arc;
use tokio::runtime::Handle;

pub const OBJECT_ADAPTER_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_OBJECT_KEY_BYTES: usize = 1024;
pub const MAXIMUM_OBJECT_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MULTIPART_PART_BYTES: usize = 5 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectLimits {
    pub maximum_object_bytes: u64,
    pub maximum_whole_read_bytes: usize,
}

impl Default for ObjectLimits {
    fn default() -> Self {
        Self {
            maximum_object_bytes: 1024 * 1024 * 1024,
            maximum_whole_read_bytes: 4 * 1024 * 1024,
        }
    }
}

impl ObjectLimits {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.maximum_object_bytes == 0 || self.maximum_object_bytes > MAXIMUM_OBJECT_BYTES {
            return Err(object_diagnostic(
                "object_size_limit",
                format!("maximum_object_bytes must be 1 through {MAXIMUM_OBJECT_BYTES}"),
            ));
        }
        if self.maximum_whole_read_bytes == 0
            || u64::try_from(self.maximum_whole_read_bytes)
                .map_or(true, |value| value > self.maximum_object_bytes)
        {
            return Err(object_diagnostic(
                "object_read_limit",
                "maximum_whole_read_bytes must be positive and no larger than maximum_object_bytes",
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct S3Credentials {
    access_key_id: Arc<str>,
    secret_access_key: Arc<str>,
}

impl S3Credentials {
    pub fn new(
        access_key_id: impl Into<Arc<str>>,
        secret_access_key: impl Into<Arc<str>>,
    ) -> Result<Self, Diagnostic> {
        let access_key_id = access_key_id.into();
        let secret_access_key = secret_access_key.into();
        if access_key_id.is_empty()
            || access_key_id.len() > 4096
            || secret_access_key.is_empty()
            || secret_access_key.len() > 4096
        {
            return Err(object_diagnostic(
                "object_s3_credentials",
                "S3 credential fields must contain 1 through 4096 bytes",
            ));
        }
        Ok(Self {
            access_key_id,
            secret_access_key,
        })
    }
}

impl fmt::Debug for S3Credentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("S3Credentials(<redacted>)")
    }
}

#[derive(Clone, Debug)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub prefix: String,
    pub allow_http: bool,
    pub path_style: bool,
    pub credentials: S3Credentials,
}

#[derive(Clone)]
pub struct ObjectStorageAdapter {
    interface: OwnerId,
    store: Arc<dyn ObjectStore>,
    runtime: Handle,
    streams: StreamRegistry,
    persists_provider_attributes: bool,
    prefix: String,
    limits: ObjectLimits,
}

impl fmt::Debug for ObjectStorageAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObjectStorageAdapter")
            .field("interface", &self.interface)
            .field("store", &"<generic object store>")
            .field(
                "persists_provider_attributes",
                &self.persists_provider_attributes,
            )
            .field("prefix", &self.prefix)
            .field("limits", &self.limits)
            .finish()
    }
}

impl ObjectStorageAdapter {
    pub fn new(
        interface: OwnerId,
        store: Arc<dyn ObjectStore>,
        runtime: Handle,
        streams: StreamRegistry,
        persists_provider_attributes: bool,
        prefix: String,
        limits: ObjectLimits,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        validate_prefix(&prefix)?;
        Ok(Self {
            interface,
            store,
            runtime,
            streams,
            persists_provider_attributes,
            prefix,
            limits,
        })
    }

    pub fn in_memory(
        interface: OwnerId,
        runtime: Handle,
        streams: StreamRegistry,
        prefix: String,
        limits: ObjectLimits,
    ) -> Result<Self, Diagnostic> {
        Self::new(
            interface,
            Arc::new(InMemory::new()),
            runtime,
            streams,
            true,
            prefix,
            limits,
        )
    }

    pub fn local(
        interface: OwnerId,
        runtime: Handle,
        streams: StreamRegistry,
        root: &FilePath,
        prefix: String,
        limits: ObjectLimits,
    ) -> Result<Self, Diagnostic> {
        let store = LocalFileSystem::new_with_prefix(root).map_err(|_| {
            object_diagnostic(
                "object_local_root",
                "local object root is unavailable or invalid",
            )
        })?;
        // `object_store::local::LocalFileSystem` has exact create semantics but deliberately
        // rejects provider attributes. The contract still validates content type; the local
        // deployment stores bytes and integrity facts without provider-visible metadata.
        Self::new(
            interface,
            Arc::new(store),
            runtime,
            streams,
            false,
            prefix,
            limits,
        )
    }

    pub fn s3(
        interface: OwnerId,
        runtime: Handle,
        streams: StreamRegistry,
        config: S3Config,
        limits: ObjectLimits,
    ) -> Result<Self, Diagnostic> {
        validate_deployment_token(&config.region, "S3 region")?;
        validate_deployment_token(&config.bucket, "S3 bucket")?;
        if config.endpoint.is_empty() || config.endpoint.len() > 4096 {
            return Err(object_diagnostic(
                "object_s3_endpoint",
                "S3 endpoint must contain 1 through 4096 bytes",
            ));
        }
        let store = AmazonS3Builder::new()
            .with_endpoint(config.endpoint)
            .with_region(config.region)
            .with_bucket_name(config.bucket)
            .with_access_key_id(config.credentials.access_key_id.to_string())
            .with_secret_access_key(config.credentials.secret_access_key.to_string())
            .with_allow_http(config.allow_http)
            .with_virtual_hosted_style_request(!config.path_style)
            .build()
            .map_err(|_| {
                object_diagnostic(
                    "object_s3_configuration",
                    "S3-compatible deployment configuration is invalid",
                )
            })?;
        Self::new(
            interface,
            Arc::new(store),
            runtime,
            streams,
            true,
            config.prefix,
            limits,
        )
    }

    fn put_new(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        let [key, stream, content_type] = arguments.as_slice() else {
            return Err(object_argument(
                "put-new expects key Text, byte stream, and StaticText content type",
            ));
        };
        let key = text(key, "object key")?;
        let path = self.path(key)?;
        let content_type = static_text(content_type, "object content type")?;
        validate_content_type(content_type)?;

        let mut total = 0u64;
        let mut digest = blake3::Hasher::new();
        let mut buffer = Vec::with_capacity(MULTIPART_PART_BYTES);
        let mut upload: Option<Box<dyn object_store::MultipartUpload>> = None;
        let mut temporary_path = None;
        loop {
            policy.control.check()?;
            let Some(chunk) = self.streams.read(stream, &policy.control)? else {
                break;
            };
            total = total
                .checked_add(u64::try_from(chunk.len()).map_err(|_| {
                    ExecutionError::resource(
                        "object_size_limit",
                        "object chunk length cannot be represented",
                    )
                })?)
                .ok_or_else(|| {
                    ExecutionError::resource(
                        "object_size_limit",
                        "object byte accounting overflowed",
                    )
                })?;
            if total > self.limits.maximum_object_bytes {
                if let Some(upload) = upload.as_mut() {
                    let _ = self.runtime.block_on(upload.abort());
                }
                return Err(ExecutionError::resource(
                    "object_size_limit",
                    "object exceeds its exact byte limit",
                ));
            }
            digest.update(&chunk);
            buffer.extend_from_slice(&chunk);
            while buffer.len() >= MULTIPART_PART_BYTES {
                if upload.is_none() {
                    let temporary = self.attempt_path()?;
                    upload = Some(self.start_upload(&temporary, content_type, policy)?);
                    temporary_path = Some(temporary);
                }
                let remainder = buffer.split_off(MULTIPART_PART_BYTES);
                let part = std::mem::replace(&mut buffer, remainder);
                let result = self.runtime.block_on(
                    upload
                        .as_mut()
                        .ok_or_else(|| object_internal("multipart upload disappeared"))?
                        .put_part(PutPayload::from(Bytes::from(part))),
                );
                if let Err(error) = result {
                    if let Some(upload) = upload.as_mut() {
                        let _ = self.runtime.block_on(upload.abort());
                    }
                    return Err(map_object_error(error, policy, false));
                }
            }
        }

        let (version, cleanup_pending) = if let Some(mut upload) = upload {
            if !buffer.is_empty()
                && let Err(error) = self.runtime.block_on(
                    upload.put_part(PutPayload::from(Bytes::from(std::mem::take(&mut buffer)))),
                )
            {
                let _ = self.runtime.block_on(upload.abort());
                return Err(map_object_error(error, policy, false));
            }
            self.runtime
                .block_on(upload.complete())
                .map_err(|error| map_object_error(error, policy, true))?;
            let temporary = temporary_path
                .as_ref()
                .ok_or_else(|| object_internal("multipart temporary path disappeared"))?;
            if let Err(error) = self
                .runtime
                .block_on(self.store.copy_if_not_exists(temporary, &path))
            {
                let _ = self.runtime.block_on(self.store.delete(temporary));
                return Err(map_object_error(error, policy, true));
            }
            let cleanup_pending = self.runtime.block_on(self.store.delete(temporary)).is_err();
            (String::new(), cleanup_pending)
        } else {
            let attributes = self.provider_attributes(content_type);
            let result = self
                .runtime
                .block_on(self.store.put_opts(
                    &path,
                    PutPayload::from(Bytes::from(buffer)),
                    PutOptions {
                        mode: PutMode::Create,
                        attributes,
                        ..PutOptions::default()
                    },
                ))
                .map_err(|error| map_object_error(error, policy, true))?;
            (result.version.unwrap_or_default(), false)
        };
        Ok(Value::record(
            None,
            [
                ("key".to_owned(), Value::text(key.to_owned())),
                (
                    "size".to_owned(),
                    Value::I64(i64::try_from(total).map_err(|_| {
                        ExecutionError::resource(
                            "object_size_limit",
                            "object size exceeds signed 64-bit range",
                        )
                    })?),
                ),
                (
                    "blake3".to_owned(),
                    Value::text(digest.finalize().to_hex().to_string()),
                ),
                ("version".to_owned(), Value::text(version)),
                ("cleanup_pending".to_owned(), Value::Bool(cleanup_pending)),
            ],
        ))
    }

    fn start_upload(
        &self,
        path: &Path,
        content_type: &str,
        policy: &CallPolicy,
    ) -> Result<Box<dyn object_store::MultipartUpload>, ExecutionError> {
        let attributes = self.provider_attributes(content_type);
        self.runtime
            .block_on(self.store.put_multipart_opts(
                path,
                PutMultipartOptions {
                    attributes,
                    ..PutMultipartOptions::default()
                },
            ))
            .map_err(|error| map_object_error(error, policy, false))
    }

    fn provider_attributes(&self, content_type: &str) -> Attributes {
        let mut attributes = Attributes::new();
        if self.persists_provider_attributes {
            attributes.insert(Attribute::ContentType, content_type.to_owned().into());
        }
        attributes
    }

    fn get(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        let [key, maximum] = arguments.as_slice() else {
            return Err(object_argument(
                "get expects key Text and maximum bytes I64",
            ));
        };
        let path = self.path(text(key, "object key")?)?;
        let maximum = self.maximum_read(maximum, policy)?;
        let result = self
            .runtime
            .block_on(self.store.get(&path))
            .map_err(|error| map_object_error(error, policy, false))?;
        if result.meta.size > u64::try_from(maximum).unwrap_or(u64::MAX) {
            return Err(ExecutionError::resource(
                "object_read_limit",
                "object exceeds the bounded whole-read maximum",
            ));
        }
        let bytes = self
            .runtime
            .block_on(result.bytes())
            .map_err(|error| map_object_error(error, policy, false))?;
        if bytes.len() > maximum {
            return Err(ExecutionError::resource(
                "object_read_limit",
                "object changed beyond the bounded whole-read maximum",
            ));
        }
        Ok(Value::bytes(bytes.to_vec()))
    }

    fn range(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        let [key, start, length] = arguments.as_slice() else {
            return Err(object_argument(
                "range expects key Text, start I64, and length I64",
            ));
        };
        let path = self.path(text(key, "object key")?)?;
        let start = nonnegative_u64(start, "object range start")?;
        let length = nonnegative_usize(length, "object range length")?;
        if length == 0 || length > self.limits.maximum_whole_read_bytes {
            return Err(ExecutionError::resource(
                "object_range_limit",
                "object range length is zero or exceeds its exact limit",
            ));
        }
        let end = start
            .checked_add(u64::try_from(length).map_err(|_| {
                ExecutionError::resource("object_range_limit", "object range length overflowed")
            })?)
            .ok_or_else(|| {
                ExecutionError::resource("object_range_limit", "object range overflowed")
            })?;
        let bytes = self
            .runtime
            .block_on(self.store.get_range(&path, start..end))
            .map_err(|error| map_object_error(error, policy, false))?;
        Ok(Value::bytes(bytes.to_vec()))
    }

    fn head(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        let [key] = arguments.as_slice() else {
            return Err(object_argument("head expects one key Text"));
        };
        let key = text(key, "object key")?;
        let path = self.path(key)?;
        let meta = self
            .runtime
            .block_on(self.store.head(&path))
            .map_err(|error| map_object_error(error, policy, false))?;
        Ok(Value::record(
            None,
            [
                ("key".to_owned(), Value::text(key.to_owned())),
                (
                    "size".to_owned(),
                    Value::I64(i64::try_from(meta.size).map_err(|_| {
                        ExecutionError::resource(
                            "object_size_limit",
                            "object size exceeds signed 64-bit range",
                        )
                    })?),
                ),
                (
                    "etag".to_owned(),
                    Value::text(meta.e_tag.unwrap_or_default()),
                ),
                (
                    "version".to_owned(),
                    Value::text(meta.version.unwrap_or_default()),
                ),
                (
                    "modified_milliseconds".to_owned(),
                    Value::I64(meta.last_modified.timestamp_millis()),
                ),
            ],
        ))
    }

    fn reconcile_put(
        &self,
        policy: &CallPolicy,
        arguments: Vec<Value>,
    ) -> Result<Value, ExecutionError> {
        let [key] = arguments.as_slice() else {
            return Err(object_argument("reconcile-put expects one key Text"));
        };
        let key = text(key, "object key")?;
        let path = self.path(key)?;
        let result = match self.runtime.block_on(self.store.get(&path)) {
            Ok(result) => result,
            Err(object_store::Error::NotFound { .. }) => {
                return Ok(Value::List(Arc::new(Vec::new())));
            }
            Err(error) => return Err(map_object_error(error, policy, false)),
        };
        if result.meta.size > self.limits.maximum_object_bytes {
            return Err(ExecutionError::resource(
                "object_size_limit",
                "reconciled object exceeds its exact byte limit",
            ));
        }
        let expected_size = result.meta.size;
        let version = result
            .meta
            .version
            .clone()
            .or_else(|| result.meta.e_tag.clone())
            .unwrap_or_default();
        let maximum = self.limits.maximum_object_bytes;
        let control = policy.control.clone();
        let (size, digest) = self.runtime.block_on(async move {
            let mut stream = result.into_stream();
            let mut size = 0u64;
            let mut digest = blake3::Hasher::new();
            while let Some(chunk) = stream
                .try_next()
                .await
                .map_err(|error| map_object_error(error, policy, false))?
            {
                control.check()?;
                size = size
                    .checked_add(u64::try_from(chunk.len()).map_err(|_| {
                        ExecutionError::resource(
                            "object_size_limit",
                            "reconciled object chunk length is not representable",
                        )
                    })?)
                    .ok_or_else(|| {
                        ExecutionError::resource(
                            "object_size_limit",
                            "reconciled object byte accounting overflowed",
                        )
                    })?;
                if size > maximum {
                    return Err(ExecutionError::resource(
                        "object_size_limit",
                        "reconciled object exceeds its exact byte limit",
                    ));
                }
                digest.update(&chunk);
            }
            Ok((size, digest.finalize().to_hex().to_string()))
        })?;
        if size != expected_size {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "object_reconcile_size",
                "object metadata and streamed bytes disagree during reconciliation",
            ));
        }
        Ok(Value::List(Arc::new(vec![Value::record(
            None,
            [
                ("key".to_owned(), Value::text(key.to_owned())),
                (
                    "size".to_owned(),
                    Value::I64(i64::try_from(size).map_err(|_| {
                        ExecutionError::resource(
                            "object_size_limit",
                            "object size exceeds signed 64-bit range",
                        )
                    })?),
                ),
                ("blake3".to_owned(), Value::text(digest)),
                ("version".to_owned(), Value::text(version)),
                ("cleanup_pending".to_owned(), Value::Bool(false)),
            ],
        )])))
    }

    fn delete(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        let [key] = arguments.as_slice() else {
            return Err(object_argument("delete expects one key Text"));
        };
        let path = self.path(text(key, "object key")?)?;
        match self.runtime.block_on(self.store.delete(&path)) {
            Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(Value::Unit),
            Err(error) => Err(map_object_error(error, policy, true)),
        }
    }

    fn path(&self, key: &str) -> Result<Path, ExecutionError> {
        validate_key(key)?;
        let complete = if self.prefix.is_empty() {
            key.to_owned()
        } else {
            format!("{}/{key}", self.prefix)
        };
        Path::parse(complete).map_err(|_| object_argument("object key is not canonical"))
    }

    fn attempt_path(&self) -> Result<Path, ExecutionError> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| {
            ExecutionError::new(
                ExecutionFailureClass::Capability,
                "object_attempt_random",
                "secure randomness for object upload attempt is unavailable",
            )
        })?;
        let identifier = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let key = format!("__lkjscript_attempts/{identifier}");
        let complete = if self.prefix.is_empty() {
            key
        } else {
            format!("{}/{key}", self.prefix)
        };
        Path::parse(complete).map_err(|_| object_internal("upload attempt path is invalid"))
    }

    fn maximum_read(&self, value: &Value, policy: &CallPolicy) -> Result<usize, ExecutionError> {
        let maximum = nonnegative_usize(value, "object whole-read maximum")?;
        let grant = policy
            .limits
            .get("maximum_read_bytes")
            .copied()
            .unwrap_or(self.limits.maximum_whole_read_bytes as u64);
        if maximum == 0
            || maximum > self.limits.maximum_whole_read_bytes
            || u64::try_from(maximum).map_or(true, |maximum| maximum > grant)
        {
            return Err(ExecutionError::resource(
                "object_read_limit",
                "object whole-read maximum is zero or exceeds its exact grant",
            ));
        }
        Ok(maximum)
    }
}

impl CapabilityAdapter for ObjectStorageAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        match policy.operation.as_str() {
            "put-new" => self.put_new(policy, arguments),
            "get" => self.get(policy, arguments),
            "range" => self.range(policy, arguments),
            "head" => self.head(policy, arguments),
            "reconcile-put" => self.reconcile_put(policy, arguments),
            "delete" => self.delete(policy, arguments),
            operation => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "object_operation_unknown",
                format!("object adapter does not implement '{operation}'"),
            )),
        }
    }
}

fn validate_key(key: &str) -> Result<(), ExecutionError> {
    if key.is_empty()
        || key.len() > MAXIMUM_OBJECT_KEY_BYTES
        || key.starts_with('/')
        || key.starts_with("__lkjscript_")
        || key.contains('\0')
        || key.contains('\\')
        || key
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(object_argument(
            "object key is empty, excessive, reserved, or noncanonical",
        ));
    }
    Ok(())
}

fn validate_prefix(prefix: &str) -> Result<(), Diagnostic> {
    if prefix.is_empty() {
        return Ok(());
    }
    validate_key(prefix)
        .map_err(|_| object_diagnostic("object_prefix", "object deployment prefix is noncanonical"))
}

fn validate_content_type(value: &str) -> Result<(), ExecutionError> {
    if value.is_empty()
        || value.len() > 255
        || !value
            .bytes()
            .all(|byte| byte == b' ' || (0x21..=0x7e).contains(&byte))
        || !value.contains('/')
    {
        return Err(object_argument("object content type is invalid"));
    }
    Ok(())
}

fn validate_deployment_token(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty()
        || value.len() > 255
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(object_diagnostic(
            "object_s3_token",
            format!("{label} is not a canonical deployment token"),
        ));
    }
    Ok(())
}

fn text<'a>(value: &'a Value, label: &str) -> Result<&'a str, ExecutionError> {
    let Value::Text(value) = value else {
        return Err(object_argument(format!("{label} must be Text")));
    };
    Ok(value)
}

fn static_text<'a>(value: &'a Value, label: &str) -> Result<&'a str, ExecutionError> {
    let Value::StaticText(value) = value else {
        return Err(object_argument(format!("{label} must be StaticText")));
    };
    Ok(value)
}

fn nonnegative_u64(value: &Value, label: &str) -> Result<u64, ExecutionError> {
    let Value::I64(value) = value else {
        return Err(object_argument(format!("{label} must be I64")));
    };
    u64::try_from(*value).map_err(|_| object_argument(format!("{label} must be non-negative")))
}

fn nonnegative_usize(value: &Value, label: &str) -> Result<usize, ExecutionError> {
    let Value::I64(value) = value else {
        return Err(object_argument(format!("{label} must be I64")));
    };
    usize::try_from(*value).map_err(|_| object_argument(format!("{label} must be non-negative")))
}

fn map_object_error(
    error: object_store::Error,
    policy: &CallPolicy,
    visibility_possible: bool,
) -> ExecutionError {
    match error {
        object_store::Error::NotFound { .. } => ExecutionError::new(
            ExecutionFailureClass::Capability,
            "object_absent",
            "object does not exist",
        ),
        object_store::Error::AlreadyExists { .. } | object_store::Error::Precondition { .. } => {
            ExecutionError::new(
                ExecutionFailureClass::Capability,
                "object_conflict",
                "object publication precondition failed",
            )
        }
        object_store::Error::InvalidPath { .. } => object_argument("object path is invalid"),
        _ if visibility_possible || policy.visibility == super::language::Visibility::Possible => {
            ExecutionError::new(
                ExecutionFailureClass::PossibleVisibility,
                "object_visibility_unknown",
                "object operation failed after publication may have become visible",
            )
        }
        _ => {
            let mut result = ExecutionError::new(
                ExecutionFailureClass::Capability,
                "object_provider",
                "object-storage provider operation failed",
            );
            result.retryable = true;
            result
        }
    }
}

fn object_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "object_adapter_argument",
        message,
    )
}

fn object_internal(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "object_adapter_internal",
        message,
    )
}

fn object_diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectAdapterIdentity {
    pub contract_version: u16,
    pub adapter_kind: String,
    pub prefix: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PackageId;
    use crate::platform::execution::ExecutionControl;
    use crate::platform::language::{Idempotency, Visibility};
    use crate::platform::stream::StreamLimits;
    use std::collections::BTreeMap;

    fn owner() -> OwnerId {
        OwnerId::deterministic_for_test(
            PackageId::parse("1234567890abcdef1234567890abcdef").expect("package id"),
            "object",
            "ObjectStorage",
        )
    }

    fn policy(operation: &str, possible: bool) -> CallPolicy {
        CallPolicy {
            requirement: "objects".to_owned(),
            interface: owner(),
            operation: operation.to_owned(),
            idempotency: Idempotency::IdempotentWithKey,
            visibility: if possible {
                Visibility::Possible
            } else {
                Visibility::None
            },
            limits: BTreeMap::from([("maximum_read_bytes".to_owned(), 1024)]),
            control: ExecutionControl::uncancelled(),
        }
    }

    #[test]
    fn in_memory_conformance_streams_put_get_range_head_and_delete() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime");
        let streams = StreamRegistry::new(StreamLimits {
            maximum_chunk_bytes: 3,
            maximum_buffered_chunks: 2,
            maximum_total_bytes: 1024,
            maximum_live_streams: 8,
        })
        .expect("streams");
        let adapter = ObjectStorageAdapter::in_memory(
            owner(),
            runtime.handle().clone(),
            streams.clone(),
            "tests".to_owned(),
            ObjectLimits {
                maximum_object_bytes: 1024,
                maximum_whole_read_bytes: 1024,
            },
        )
        .expect("adapter");
        let lease = streams
            .register_memory(b"streamed object".to_vec())
            .expect("stream");
        let receipt = adapter
            .call(
                &policy("put-new", true),
                vec![
                    Value::text("objects/one"),
                    lease.value(),
                    Value::static_text("text/plain"),
                ],
            )
            .expect("put");
        assert_eq!(
            receipt.field("size").and_then(|value| match value {
                Value::I64(value) => Some(*value),
                _ => None,
            }),
            Some(15)
        );
        let bytes = adapter
            .call(
                &policy("get", false),
                vec![Value::text("objects/one"), Value::I64(1024)],
            )
            .expect("get");
        assert!(matches!(bytes, Value::Bytes(value) if value.as_ref() == b"streamed object"));
        let range = adapter
            .call(
                &policy("range", false),
                vec![Value::text("objects/one"), Value::I64(9), Value::I64(6)],
            )
            .expect("range");
        assert!(matches!(range, Value::Bytes(value) if value.as_ref() == b"object"));
        assert!(
            adapter
                .call(&policy("head", false), vec![Value::text("objects/one")])
                .is_ok()
        );
        let reconciled = adapter
            .call(
                &policy("reconcile-put", false),
                vec![Value::text("objects/one")],
            )
            .expect("reconcile visible publication");
        assert!(matches!(
            reconciled,
            Value::List(items)
                if items.len() == 1
                    && matches!(items[0].field("blake3"), Some(Value::Text(value)) if value.len() == 64)
        ));
        assert!(
            adapter
                .call(&policy("delete", true), vec![Value::text("objects/one")])
                .is_ok()
        );
        let absent = adapter
            .call(
                &policy("get", false),
                vec![Value::text("objects/one"), Value::I64(1024)],
            )
            .expect_err("absent");
        assert_eq!(absent.code, "object_absent");
        let reconciled = adapter
            .call(
                &policy("reconcile-put", false),
                vec![Value::text("objects/one")],
            )
            .expect("reconcile absent publication");
        assert!(matches!(reconciled, Value::List(items) if items.is_empty()));
    }

    #[test]
    fn local_conformance_creates_nested_prefixes() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime");
        let temporary = tempfile::TempDir::new().expect("temporary object root");
        let streams = StreamRegistry::new(StreamLimits::default()).expect("streams");
        let adapter = ObjectStorageAdapter::local(
            owner(),
            runtime.handle().clone(),
            streams.clone(),
            temporary.path(),
            "lkjournal".to_owned(),
            ObjectLimits::default(),
        )
        .expect("local adapter");
        let lease = streams
            .register_memory(vec![0x5a; 200_000])
            .expect("stream");
        let receipt = adapter
            .call(
                &policy("put-new", true),
                vec![
                    Value::text("operator/attachment.bin"),
                    lease.value(),
                    Value::static_text("application/octet-stream"),
                ],
            )
            .expect("local put");
        assert!(matches!(receipt.field("size"), Some(Value::I64(200_000))));
    }

    #[test]
    fn keys_credentials_and_limits_are_closed() {
        assert!(validate_key("../escape").is_err());
        assert!(validate_key("__lkjscript_private").is_err());
        let credentials = S3Credentials::new("access", "private").expect("credentials");
        assert_eq!(format!("{credentials:?}"), "S3Credentials(<redacted>)");
        assert!(
            ObjectLimits {
                maximum_object_bytes: 0,
                maximum_whole_read_bytes: 1,
            }
            .validate()
            .is_err()
        );
    }
}
