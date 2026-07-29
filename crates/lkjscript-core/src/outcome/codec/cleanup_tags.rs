fn phase_tag(phase: CleanupPhase) -> u8 {
    match phase {
        CleanupPhase::Ordinary => 0,
        CleanupPhase::Emergency => 1,
        CleanupPhase::RuntimeTeardown => 2,
    }
}

fn decode_phase(tag: u8) -> Result<CleanupPhase> {
    Ok(match tag {
        0 => CleanupPhase::Ordinary,
        1 => CleanupPhase::Emergency,
        2 => CleanupPhase::RuntimeTeardown,
        _ => return Err(Error::msg("unknown cleanup phase tag")),
    })
}

fn encode_subject(out: &mut Encoder, subject: CleanupSubject) -> Result<()> {
    match subject {
        CleanupSubject::UniqueStorage => out.u8(0),
        CleanupSubject::Resource(kind) => {
            out.u8(1)?;
            out.u8(kind as u8)
        }
        CleanupSubject::ResourceTable => out.u8(2),
        CleanupSubject::BorrowedResource(kind) => {
            out.u8(3)?;
            out.u8(kind as u8)
        }
        CleanupSubject::Terminal => out.u8(4),
        CleanupSubject::StandardOutput => out.u8(5),
        CleanupSubject::EvaluatorProvider => out.u8(6),
    }
}

fn decode_subject(input: &mut Decoder<'_>) -> Result<CleanupSubject> {
    Ok(match input.u8()? {
        0 => CleanupSubject::UniqueStorage,
        1 => CleanupSubject::Resource(resource_kind_subject(input.u8()?)?),
        2 => CleanupSubject::ResourceTable,
        3 => CleanupSubject::BorrowedResource(resource_kind_subject(input.u8()?)?),
        4 => CleanupSubject::Terminal,
        5 => CleanupSubject::StandardOutput,
        6 => CleanupSubject::EvaluatorProvider,
        _ => return Err(Error::msg("unknown cleanup subject tag")),
    })
}

fn resource_kind_subject(tag: u8) -> Result<crate::ResourceKind> {
    crate::ResourceKind::from_tag(tag).ok_or_else(|| Error::msg("unknown cleanup resource tag"))
}
