enum List { End, Next(i64, Box<List>) }
        struct Pair { first: Box<List>, second: Box<List> }
        pub fn probe() -> i64 {
            let mut list = List::End;
            let mut i: i64 = 0;
            while i < 20000 {
                list = List::Next(i, Box::new(list));
                i = i + 1;
            }
            let pair = Pair { first: Box::new(list), second: Box::new(List::End) };
            let moved = pair.first;
            i
        }
