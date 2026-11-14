// expect: reject E_TYPE
// witness: {}
type A<T>=B<T>; type B<T>=A<T>; fn probe()->i64 {0}
