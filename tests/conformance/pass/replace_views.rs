struct View<'a> { value: &'a mut i64 }
struct Data { left: i64, right: i64 }
pub fn probe() -> i64 {
    let mut data = Data { left: 7, right: 35 };
    let mut view = View { value: &mut data.left };
    let old = std::mem::replace(&mut view, View { value: &mut data.right });
    *old.value = 11;
    *view.value = 31;
    *old.value + *view.value
}
