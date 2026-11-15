// expect: reject E_UNSUPPORTED
// pending: Name the unsupported local const declaration and suggest moving it to module scope.
pub fn probe() -> i64 { const VALUE: i64 = 7; VALUE }
