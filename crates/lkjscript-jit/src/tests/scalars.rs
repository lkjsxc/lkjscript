use crate::scalar::owned_scalar;
use crate::*;

#[test]
fn detached_native_scalars_retain_exact_payload_without_snapshot_objects() {
    for value in [i64::MIN, -1, 0, 1, i64::MAX] {
        let owned = owned_scalar(NativeValue::I64(value)).expect("owned I64");
        assert_eq!(owned.as_i64(), Some(value));
        assert_eq!(owned.snapshot_object_count(), 0);
    }
    for bits in [
        0_u64,
        1_u64 << 63,
        0x7ff0_0000_0000_0000,
        0x7ff8_0000_0000_0042,
        0xfff8_dead_beef_cafe,
    ] {
        let owned = owned_scalar(NativeValue::F64Bits(bits)).expect("owned F64");
        assert_eq!(owned.as_f64_bits(), Some(bits));
        assert_eq!(owned.snapshot_object_count(), 0);
    }
}
