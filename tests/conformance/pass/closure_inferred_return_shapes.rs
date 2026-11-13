// Inferred result shape must not turn moved capture values into reborrows.
fn option_shared(yes:bool)->i64{let value=42i64; let reference=&value; let once=move || if yes { None } else { Some(reference) }; match once(){Some(x)=>*x,None=>42}}
fn option_mutable(yes:bool)->i64{let mut value=42i64; let reference=&mut value; let once=move || if yes { None } else { Some(reference) }; match once(){Some(x)=>*x,None=>42}}
fn if_mutable(yes:bool)->i64{let mut a=42i64; let mut b=42i64; let first=&mut a; let second=&mut b; let once=move || if yes { first } else { second }; *once()}
fn match_mutable(yes:bool)->i64{let mut a=42i64; let mut b=42i64; let first=&mut a; let second=&mut b; let once=move || match yes { true=>first,false=>second }; *once()}
fn option_match(yes:bool)->i64{let mut value=42i64; let reference=&mut value; let once=move || match yes {true=>None,false=>Some(reference)}; match once(){Some(x)=>*x,None=>42}}
fn option_tail(yes:bool)->i64{let mut value=42i64; let reference=&mut value; let once=move || Some(reference); match once(){Some(x)=>*x,None=>42}}
fn pair(yes:bool)->i64{let mut value=42i64; let reference=&mut value; let once=move || (None::<i64>,Some(reference)); match once().1{Some(x)=>*x,None=>42}}
fn explicit_return_option(yes:bool)->i64{let mut value=42i64; let reference=&mut value; let once=move || { if yes { return None; } Some(reference) }; match once(){Some(x)=>*x,None=>42}}
fn loop_value(yes: bool) -> i64 {
    let mut a = 42i64;
    let mut b = 42i64;
    let first = &mut a;
    let second = &mut b;
    let once = move || loop {
        if yes { break Some(first); }
        break Some(second);
    };
    match once() { Some(value) => *value, None => 0 }
}
fn generic<T>(value: T) -> T { value }
fn generic_tail() -> i64 {
    let mut value = 42i64;
    let reference = &mut value;
    let once = move || generic(reference);
    *once()
}
struct Updated<T> { value: T, tag: i64 }
fn struct_update_field_move() -> i64 {
    let mut first = 42i64;
    let mut second = 0i64;
    let reference = &mut first;
    let base = Updated { value: &mut second, tag: 1 };
    let once = move || Updated { value: reference, ..base };
    let result = once();
    assert_eq!(result.tag, 1);
    *result.value
}
pub fn probe() -> i64 {
    assert_eq!(option_shared(true), 42);
    assert_eq!(option_shared(false), 42);
    assert_eq!(option_mutable(true), 42);
    assert_eq!(option_mutable(false), 42);
    assert_eq!(if_mutable(true), 42);
    assert_eq!(if_mutable(false), 42);
    assert_eq!(match_mutable(true), 42);
    assert_eq!(match_mutable(false), 42);
    assert_eq!(option_match(true), 42);
    assert_eq!(option_match(false), 42);
    assert_eq!(option_tail(true), 42);
    assert_eq!(option_tail(false), 42);
    assert_eq!(pair(true), 42);
    assert_eq!(pair(false), 42);
    assert_eq!(explicit_return_option(true), 42);
    assert_eq!(explicit_return_option(false), 42);
    assert_eq!(loop_value(true), 42);
    assert_eq!(loop_value(false), 42);
    assert_eq!(struct_update_field_move(), 42);
    generic_tail()
}
