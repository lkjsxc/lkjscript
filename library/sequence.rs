//! Lexicographic vector order is ordinary source, shared by every engine.
pub fn __lkj_vec_compare<T: Ord>(left: &Vec<T>, right: &Vec<T>) -> i32 {
    let mut i: usize = 0;
    while i < left.len() && i < right.len() {
        if left[i] < right[i] {
            return -1;
        }
        if left[i] > right[i] {
            return 1;
        }
        i = i + 1;
    }
    if left.len() < right.len() {
        return -1;
    }
    if left.len() > right.len() {
        return 1;
    }
    0
}
