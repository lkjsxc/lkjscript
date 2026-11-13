enum E { A(Box<i64>), B(Box<i64>) }
fn uniform()->i64{let p=(true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,Box::new(42i64));let x=p.32;*x}
fn selected()->i64{let p=(E::A(Box::new(42i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)),E::A(Box::new(0i64)),E::B(Box::new(0i64)));
let x0=match p.0{E::A(value)=>value,E::B(value)=>value};
let x1=match p.1{E::A(value)=>value,E::B(value)=>value};
let x2=match p.2{E::A(value)=>value,E::B(value)=>value};
let x3=match p.3{E::A(value)=>value,E::B(value)=>value};
let x4=match p.4{E::A(value)=>value,E::B(value)=>value};
let x5=match p.5{E::A(value)=>value,E::B(value)=>value};
let x6=match p.6{E::A(value)=>value,E::B(value)=>value};
let x7=match p.7{E::A(value)=>value,E::B(value)=>value};
let x8=match p.8{E::A(value)=>value,E::B(value)=>value};
let x9=match p.9{E::A(value)=>value,E::B(value)=>value};
let x10=match p.10{E::A(value)=>value,E::B(value)=>value};
let x11=match p.11{E::A(value)=>value,E::B(value)=>value};
let x12=match p.12{E::A(value)=>value,E::B(value)=>value};
let x13=match p.13{E::A(value)=>value,E::B(value)=>value};
let x14=match p.14{E::A(value)=>value,E::B(value)=>value};
let x15=match p.15{E::A(value)=>value,E::B(value)=>value};
let x16=match p.16{E::A(value)=>value,E::B(value)=>value};
let x17=match p.17{E::A(value)=>value,E::B(value)=>value};
let x18=match p.18{E::A(value)=>value,E::B(value)=>value};
let x19=match p.19{E::A(value)=>value,E::B(value)=>value};
let x20=match p.20{E::A(value)=>value,E::B(value)=>value};
let x21=match p.21{E::A(value)=>value,E::B(value)=>value};
let x22=match p.22{E::A(value)=>value,E::B(value)=>value};
let x23=match p.23{E::A(value)=>value,E::B(value)=>value};
let x24=match p.24{E::A(value)=>value,E::B(value)=>value};
let x25=match p.25{E::A(value)=>value,E::B(value)=>value};
let x26=match p.26{E::A(value)=>value,E::B(value)=>value};
let x27=match p.27{E::A(value)=>value,E::B(value)=>value};
let x28=match p.28{E::A(value)=>value,E::B(value)=>value};
let x29=match p.29{E::A(value)=>value,E::B(value)=>value};
let x30=match p.30{E::A(value)=>value,E::B(value)=>value};
let x31=match p.31{E::A(value)=>value,E::B(value)=>value};
*x0}
pub fn probe()->i64{uniform()+selected()}
