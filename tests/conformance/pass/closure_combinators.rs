pub fn probe()->i64{
 let some:Option<i64>=Some(41);let none:Option<i64>=None;
 assert!(some.map(|x:i64|x+1).unwrap()==42);
 assert!(none.map(|x:i64|x+1).is_none());
 assert!(some.map_or(0i64,|x:i64|x+1)==42);
 assert!(none.map_or(42i64,|x:i64|x+1)==42);
 assert!(some.map_or_else(||0i64,|x:i64|x+1)==42);
 assert!(none.map_or_else(||42i64,|x:i64|x+1)==42);
 assert!(some.and_then(|x:i64|Some(x+1)).unwrap()==42);
 assert!(none.and_then(|x:i64|Some(x+1)).is_none());
 assert!(none.or_else(||Some(42i64)).unwrap()==42);
 assert!(some.or_else(|| -> Option<i64> {None}).unwrap()==41);
 assert!(none.unwrap_or_else(||42i64)==42);
 assert!(some.unwrap_or_else(||0i64)==41);
 assert!(some.filter(|x:&i64|*x>0).unwrap()==41);
 assert!(some.filter(|x:&i64|*x<0).is_none());
 let okay:Result<i64,i64>=Ok(41);let error:Result<i64,i64>=Err(41);
 assert!(okay.map(|x:i64|x+1).unwrap()==42);
 assert!(error.map(|x:i64|x+1).unwrap_err()==41);
 assert!(error.map_err(|x:i64|x+1).unwrap_err()==42);
 assert!(okay.map_err(|x:i64|x+1).unwrap()==41);
 assert!(okay.map_or(0i64,|x:i64|x+1)==42);
 assert!(error.map_or(42i64,|x:i64|x+1)==42);
 assert!(okay.map_or_else(|x:i64|x-1,|x:i64|x+1)==42);
 assert!(error.map_or_else(|x:i64|x+1,|x:i64|x-1)==42);
 assert!(okay.and_then(|x:i64| -> Result<i64,i64> {Ok(x+1)}).unwrap()==42);
 assert!(error.and_then(|x:i64| -> Result<i64,i64> {Ok(x+1)}).unwrap_err()==41);
 let recovered:Result<i64,i64>=error.or_else(|x:i64| -> Result<i64,i64> {Ok(x+1)});assert!(recovered.unwrap()==42);
 let unchanged:Result<i64,i64>=okay.or_else(|x:i64| -> Result<i64,i64> {Ok(x+1)});assert!(unchanged.unwrap()==41);
 assert!(error.unwrap_or_else(|x:i64|x+1)==42);
 assert!(okay.unwrap_or_else(|x:i64|x+1)==41);42
}
