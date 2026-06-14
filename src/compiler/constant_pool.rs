use crate::vm::value::Value;
use crate::compiler::compiler::CompileContext;

impl<'a> CompileContext<'a> {
    pub fn add_constant(&mut self, val: Value) -> u32 {
        if val.is_string() {
            let s = val.as_string();
            if let Some(&idx) = self.string_constants.get(&s.data) {
                idx as u32
            } else {
                let idx = self.constants.len();
                self.string_constants.insert(s.data.clone(), idx);
                self.constants.push(val);
                idx as u32
            }
        } else if val.is_int() || val.is_float() {
            let key = (val.bits, val.tag);
            if let Some(&idx) = self.numeric_constants.get(&key) {
                idx as u32
            } else {
                let idx = self.constants.len();
                self.numeric_constants.insert(key, idx);
                self.constants.push(val);
                idx as u32
            }
        } else {
            self.constants.push(val);
            (self.constants.len() - 1) as u32
        }
    }
}
