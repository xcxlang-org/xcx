use crate::vm::opcode::OpCode;
use std::collections::HashMap;

pub struct LivenessAnalysis {
    // Instruction index -> list of registers whose lifetime ends at this instruction.
    pub last_uses: HashMap<usize, Vec<u8>>,
    // Virtual register -> index of the last instruction that uses it.
    pub virtual_last_use: HashMap<u8, usize>,
}

impl LivenessAnalysis {
    pub fn analyze(bytecode: &[OpCode]) -> Self {
        let mut last_uses: HashMap<usize, Vec<u8>> = HashMap::new();
        let mut virtual_last_use: HashMap<u8, usize> = HashMap::new();

        // Pass 1: Identify loop back-edges
        let mut loops = Vec::new();
        for (i, op) in bytecode.iter().enumerate() {
            match op {
                OpCode::Jump { target } |
                OpCode::JumpIfFalse { target, .. } |
                OpCode::JumpIfTrue { target, .. } |
                OpCode::LoopNext { target, .. } |
                OpCode::LoopPrev { target, .. } |
                OpCode::IncLocalLoopNext { target, .. } |
                OpCode::DecLocalLoopPrev { target, .. } |
                OpCode::IncVarLoopNext { target, .. } |
                OpCode::DecVarLoopPrev { target, .. } |
                OpCode::ArrayLoopNext { target, .. } |
                OpCode::TableIter { target, .. } => {
                    if (*target as usize) < i {
                        loops.push((*target as usize, i));
                    }
                }
                _ => {}
            }
        }

        let mut last_use_found = std::collections::HashSet::new();

        for i in (0..bytecode.len()).rev() {
            let op = &bytecode[i];
            let mut registers = Vec::new();
            Self::collect_registers(op, &mut registers);

            let mut last_idx = i;
            
            // If this register is used inside a loop, its lifetime must extend to the end of the loop
            for (start, end) in &loops {
                if i >= *start && i <= *end {
                    if *end > last_idx {
                        last_idx = *end;
                    }
                }
            }

            for reg in registers {

                if !last_use_found.contains(&reg) {
                    last_use_found.insert(reg);
                    virtual_last_use.insert(reg, last_idx);
                    last_uses.entry(last_idx).or_default().push(reg);
                } else {
                    // Even if seen, if this use is part of a loop that ends later than current last_use, extend it
                    if let Some(prev_last) = virtual_last_use.get_mut(&reg) {
                        if last_idx > *prev_last {
                            // Remove from old last_uses
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
        match op {
            OpCode::Move { src, .. } => regs.push(*src),
            OpCode::Add { src1, src2, .. } |
            OpCode::Sub { src1, src2, .. } |
            OpCode::Mul { src1, src2, .. } |
            OpCode::Div { src1, src2, .. } |
            OpCode::Mod { src1, src2, .. } |
            OpCode::Pow { src1, src2, .. } |
            OpCode::Equal { src1, src2, .. } |
            OpCode::NotEqual { src1, src2, .. } |
            OpCode::Greater { src1, src2, .. } |
            OpCode::Less { src1, src2, .. } |
            OpCode::GreaterEqual { src1, src2, .. } |
            OpCode::LessEqual { src1, src2, .. } |
            OpCode::And { src1, src2, .. } |
            OpCode::Or { src1, src2, .. } |
            OpCode::Has { src1, src2, .. } |
            OpCode::SetUnion { src1, src2, .. } |
            OpCode::SetIntersection { src1, src2, .. } |
            OpCode::SetDifference { src1, src2, .. } |
            OpCode::SetSymDifference { src1, src2, .. } |
            OpCode::IntConcat { src1, src2, .. } => {
                regs.push(*src1);
                regs.push(*src2);
            }
            OpCode::Not { src, .. } |
            OpCode::JumpIfFalse { src, .. } |
            OpCode::JumpIfTrue { src, .. } |
            OpCode::Print { src, .. } |
            OpCode::HaltAlert { src, .. } |
            OpCode::HaltError { src, .. } |
            OpCode::HaltFatal { src, .. } |
            OpCode::Return { src, .. } |
            OpCode::Yield { src, .. } |
            OpCode::YieldWithTarget { src, .. } |
            OpCode::Wait { src, .. } |
            OpCode::CastInt { src, .. } |
            OpCode::CastFloat { src, .. } |
            OpCode::CastString { src, .. } |
            OpCode::CastBool { src, .. } |
            OpCode::Neg { src, .. } |
            OpCode::Typeof { src, .. } |
            OpCode::SetName { src, .. } |
            OpCode::SetVar { src, .. } |
            OpCode::RandomChoice { src, .. } |
            OpCode::JsonParse { src, .. } |
            OpCode::EnvGet { src, .. } |
            OpCode::TableCloneSkeleton { src, .. } |
            OpCode::TerminalWrite { src, .. } => {
                regs.push(*src);
            }
            
            OpCode::TerminalRun { cmd_src, .. } => { regs.push(*cmd_src); }
            OpCode::TerminalMove { x_src, y_src, .. } => { regs.push(*x_src); regs.push(*y_src); }

            OpCode::Call { base, arg_count, .. } |
            OpCode::FiberCreate { base, arg_count, .. } => {
                let start = *base as usize;
                let end = start + *arg_count as usize;
                for r in start..end {
                    regs.push(r as u8);
                }
            }
            OpCode::MethodCall { base, arg_count, .. } |
            OpCode::MethodCallNamed { base, arg_count, .. } |
            OpCode::MethodCallCustom { base, arg_count, .. } => {
                let start = *base as usize;
                let end = start + *arg_count as usize + 1;
                for r in start..end {
                    regs.push(r as u8);
                }
            }

            OpCode::ArrayInit { base, count, .. } |
            OpCode::SetInit { base, count, .. } => {
                let start = *base as usize;
                let end = start + *count as usize;
                for r in start..end {
                    regs.push(r as u8);
                }
            }
            OpCode::MapInit { base, count, .. } => {
                let start = *base as usize;
                let end = start + (*count as usize * 2);
                for r in start..end {
                    regs.push(r as u8);
                }
            }
            
            OpCode::TableInit { base, row_count, col_count, .. } => {
                let start = *base as usize;
                let end = start + (*row_count as usize * *col_count as usize);
                for r in start..end {
                    regs.push(r as u8);
                }
            }
            
            OpCode::TableInitRow { tbl_dst, base, col_count } => {
                regs.push(*tbl_dst);
                let start = *base as usize;
                let end = start + *col_count as usize;
                for r in start..end {
                    regs.push(r as u8);
                }
            }

            OpCode::SetRange { start, end, step, .. } |
            OpCode::RandomInt { min: start, max: end, step, .. } |
            OpCode::RandomFloat { min: start, max: end, step, .. } => { regs.push(*start); regs.push(*end); regs.push(*step); }
            
            OpCode::StoreWrite { base, .. } |
            OpCode::StoreRead { base, .. } |
            OpCode::StoreAppend { base, .. } |
            OpCode::StoreExists { base, .. } |
            OpCode::StoreDelete { base, .. } |
            OpCode::StoreList { base, .. } |
            OpCode::StoreIsDir { base, .. } |
            OpCode::StoreSize { base, .. } |
            OpCode::StoreMkdir { base, .. } |
            OpCode::StoreGlob { base, .. } |
            OpCode::StoreZip { base, .. } |
            OpCode::StoreUnzip { base, .. } => { regs.push(*base); }

            OpCode::JsonBind { json_src, path_src, .. } |
            OpCode::JsonBindLocal { json_src, path_src, .. } => { regs.push(*json_src); regs.push(*path_src); }

            OpCode::JsonInject { json_src, mapping_src, .. } |
            OpCode::JsonInjectLocal { json_src, mapping_src, .. } => { regs.push(*json_src); regs.push(*mapping_src); }

            OpCode::JsonFastGetPush { json_src, path_src, val_src } => { regs.push(*json_src); regs.push(*path_src); regs.push(*val_src); }

            OpCode::HttpCall { url_src, body_src, .. } => { regs.push(*url_src); regs.push(*body_src); }
            OpCode::HttpRequest { arg_src, .. } => { regs.push(*arg_src); }
            OpCode::HttpRespond { status_src, body_src, headers_src, .. } => { regs.push(*status_src); regs.push(*body_src); regs.push(*headers_src); }
            OpCode::HttpServe { port_src, host_src, workers_src, routes_src, .. } => { regs.push(*port_src); regs.push(*host_src); regs.push(*workers_src); regs.push(*routes_src); }
            
            OpCode::CryptoHash { pass_src, alg_src, .. } => { regs.push(*pass_src); regs.push(*alg_src); }
            OpCode::CryptoVerify { pass_src, hash_src, alg_src, .. } => { regs.push(*pass_src); regs.push(*hash_src); regs.push(*alg_src); }
            OpCode::CryptoToken { len_src, .. } => { regs.push(*len_src); }
            
            OpCode::IncLocal { reg } => { regs.push(*reg); }
            OpCode::LoopNext { reg, limit_reg, .. } |
            OpCode::LoopPrev { reg, limit_reg, .. } => { regs.push(*reg); regs.push(*limit_reg); }
            OpCode::IncLocalLoopNext { inc_reg, reg, limit_reg, .. } => { regs.push(*inc_reg); regs.push(*reg); regs.push(*limit_reg); }
            OpCode::DecLocalLoopPrev { dec_reg, reg, limit_reg, .. } => { regs.push(*dec_reg); regs.push(*reg); regs.push(*limit_reg); }
            OpCode::IncVarLoopNext { reg, limit_reg, .. } |
            OpCode::DecVarLoopPrev { reg, limit_reg, .. } => { regs.push(*reg); regs.push(*limit_reg); }
            OpCode::ArrayLoopNext { idx_reg, size_reg, .. } => { regs.push(*idx_reg); regs.push(*size_reg); }
            
            OpCode::DatabaseInit { engine_src, path_src, tables_base_reg, .. } => { regs.push(*engine_src); regs.push(*path_src); regs.push(*tables_base_reg); }

            OpCode::GetIndex { container, index, .. } => { regs.push(*container); regs.push(*index); }
            OpCode::SetIndex { container, index, src } => { regs.push(*container); regs.push(*index); regs.push(*src); }
            OpCode::GetMember { container, .. } => { regs.push(*container); }
            OpCode::SetMember { container, src, .. } => { regs.push(*container); regs.push(*src); }
            OpCode::RowGet { row_reg, .. } => { regs.push(*row_reg); }
            OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, .. } => { regs.push(*tbl_reg); regs.push(*idx_reg); regs.push(*row_reg); regs.push(*limit_reg); }
            OpCode::TablePushRow { tbl_reg, row_reg } => { regs.push(*tbl_reg); regs.push(*row_reg); }
            OpCode::MakeClosure { capture_start, capture_count, .. } => {
                let start = *capture_start as usize;
                let end = start + *capture_count as usize;
                for r in start..end {
                    regs.push(r as u8);
                }
            }
            _ => {}
        }
    }

    pub fn get_dst_reg(op: &OpCode) -> Option<u8> {
        match op {
            OpCode::Move { dst, .. } |
            OpCode::LoadConst { dst, .. } |
            OpCode::Add { dst, .. } |
            OpCode::Sub { dst, .. } |
            OpCode::Mul { dst, .. } |
            OpCode::Div { dst, .. } |
            OpCode::Mod { dst, .. } |
            OpCode::Pow { dst, .. } |
            OpCode::Equal { dst, .. } |
            OpCode::NotEqual { dst, .. } |
            OpCode::Greater { dst, .. } |
            OpCode::Less { dst, .. } |
            OpCode::GreaterEqual { dst, .. } |
            OpCode::LessEqual { dst, .. } |
            OpCode::And { dst, .. } |
            OpCode::Or { dst, .. } |
            OpCode::Has { dst, .. } |
            OpCode::SetUnion { dst, .. } |
            OpCode::SetIntersection { dst, .. } |
            OpCode::SetDifference { dst, .. } |
            OpCode::SetSymDifference { dst, .. } |
            OpCode::IntConcat { dst, .. } |
            OpCode::Not { dst, .. } |
            OpCode::CastInt { dst, .. } |
            OpCode::CastFloat { dst, .. } |
            OpCode::CastString { dst, .. } |
            OpCode::CastBool { dst, .. } |
            OpCode::Neg { dst, .. } |
            OpCode::Typeof { dst, .. } |
            OpCode::RandomChoice { dst, .. } |
            OpCode::JsonParse { dst, .. } |
            OpCode::EnvGet { dst, .. } |
            OpCode::TableCloneSkeleton { dst, .. } |
            OpCode::Input { dst, .. } |
            OpCode::TerminalExit { dst, .. } |
            OpCode::TerminalClear { dst, .. } |
            OpCode::TerminalRaw { dst, .. } |
            OpCode::TerminalNormal { dst, .. } |
            OpCode::TerminalCursor { dst, .. } |
            OpCode::TerminalRun { dst, .. } |
            OpCode::TerminalMove { dst, .. } |
            OpCode::TerminalWrite { dst, .. } |
            OpCode::InputKey { dst, .. } |
            OpCode::InputKeyWait { dst, .. } |
            OpCode::InputReady { dst, .. } |
            OpCode::DateNow { dst, .. } |
            OpCode::EnvArgs { dst, .. } |
            OpCode::Call { dst, .. } |
            OpCode::MethodCall { dst, .. } |
            OpCode::MethodCallNamed { dst, .. } |
            OpCode::MethodCallCustom { dst, .. } |
            OpCode::FiberCreate { dst, .. } |
            OpCode::ArrayInit { dst, .. } |
            OpCode::SetInit { dst, .. } |
            OpCode::MapInit { dst, .. } |
            OpCode::TableInit { dst, .. } |
            OpCode::TableBegin { dst, .. } |
            OpCode::SetRange { dst, .. } |
            OpCode::RandomInt { dst, .. } |
            OpCode::RandomFloat { dst, .. } |
            OpCode::StoreWrite { dst, .. } |
            OpCode::StoreRead { dst, .. } |
            OpCode::StoreAppend { dst, .. } |
            OpCode::StoreExists { dst, .. } |
            OpCode::StoreDelete { dst, .. } |
            OpCode::StoreList { dst, .. } |
            OpCode::StoreIsDir { dst, .. } |
            OpCode::StoreSize { dst, .. } |
            OpCode::StoreMkdir { dst, .. } |
            OpCode::StoreGlob { dst, .. } |
            OpCode::StoreZip { dst, .. } |
            OpCode::StoreUnzip { dst, .. } |
            OpCode::JsonBindLocal { dst, .. } |
            OpCode::JsonInjectLocal { table_reg: dst, .. } |
            OpCode::YieldWithTarget { dst, .. } |
            OpCode::HttpCall { dst, .. } |
            OpCode::HttpRequest { dst, .. } |
            OpCode::HttpRespond { dst, .. } |
            OpCode::CryptoHash { dst, .. } |
            OpCode::CryptoVerify { dst, .. } |
            OpCode::CryptoToken { dst, .. } |
            OpCode::GetIndex { dst, .. } |
            OpCode::GetMember { dst, .. } |
            OpCode::RowGet { dst, .. } |
            OpCode::MakeClosure { dst, .. } |
            OpCode::IncLocal { reg: dst } |
            OpCode::LoopNext { reg: dst, .. } |
            OpCode::IncLocalLoopNext { reg: dst, .. } |
            OpCode::IncVarLoopNext { reg: dst, .. } |
            OpCode::ArrayLoopNext { idx_reg: dst, .. } |
            OpCode::DatabaseInit { dst, .. } => Some(*dst),
            _ => None,
        }
    }
}
