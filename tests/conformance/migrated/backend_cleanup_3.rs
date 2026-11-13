enum List { End, Next(Vec<List>) }
        pub fn probe() -> i64 {
            let mut list = List::End;
            let mut i: i64 = 0;
            while i < 20000 {
                let mut child: Vec<List> = Vec::new();
                child.push(list);
                list = List::Next(child);
                i = i + 1;
            }
            i
        }
