use crate::vm::value::Value;

pub const MAX_STACK: usize = 256 * 1024;

pub struct ValueStack {
    pub stack: Vec<Value>,
    pub sp: usize,
}

impl ValueStack {
    pub fn new() -> Self {
        Self {
            stack: vec![Value::from_bool(false); MAX_STACK],
            sp: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, val: Value) {
        if self.sp >= MAX_STACK {
            panic!("Stack overflow");
        }
        self.stack[self.sp] = val;
        self.sp += 1;
    }

    #[inline]
    pub fn pop(&mut self) -> Value {
        if self.sp == 0 {
            panic!("Stack underflow");
        }
        self.sp -= 1;
        self.stack[self.sp]
    }

    #[inline]
    pub fn peek(&self, distance: usize) -> Value {
        self.stack[self.sp - 1 - distance]
    }

    #[inline]
    pub fn get(&self, idx: usize) -> Value {
        self.stack[idx]
    }

    #[inline]
    pub fn set(&mut self, idx: usize, val: Value) {
        self.stack[idx] = val;
    }
}

impl Drop for ValueStack {
    fn drop(&mut self) {
        for i in 0..self.sp {
            unsafe { self.stack[i].dec_ref(); }
        }
    }
}
