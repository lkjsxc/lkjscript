use super::*;

#[test]
fn rename_requires_two_path_operands() {
    use lkjscript_contracts::CapabilityKind::FileSystem;
    let result = system_result(SsaType::Unit);
    let prefix = SsaType::Capability(FileSystem);
    assert_eq!(
        host::host_signature(
            RuntimeOp::SysRename,
            &[prefix.clone(), SsaType::Path, SsaType::Path],
            &result,
        ),
        Some(true)
    );
    assert_eq!(
        host::host_signature(
            RuntimeOp::SysRename,
            &[prefix, SsaType::Path, SsaType::Str],
            &result,
        ),
        Some(false)
    );
}
