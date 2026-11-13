struct Triple<T>(T, T, T);
enum Choice { Empty, Values(i64, i64, i64) }
fn ends<T>(value: (T,T,T)) -> (T,T) { let (first, .., last) = value; (first,last) }
pub fn probe() -> i64 {
    let (first, .., last) = (1i64, 99i64, 2i64, 3i64);
    assert!(first + last == 4);
    let Triple(.., tail) = Triple(5i64, 6i64, 7i64);
    assert!(tail == 7);
    let choice = Choice::Values(8, 99, 9);
    let n = match choice { Choice::Values(a, .., b) => a+b, Choice::Empty => 0 };
    assert!(n == 17);
    let (..) = ();
    let pair = ends((Box::new(20i64), Box::new(99i64), Box::new(22i64)));
    let (left, right) = pair;
    *left + *right
}
