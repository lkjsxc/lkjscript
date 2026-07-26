pub(crate) const MAX_PATH_BYTES: usize = 4095;

pub(crate) fn validate(path: &[u8]) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH_BYTES
        && path.first() == Some(&b'/')
        && !path.contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_exact_absolute_linux_path_bounds() {
        let mut maximum = vec![b'x'; MAX_PATH_BYTES];
        maximum[0] = b'/';
        assert!(validate(&maximum));
        maximum.push(b'x');
        assert!(!validate(&maximum));
        assert!(!validate(b""));
        assert!(!validate(b"relative"));
        assert!(!validate(b"/nul\0byte"));
    }
}
