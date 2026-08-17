use super::call_frame::CallFrame;

pub const MAX_FRAMES: usize = 1024;

pub struct FrameStack {
    pub frames: Vec<CallFrame>,
}

impl FrameStack {
    pub fn new() -> Self {
        Self {
            frames: Vec::with_capacity(MAX_FRAMES),
        }
    }

    #[inline]
    pub fn push(&mut self, frame: CallFrame) {
        if self.frames.len() >= MAX_FRAMES {
            panic!("Stack overflow: too many call frames");
        }
        self.frames.push(frame);
    }

    #[inline]
    pub fn pop(&mut self) -> Option<CallFrame> {
        self.frames.pop()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.frames.len()
    }
}
