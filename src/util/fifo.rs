use std::sync::Arc;

pub struct FIFO {
    pub data: [u8; 16],
    length: usize,
}

impl FIFO {
    pub fn new() -> FIFO {
        FIFO {
            data: [0; 16],
            length: 0
        }
    }

    pub fn push(&mut self, new: u8) {
        if self.length == 16 {
            panic!("Too long");
        }
        self.data[self.length] = new;
        self.length += 1;
    }

    pub fn pop(&mut self) -> u8 {
        if self.length == 0 {
            panic!("Already empty");
        }
        let tmp = self.data[0];
        self.data.rotate_left(1);
        self.length -= 1;
        tmp
    }

    pub fn clear(&mut self) {
        self.length = 0;
    }

    pub fn length(&mut self) -> usize {
        self.length
    }
}

