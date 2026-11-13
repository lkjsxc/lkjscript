type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&i64" ref shared 3
type "Pair" struct move "left" 7 "right" 7
fn "capture" 2 8
return-leaf 0 field 0 exact 0
return-leaf 0 field 1 exact 1
local "left" 7 let
local "right" 7 let
local "pair" 8 let
@1 struct 2 0 1
@2 return 2
endfn
fn "identity" 1 8
return-leaf 0 field 0 exact 0 field 0
return-leaf 0 field 1 exact 0 field 1
local "value" 8 let
@3 return 0
endfn
fn "probe" 0 3
local "left" 3 let
local "right" 3 let
local "a" 7 let
local "b" 7 let
local "pair" 8 let
local "transported" 8 let
local "x" 3 let
local "y" 3 let
local "result" 3 let
@4 int 0 20
@5 int 1 22
@6 borrow 2 shared 0
@7 borrow 3 shared 1
@8 call 4 0 2 3
@9 call 5 1 4
@10 read 6 copy 5 field 0 deref
@11 read 7 copy 5 field 1 deref
@12 binary 8 "+" 6 7
@13 drop 5
@14 drop 1
@15 drop 0
@16 return 8
endfn
