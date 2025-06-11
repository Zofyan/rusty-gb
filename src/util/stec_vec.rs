use std::sync::Arc;

pub struct StecVec<A> {
    pub data: [Option<A>; 10],
    pub(crate) length: usize
}

impl<A: Copy> StecVec<A> {
    pub fn new() -> StecVec<A> {
        StecVec {
            data: [None; 10],
            length: 0
        }
    }

    pub fn push(&mut self, new: Option<A>) {
        if self.length == 10 {
            panic!("Too long");
        }
        self.data[self.length] = new;
        self.length += 1;
    }

    pub fn clear(&mut self) {
        for i in 0..10 {
            self.data[i] = None;
        }
        self.length = 0;
    }
}

