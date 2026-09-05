//! Prepared execution over strict Graph 10 artifacts.

mod byte_stream;
mod capability;
mod codec;
mod configuration;
mod data;
mod data_codec;
mod data_codec_reference;
mod deployment;
mod http;
mod http_client;
mod object;
mod password;
mod prepare;
pub(crate) mod pure_tail_probe;
mod queue;
mod reference;
mod reference_schema;
pub(crate) use reference::{
    NormalizedReferenceBinding, NormalizedReferenceOwnerRead, NormalizedReferenceRead,
    NormalizedReferenceReadWork,
};
pub(crate) use reference_schema::NormalizedReferenceSchema;
mod resident;
mod resource;
mod runner;
mod secret;
mod security;
mod session;
mod value;
mod value_schema;
mod vm;
mod worker;

pub(crate) use capability::{
    CAPABILITY_GRANT_CONTRACT_VERSION, NormalizedGrantAuthorityRevision, NormalizedGrantLimit,
    NormalizedSharingDomain,
};
pub(crate) use deployment::{
    NormalizedAdapterDescriptor, NormalizedDeploymentGrant, NormalizedDeploymentResourcePolicy,
    NormalizedPreparedDeployment,
};
pub(crate) use http::NormalizedHttpApplication;
pub(crate) use prepare::NormalizedProgram;
pub(crate) use resident::NormalizedResidentDeployment;
pub(crate) use runner::{
    NormalizedCommandPolicy, NormalizedCommandReceipt, NormalizedTestReceipt, run_graph_tests,
    run_pure_command,
};
pub(crate) use session::NormalizedSessionApplication;
pub(crate) use vm::NormalizedRunPolicy;
pub(crate) use worker::NormalizedWorkerApplication;
#[cfg(test)]
mod lkjournal_tests;
#[cfg(test)]
pub(crate) mod tests;
