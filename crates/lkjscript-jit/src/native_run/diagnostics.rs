use crate::*;

impl NativeRun {
    pub(crate) fn trap_message(
        &self,
        _function: FunctionId,
        trap: TrapCode,
        site: Option<u64>,
    ) -> String {
        match trap {
            TrapCode::I64Overflow => "checked I64 overflow".to_string(),
            TrapCode::DivisionByZero => "div: I64 division by zero".to_string(),
            TrapCode::Explicit => {
                self.last_runtime_trap
                    .clone()
                    .or_else(|| {
                        let site = site?;
                        self.object.as_ref()?.explicit_traps.iter().find_map(
                            |(candidate, message)| (*candidate == site).then(|| message.clone()),
                        )
                    })
                    .unwrap_or_else(|| "explicit SSA trap".to_string())
            }
        }
    }
}
