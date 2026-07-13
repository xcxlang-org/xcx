use crate::vm::opcode::OpCode;
use std::collections::HashMap;

pub struct LivenessAnalysis {
    pub last_uses: HashMap<usize, Vec<u8>>,
    pub virtual_last_use: HashMap<u8, usize>,
}

impl LivenessAnalysis {
    pub fn analyze(bytecode: &[OpCode]) -> Self {
        let mut last_uses: HashMap<usize, Vec<u8>> = HashMap::new();
        let mut virtual_last_use: HashMap<u8, usize> = HashMap::new();

        let loops = crate::vm::opcode::collect_backedges(bytecode);

        let mut last_use_found = std::collections::HashSet::new();

        let mut registers = Vec::new();
        for i in (0..bytecode.len()).rev() {
            let op = &bytecode[i];
            registers.clear();
            Self::collect_registers(op, &mut registers);

            let mut last_idx = i;
            
            for (start, end) in &loops {
                if i >= *start && i <= *end {
                    if *end > last_idx {
                        last_idx = *end;
                    }
                }
            }

            for &reg in &registers {

                if !last_use_found.contains(&reg) {
                    last_use_found.insert(reg);
                    virtual_last_use.insert(reg, last_idx);
                    last_uses.entry(last_idx).or_default().push(reg);
                } else {
                    if let Some(prev_last) = virtual_last_use.get_mut(&reg) {
                        if last_idx > *prev_last {
                            if let Some(list) = last_uses.get_mut(prev_last) {
                                list.retain(|&r| r != reg);
                            }
                            *prev_last = last_idx;
                            last_uses.entry(last_idx).or_default().push(reg);
                        }
                    }
                }
            }
        }

        Self {
            last_uses,
            virtual_last_use,
        }
    }

    pub fn collect_registers(op: &OpCode, regs: &mut Vec<u8>) {
        op.src_regs(regs);
    }

    pub fn get_dst_reg(op: &OpCode) -> Option<u8> {
        op.dst_reg()
    }
}
