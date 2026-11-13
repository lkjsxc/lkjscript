type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&i64" ref shared 3
type "str" str
type "&str" ref shared 8
type "Mixed" struct move "number" 7 "text" 9
fn "mixed" 1 10
return-leaf 0 field 0 exact 0
return-leaf 0 field 1 static
local "number" 7 let
local "text" 9 let
local "mixed" 10 let
@1 text 1 6f6b
@2 struct 2 0 1
@3 return 2
endfn
fn "probe" 0 3
local "number" 3 let
local "reference" 7 let
local "mixed" 10 let
local "result" 3 let
@4 int 0 42
@5 borrow 1 shared 0
@6 call 2 0 1
@7 read 3 copy 2 field 0 deref
@8 drop 2
@9 drop 0
@10 return 3
endfn
