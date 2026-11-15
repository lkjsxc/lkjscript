// expect: reject E_UNSUPPORTED
// pending: Suggest `text.as_str().split('\n')` when split exists on str but the receiver is String.
pub fn probe() -> usize { "a\nb".to_string().split('\n').count() }
