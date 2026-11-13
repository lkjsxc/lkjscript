use std::rc::Rc;
fn unavailable(value: Rc<i64>) -> i64 { *value }
fn wrong_type() -> bool { 42i64 }
fn missing_method() -> i64 { 42i64.absent_method() }
fn borrow_error() -> i64 {
    let mut values = vec![42i64];
    let item = &values[0];
    values.push(1);
    *item
}
