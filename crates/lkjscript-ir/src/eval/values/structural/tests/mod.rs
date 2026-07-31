use lkjscript_core::{SemanticPayload, StructuralEventKind};

use crate::tests::fixtures::{block_metadata_cleanup, core_traits, metadata, one_block_program};
use crate::*;

use super::*;

mod classification;
mod execution;
mod explicit;
mod support;

use support::*;
