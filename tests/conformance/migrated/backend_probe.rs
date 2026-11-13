struct Node { ready: i64, next: List }
        enum List { End, Next(Box<Node>) }
        impl Drop for Node {
            fn drop(&mut self) {
                assert_eq!(self.ready, 1);
                match &mut self.next {
                    List::End => {},
                    List::Next(child) => { child.ready = 1; }
                }
            }
        }
        fn work() -> i64 {
            let mut list = List::End;
            let mut i: i64 = 0;
            while i < 20000 {
                list = List::Next(Box::new(Node { ready: 0, next: list }));
                i = i + 1;
            }
            match &mut list {
                List::End => {},
                List::Next(head) => { head.ready = 1; }
            }
            i
        }
    
pub fn probe()->i64 { let mut i=0; while i<3 { assert_eq!(work(),20000); i+=1; } 20000 }
