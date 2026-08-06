use super::*;

/// One generic runtime-value dispatch site whose arguments and result are copied only
/// through verified generated-frame homes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeapRuntimeSite {
    pub(super) id: u64,
    pub(super) function: FunctionId,
    pub(super) descriptor: HeapCallDescriptor,
    pub(super) arguments: Vec<FrameHome>,
    pub(super) result: FrameHome,
    pub(super) source: Option<SourceOrigin>,
}

impl HeapRuntimeSite {
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn descriptor(&self) -> &HeapCallDescriptor {
        &self.descriptor
    }

    #[must_use]
    pub fn arguments(&self) -> &[FrameHome] {
        &self.arguments
    }

    #[must_use]
    pub const fn result(&self) -> FrameHome {
        self.result
    }

    #[must_use]
    pub const fn source(&self) -> Option<SourceOrigin> {
        self.source
    }
}

pub(super) fn verify_heap_sites(
    image: &InstallableImage,
    runtime_calls: &std::collections::HashSet<RuntimeCallSlot>,
) -> Result<(), ImageIntegrityError> {
    for (expected_id, site) in image.heap_runtime_sites.iter().enumerate() {
        let frame = image
            .frames
            .iter()
            .find(|frame| frame.function == site.function)
            .ok_or(ImageIntegrityError::HeapRuntimeSite)?;
        if usize::try_from(site.id).ok() != Some(expected_id)
            || site.arguments.len() != site.descriptor.input_types().len()
            || site
                .arguments
                .iter()
                .zip(site.descriptor.input_types())
                .any(|(home, expected)| home.value_type != *expected || !frame.homes.contains(home))
            || site.result.value_type != site.descriptor.result_type()
            || !frame.homes.contains(&site.result)
            || !site.descriptor.canonical_facts_are_valid()
        {
            return Err(ImageIntegrityError::HeapRuntimeSite);
        }
    }
    if image.heap_runtime_sites.is_empty() == runtime_calls.contains(&RuntimeCallSlot::HeapDispatch)
    {
        return Err(ImageIntegrityError::HeapRuntimeSite);
    }
    Ok(())
}
