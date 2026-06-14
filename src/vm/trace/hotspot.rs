use std::collections::{HashSet, HashMap};

pub struct Hotspot {
    pub counts: Vec<u32>,
    pub threshold: u32,
    pub blacklist: HashSet<usize>,
    pub guard_failures: HashMap<usize, u32>,
}

impl Hotspot {
    pub fn new() -> Self {
        Self {
            counts: Vec::new(),
            threshold: 50,
            blacklist: HashSet::new(),
            guard_failures: HashMap::new(),
        }
    }

    pub fn resize(&mut self, size: usize) {
        if size > self.counts.len() {
            self.counts.resize(size, 0);
        }
    }

    pub fn tick(&mut self, ip: usize) -> bool {
        if self.blacklist.contains(&ip) {
            return false;
        }
        if ip < self.counts.len() {
            self.counts[ip] += 1;
            return self.counts[ip] >= self.threshold;
        }
        false
    }

    pub fn reset(&mut self, ip: usize) {
        if ip < self.counts.len() {
            self.counts[ip] = 0;
        }
    }

    pub fn blacklist(&mut self, ip: usize) {
        self.blacklist.insert(ip);
    }

    pub fn on_guard_failure(&mut self, ip: usize) -> bool {
        let count = {
            let entry = self.guard_failures.entry(ip).or_insert(0);
            *entry += 1;
            *entry
        };
        self.reset(ip);
        if count >= 3 {
            self.blacklist(ip);
            return true;
        }
        false
    }
}
