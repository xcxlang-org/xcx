use crate::vm::opcode::OpCode;
use std::collections::HashMap;

pub struct RegisterManager;

impl RegisterManager {
    pub fn compress_registers(bytecode: &mut [OpCode], num_params: usize) -> usize {
        let mut mapping: HashMap<u8, u8> = HashMap::new();
        let mut next_free = num_params;
        let mut max_slot = if num_params > 0 { num_params - 1 } else { 0 };

        // 1. Pin parameters to their slots to preserve ABI.
        for i in 0..num_params {
            mapping.insert(i as u8, i as u8);
        }

        // 2. Identify and pre-map contiguous block registers to preserve adjacency.
        for op in bytecode.iter() {
            let (base, len) = match op {
                OpCode::Call { base, arg_count, .. } |
                OpCode::FiberCreate { base, arg_count, .. } => {
                    (*base, *arg_count as usize)
                }
                OpCode::MethodCall { base, arg_count, .. } |
                OpCode::MethodCallNamed { base, arg_count, .. } |
                OpCode::MethodCallCustom { base, arg_count, .. } => {
                    (*base, *arg_count as usize + 1)
                }
                OpCode::ArrayInit { base, count, .. } |
                OpCode::SetInit { base, count, .. } => {
                    (*base, *count as usize)
                }
                OpCode::MapInit { base, count, .. } => {
                    (*base, *count as usize * 2)
                }
                OpCode::TableInit { base, row_count, col_count, .. } => {
                    (*base, *row_count as usize * *col_count as usize)
                }
                OpCode::TableInitRow { base, col_count, .. } => {
                    (*base, *col_count as usize)
                }
                OpCode::MakeClosure { capture_start, capture_count, .. } => {
                    (*capture_start, *capture_count as usize)
                }
                OpCode::DatabaseInit { tables_base_reg, table_count, .. } => {
                    (*tables_base_reg, *table_count as usize * 2)
                }
                _ => continue,
            };

            if len == 0 { continue; }
            if base as usize >= num_params {
                let mut contiguous = true;
                if let Some(&start_slot) = mapping.get(&base) {
                    for i in 1..len {
                        if mapping.get(&(base + i as u8)) != Some(&(start_slot + i as u8)) {
                            contiguous = false;
                            break;
                        }
                    }
                } else {
                    contiguous = false;
                }

                if !contiguous {
                    let slot = next_free as u8;
                    next_free += len;
                    for i in 0..len {
                        let reg = base + i as u8;
                        let s = slot + i as u8;
                        mapping.insert(reg, s);
                        if s as usize > max_slot { max_slot = s as usize; }
                    }
                }
            }
        }

        // 3. Collect all other used registers.
        let mut used_regs: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
        let mut temp_regs = Vec::new();
        for op in bytecode.iter() {
            Self::collect_all_regs(op, &mut used_regs, &mut temp_regs);
        }

        // 4. Map used registers in increasing order to preserve contiguity.
        for reg in used_regs {
            if !mapping.contains_key(&reg) {
                let slot = next_free as u8;
                next_free += 1;
                mapping.insert(reg, slot);
                if slot as usize > max_slot { max_slot = slot as usize; }
            }
        }

        // 5. Apply mapping to all instructions.
        for op in bytecode.iter_mut() {
            Self::apply_mapping(op, &mapping);
        }

        if max_slot == 0 && num_params == 0 && bytecode.is_empty() { 0 } else { max_slot + 1 }
    }

    fn collect_all_regs(op: &OpCode, used: &mut std::collections::BTreeSet<u8>, regs: &mut Vec<u8>) {
        use super::liveness::LivenessAnalysis;
        regs.clear();
        LivenessAnalysis::collect_registers(op, regs);
        
        // Destination register
        if let Some(dst) = LivenessAnalysis::get_dst_reg(op) {
            used.insert(dst);
        }
        
        // Source registers
        for &r in regs.iter() {
            used.insert(r);
        }
    }

    fn apply_mapping(op: &mut OpCode, mapping: &HashMap<u8, u8>) {
        let map = |r: &mut u8| {
            if let Some(&m) = mapping.get(r) {
                *r = m;
            }
        };

        match op {
            OpCode::Move { dst, src } => { map(dst); map(src); }
            OpCode::LoadConst { dst, .. } => { map(dst); }
            
            OpCode::Add { dst, src1, src2 } |
            OpCode::Sub { dst, src1, src2 } |
            OpCode::Mul { dst, src1, src2 } |
            OpCode::Div { dst, src1, src2 } |
            OpCode::Mod { dst, src1, src2 } |
            OpCode::Pow { dst, src1, src2 } |
            OpCode::Equal { dst, src1, src2 } |
            OpCode::NotEqual { dst, src1, src2 } |
            OpCode::Greater { dst, src1, src2 } |
            OpCode::Less { dst, src1, src2 } |
            OpCode::GreaterEqual { dst, src1, src2 } |
            OpCode::LessEqual { dst, src1, src2 } |
            OpCode::And { dst, src1, src2 } |
            OpCode::Or { dst, src1, src2 } |
            OpCode::Has { dst, src1, src2 } |
            OpCode::SetUnion { dst, src1, src2 } |
            OpCode::SetIntersection { dst, src1, src2 } |
            OpCode::SetDifference { dst, src1, src2 } |
            OpCode::SetSymDifference { dst, src1, src2 } |
            OpCode::IntConcat { dst, src1, src2 } => { map(dst); map(src1); map(src2); }
            
            OpCode::Not { dst, src } |
            OpCode::CastInt { dst, src } |
            OpCode::CastFloat { dst, src } |
            OpCode::CastString { dst, src } |
            OpCode::CastBool { dst, src } |
            OpCode::Neg { dst, src } |
            OpCode::Typeof { dst, src } |
            OpCode::RandomChoice { dst, src } |
            OpCode::JsonParse { dst, src } |
            OpCode::EnvGet { dst, src } |
            OpCode::TableCloneSkeleton { dst, src } => { map(dst); map(src); }

            OpCode::GetVar { dst, .. } |
            OpCode::Input { dst, .. } |
            OpCode::TerminalExit { dst, .. } |
            OpCode::TerminalClear { dst, .. } |
            OpCode::TerminalRaw { dst, .. } |
            OpCode::TerminalNormal { dst, .. } |
            OpCode::TerminalCursor { dst, .. } |
            OpCode::InputKey { dst, .. } |
            OpCode::InputKeyWait { dst, .. } |
            OpCode::InputReady { dst, .. } |
            OpCode::DateNow { dst, .. } |
            OpCode::EnvArgs { dst, .. } => { map(dst); }
            
            OpCode::SetVar { src, .. } |
            OpCode::Print { src, .. } |
            OpCode::HaltAlert { src, .. } |
            OpCode::HaltError { src, .. } |
            OpCode::HaltFatal { src, .. } |
            OpCode::Return { src, .. } |
            OpCode::SetName { src, .. } |
            OpCode::Yield { src, .. } |
            OpCode::Wait { src, .. } => { map(src); }

            OpCode::JumpIfFalse { src, .. } |
            OpCode::JumpIfTrue { src, .. } => { map(src); }

            OpCode::TerminalRun { dst, cmd_src } => { map(dst); map(cmd_src); }
            OpCode::TerminalMove { dst, x_src, y_src } => { map(dst); map(x_src); map(y_src); }
            OpCode::TerminalWrite { dst, src } |
            OpCode::YieldWithTarget { dst, src } => { map(dst); map(src); }

            OpCode::Call { dst, base, .. } |
            OpCode::MethodCall { dst, base, .. } |
            OpCode::MethodCallNamed { dst, base, .. } |
            OpCode::MethodCallCustom { dst, base, .. } |
            OpCode::FiberCreate { dst, base, .. } => {
                map(dst);
                map(base);
            }

            OpCode::ArrayInit { dst, base, .. } |
            OpCode::SetInit { dst, base, .. } |
            OpCode::MapInit { dst, base, .. } => {
                map(dst);
                map(base);
            }
            
            OpCode::TableInit { dst, base, .. } => {
                map(dst);
                map(base);
            }
            OpCode::TableBegin { dst, .. } => {
                map(dst);
            }
            OpCode::TableInitRow { tbl_dst, base, .. } => {
                map(tbl_dst);
                map(base);
            }

            OpCode::SetRange { dst, start, end, step, .. } |
            OpCode::RandomInt { dst, min: start, max: end, step, .. } |
            OpCode::RandomFloat { dst, min: start, max: end, step, .. } => { map(dst); map(start); map(end); map(step); }
            
            OpCode::StoreWrite { dst, base } |
            OpCode::StoreRead { dst, base } |
            OpCode::StoreAppend { dst, base } |
            OpCode::StoreExists { dst, base } |
            OpCode::StoreDelete { dst, base } |
            OpCode::StoreList { dst, base } |
            OpCode::StoreIsDir { dst, base } |
            OpCode::StoreSize { dst, base } |
            OpCode::StoreMkdir { dst, base } |
            OpCode::StoreGlob { dst, base } |
            OpCode::StoreZip { dst, base } |
            OpCode::StoreUnzip { dst, base } => { map(dst); map(base); }

            OpCode::JsonBind { json_src, path_src, .. } => { map(json_src); map(path_src); }
            OpCode::JsonBindLocal { dst, json_src, path_src } => { map(dst); map(json_src); map(path_src); }
            OpCode::JsonInject { json_src, mapping_src, .. } => { map(json_src); map(mapping_src); }
            OpCode::JsonInjectLocal { table_reg, json_src, mapping_src } => { map(table_reg); map(json_src); map(mapping_src); }
            OpCode::JsonFastGetPush { json_src, path_src, val_src } => { map(json_src); map(path_src); map(val_src); }

            OpCode::HttpCall { dst, url_src, body_src, .. } => { map(dst); map(url_src); map(body_src); }
            OpCode::HttpRequest { dst, arg_src } => { map(dst); map(arg_src); }
            OpCode::HttpRespond { dst, status_src, body_src, headers_src } => { map(dst); map(status_src); map(body_src); map(headers_src); }
            OpCode::HttpServe { port_src, host_src, workers_src, routes_src, .. } => { map(port_src); map(host_src); map(workers_src); map(routes_src); }
            
            OpCode::CryptoHash { dst, pass_src, alg_src } => { map(dst); map(pass_src); map(alg_src); }
            OpCode::CryptoVerify { dst, pass_src, hash_src, alg_src } => { map(dst); map(pass_src); map(hash_src); map(alg_src); }
            OpCode::CryptoToken { dst, len_src } => { map(dst); map(len_src); }
            
            OpCode::IncLocal { reg } => { map(reg); }
            OpCode::LoopNext { reg, limit_reg, .. } => { map(reg); map(limit_reg); }
            OpCode::IncLocalLoopNext { inc_reg, reg, limit_reg, .. } => { map(inc_reg); map(reg); map(limit_reg); }
            OpCode::IncVarLoopNext { reg, limit_reg, .. } => { map(reg); map(limit_reg); }
            OpCode::ArrayLoopNext { idx_reg, size_reg, .. } => { map(idx_reg); map(size_reg); }
            
            OpCode::DatabaseInit { dst, engine_src, path_src, tables_base_reg, .. } => { map(dst); map(engine_src); map(path_src); map(tables_base_reg); }

            OpCode::GetIndex { dst, container, index } => { map(dst); map(container); map(index); }
            OpCode::SetIndex { container, index, src } => { map(container); map(index); map(src); }
            OpCode::GetMember { dst, container, .. } => { map(dst); map(container); }
            OpCode::SetMember { container, src, .. } => { map(container); map(src); }
            OpCode::RowGet { dst, row_reg, .. } => { map(dst); map(row_reg); }
            OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, .. } => { map(tbl_reg); map(idx_reg); map(row_reg); map(limit_reg); }
            OpCode::TablePushRow { tbl_reg, row_reg } => { map(tbl_reg); map(row_reg); }
            OpCode::MakeClosure { dst, capture_start, .. } => { map(dst); map(capture_start); }
            
            _ => {}
        }
    }
}
