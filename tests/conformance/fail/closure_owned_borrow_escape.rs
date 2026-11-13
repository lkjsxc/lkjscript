pub fn probe()->i64 {let owned=Box::new(42i64);let f=move||&*owned;let r=f();*r}
