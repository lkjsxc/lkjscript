use crate::*;

impl JitSession {
    pub(crate) fn trap_message(
        &self,
        function: FunctionId,
        trap: TrapCode,
        site: Option<u64>,
    ) -> String {
        match trap {
            TrapCode::I64Overflow => "checked I64 overflow".to_string(),
            TrapCode::DivisionByZero => "div: I64 division by zero".to_string(),
            TrapCode::Explicit => self
                .last_runtime_trap
                .clone()
                .or_else(|| {
                    function
                        .index()
                        .and_then(|function| self.functions.get(function))
                        .and_then(|record| record.code_object)
                        .and_then(|identity| {
                            self.objects
                                .iter()
                                .find(|object| object.identity == identity)
                        })
                        .and_then(|object| {
                            let site = site?;
                            object
                                .explicit_traps
                                .iter()
                                .find_map(|(candidate, message)| {
                                    (*candidate == site).then(|| message.clone())
                                })
                        })
                })
                .unwrap_or_else(|| "explicit SSA trap".to_string()),
        }
    }
}
