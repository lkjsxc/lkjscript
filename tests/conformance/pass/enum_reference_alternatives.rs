enum Either<'a> { Left(&'a mut i64), Right(&'a mut i64) }
struct Nested<'a> { value:Either<'a> }
fn selected(value:Either<'_>)->i64 {
    match value { Either::Left(item)|Either::Right(item)=>*item }
}
fn nested(value:Nested<'_>)->i64 {
    match value { Nested { value:Either::Left(item) }=>*item, Nested { value:Either::Right(item) }=>*item }
}
pub fn probe()->i64 {
    let mut left=20i64; let mut right=22i64;
    selected(Either::Right(&mut left))+nested(Nested { value:Either::Left(&mut right) })
}
