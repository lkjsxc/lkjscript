use crate::*;

impl JitSession {
    pub fn code_objects(&self) -> &[CodeObject] {
        &self.objects
    }

    pub(crate) fn function_for_prototype(&self, prototype: u32) -> Option<FunctionId> {
        self.links
            .as_ref()?
            .functions
            .iter()
            .find_map(|link| (link.prototype == Some(prototype)).then_some(link.function))
    }
}
