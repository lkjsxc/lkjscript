use super::*;

impl NativeCallState<'_> {
    pub(in crate::executable) fn materialize_heap_arguments(
        &mut self,
        site: &HeapRuntimeSite,
        facts: &lkjscript_native::FrameFacts,
        frame: ActiveFrame,
    ) -> bool {
        self.heap_arguments.clear();
        for home in site.arguments() {
            if !facts.homes().contains(home) {
                return false;
            }
            // SAFETY: image integrity and the active descriptor bind this
            // aligned home to the currently registered generated frame.
            let address = unsafe {
                frame
                    .rbp
                    .offset(home.rbp_displacement() as isize)
                    .cast::<u64>()
            };
            // SAFETY: each verified argument home is initialized at this site.
            let word = unsafe { address.read() };
            let value = match home.value_type() {
                ValueType::I64 => NativeValue::I64(word as i64),
                ValueType::F64 => NativeValue::F64Bits(word),
                ValueType::Bool if word <= 1 => NativeValue::Bool(word == 1),
                ValueType::Bool => return false,
                ValueType::Unit if word == 0 => NativeValue::Unit,
                ValueType::Unit => return false,
                ValueType::Reference(reference_type) => {
                    NativeValue::Reference(NativeReference::new(reference_type, word))
                }
                _ => return false,
            };
            self.heap_arguments.push(value);
        }
        true
    }
}
