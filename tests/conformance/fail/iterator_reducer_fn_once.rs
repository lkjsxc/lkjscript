pub fn probe()->i64{let values=vec![1i64];let capture=Box::new(2i64);let mut input=values.iter();input.any(move |_|{drop(capture);true});42}
