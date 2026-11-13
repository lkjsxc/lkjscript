use std::rc::Rc;
struct Mixed { missing: Rc<i64>, count: i64 }
fn type_error(value: &Mixed) -> bool { value.count }
fn method_error(value: &Mixed) -> i64 { value.count.absent() }
