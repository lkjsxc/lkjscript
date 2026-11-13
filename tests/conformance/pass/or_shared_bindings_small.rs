enum E { A(Box<i64>), B(Box<i64>) }
fn uniform()->i64{let p=(true,true,true,true,true,Box::new(42i64));match p{(true|false,true|false,true|false,true|false,true|false,x)=>*x}}
fn selected()->i64{let (E::A(x0)|E::B(x0),E::A(x1)|E::B(x1),E::A(x2)|E::B(x2),E::A(x3)|E::B(x3),E::A(x4)|E::B(x4))=(E::A(Box::new(42i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)));*x0}
pub fn probe()->i64{uniform()+selected()}
