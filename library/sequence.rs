//! Vec cloning, equality and lexicographic order are ordinary source.
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

pub fn __lkj_vec_clone<T: Clone>(source: &Vec<T>) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    let mut index: usize = 0;
    while index < source.len() {
        result.push(source[index].clone());
        index = index + 1;
    }
    result
}

pub fn __lkj_vec_equal<T: PartialEq>(left: &Vec<T>, right: &Vec<T>) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut index: usize = 0;
    while index < left.len() {
        // Equality uses the element's eq contract; ne may be independently overridden.
        if !(left[index] == right[index]) {
            return false;
        }
        index = index + 1;
    }
    true
}
