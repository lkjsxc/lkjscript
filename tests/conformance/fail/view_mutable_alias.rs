struct View<'a> { value: &'a mut i64 }
fn change(view: &mut View<'_>) { *view.value = 0; }
pub fn probe() -> i64 {
    let mut value: i64 = 42;
    let mut view = View { value: &mut value };
    let borrowed = &*view.value;
    change(&mut view);
    *borrowed
}
