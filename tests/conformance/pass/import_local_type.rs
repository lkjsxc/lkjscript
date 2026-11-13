struct T {n:i64} fn read<T>(value:T)->i64 { use self::T; let current=T{n:42}; current.n } pub fn probe()->i64{read(1)}
