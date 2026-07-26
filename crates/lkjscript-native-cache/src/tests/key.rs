use super::*;
use lkjscript_native::BackendLimits;

#[test]
fn key_separates_source_tier_root_policy_and_profile() {
    let root = Root::new();
    let base = context(&root);
    let baseline = artifact_key(
        &base,
        [5; 32],
        CacheTier::Baseline,
        0,
        [0; 32],
        BackendLimits::default(),
    )
    .expect("baseline");
    let mut changed_source = base.clone();
    changed_source.source_sha256 = [9; 32];
    let cases = [
        artifact_key(
            &changed_source,
            [5; 32],
            CacheTier::Baseline,
            0,
            [0; 32],
            BackendLimits::default(),
        ),
        artifact_key(
            &base,
            [6; 32],
            CacheTier::Baseline,
            0,
            [0; 32],
            BackendLimits::default(),
        ),
        artifact_key(
            &base,
            [5; 32],
            CacheTier::Optimizing,
            0,
            [0; 32],
            BackendLimits::default(),
        ),
        artifact_key(
            &base,
            [5; 32],
            CacheTier::Baseline,
            1,
            [0; 32],
            BackendLimits::default(),
        ),
        artifact_key(
            &base,
            [5; 32],
            CacheTier::Baseline,
            0,
            [1; 32],
            BackendLimits::default(),
        ),
    ];
    for case in cases {
        assert_ne!(baseline, case.expect("separated key"));
    }
}
