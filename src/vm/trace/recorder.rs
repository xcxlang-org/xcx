use crate::vm::trace::{Trace, TraceOp};
use parking_lot::RwLock;

pub struct Recorder {
    pub recording_trace: Option<RwLock<Trace>>,
    pub is_recording: bool,
    /// The IP at which the current trace started.
    pub start_ip: Option<usize>,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            recording_trace: None,
            is_recording: false,
            start_ip: None,
        }
    }

    pub fn start(&mut self, trace: Trace) {
        self.start_ip = Some(trace.start_ip);
        self.recording_trace = Some(RwLock::new(trace));
        self.is_recording = true;
    }

    pub fn stop(&mut self) -> Option<RwLock<Trace>> {
        self.is_recording = false;
        self.start_ip = None;
        self.recording_trace.take()
    }

    pub fn record(&mut self, op: TraceOp) {
        if let Some(ref lock) = self.recording_trace {
            lock.write().ops.push(op);
        }
    }
}
