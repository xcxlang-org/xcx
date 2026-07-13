use crate::vm::opcode::OpCode;
use std::collections::HashSet;


pub fn analyze_bool_array_regs(bytecode: &[OpCode], constants: &[crate::vm::value::Value]) -> HashSet<u8> {
    let mut bool_array: HashSet<u8> = HashSet::new();
    let mut non_bool_array: HashSet<u8> = HashSet::new();
    let mut reg_is_bool_const = [false; 256];
    let mut array_origins: [Option<u8>; 260] = [None; 260];
    let mut global_origins: std::collections::HashMap<u32, u8> = std::collections::HashMap::new();

    for op in bytecode {
        match op {
            OpCode::LoadConst { dst, idx } => {
                if let Some(val) = constants.get(*idx as usize) {
                    reg_is_bool_const[*dst as usize] = val.is_bool();
                } else {
                    reg_is_bool_const[*dst as usize] = false;
                }
            }
            OpCode::ArrayInit { dst, base, count } => {
                array_origins[*dst as usize] = Some(*dst);
                let mut all_bool = true;
                for i in 0..*count {
                    let reg = *base + i as u8;
                    if !reg_is_bool_const[reg as usize] {
                        all_bool = false;
                        break;
                    }
                }
                if all_bool {
                    bool_array.insert(*dst);
                } else {
                    non_bool_array.insert(*dst);
                }
            }
            OpCode::BoolArrayInit { dst } => {
                array_origins[*dst as usize] = Some(*dst);
                bool_array.insert(*dst);
            }
            OpCode::MethodCall { kind, base, arg_count, .. } => {
                let base_reg = *base;
                match kind {
                    crate::vm::opcode::MethodKind::Push => {
                        if *arg_count >= 1 {
                            let arg_reg = base_reg + 1;
                            if !reg_is_bool_const[arg_reg as usize] {
                                if let Some(origin) = array_origins[base_reg as usize] {
                                    non_bool_array.insert(origin);
                                } else {
                                    non_bool_array.insert(base_reg);
                                }
                            }
                        }
                    }
                    crate::vm::opcode::MethodKind::Set | crate::vm::opcode::MethodKind::Update => {
                        if *arg_count >= 2 {
                            let val_reg = base_reg + 2;
                            if !reg_is_bool_const[val_reg as usize] {
                                if let Some(origin) = array_origins[base_reg as usize] {
                                    non_bool_array.insert(origin);
                                } else {
                                    non_bool_array.insert(base_reg);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            OpCode::SetIndex { container, src, .. } => {
                if !reg_is_bool_const[*src as usize] {
                    if let Some(origin) = array_origins[*container as usize] {
                        non_bool_array.insert(origin);
                    } else {
                        non_bool_array.insert(*container);
                    }
                }
            }
            OpCode::Move { dst, src } => {
                reg_is_bool_const[*dst as usize] = reg_is_bool_const[*src as usize];
                array_origins[*dst as usize] = array_origins[*src as usize];
            }
            OpCode::GetVar { dst, idx } => {
                if let Some(origin) = global_origins.get(idx) {
                    array_origins[*dst as usize] = Some(*origin);
                } else {
                    array_origins[*dst as usize] = None;
                }
            }
            OpCode::SetVar { idx, src } => {
                if let Some(origin) = array_origins[*src as usize] {
                    global_origins.insert(*idx, origin);
                } else {
                    global_origins.remove(idx);
                }
            }
            _ => {}
        }
    }

    for reg in &non_bool_array {
        bool_array.remove(reg);
    }
    bool_array
}

pub fn analyze_chunk_locals(bytecode: &[OpCode]) -> Vec<u8> {
    analyze_locals_iter(bytecode.iter().cloned())
}

pub fn analyze_chunk_locals_init(bytecode: &[OpCode], arity: u8) -> Vec<u8> {
    let mut needs_init = HashSet::new();
    let mut written = HashSet::new();
    for i in 0..arity {
        written.insert(i); // Args are written by caller
    }

    // Helper macro to process a read
    macro_rules! read {
        ($reg:expr) => {
            if !written.contains(&$reg) {
                needs_init.insert($reg);
            }
        }
    }

    // Helper macro to process a write
    macro_rules! write {
        ($reg:expr) => {
            written.insert($reg);
        }
    }

    for op in bytecode.iter().cloned() {
        match op {
            OpCode::Move { dst, src } => { read!(src); write!(dst); }
            OpCode::LoadConst { dst, .. } => { write!(dst); }
            OpCode::Add { dst, src1, src2 } | OpCode::Sub { dst, src1, src2 } |
            OpCode::Mul { dst, src1, src2 } | OpCode::Div { dst, src1, src2 } |
            OpCode::Mod { dst, src1, src2 } | OpCode::Pow { dst, src1, src2 } |
            OpCode::Equal { dst, src1, src2 } | OpCode::NotEqual { dst, src1, src2 } |
            OpCode::Greater { dst, src1, src2 } | OpCode::Less { dst, src1, src2 } |
            OpCode::GreaterEqual { dst, src1, src2 } | OpCode::LessEqual { dst, src1, src2 } |
            OpCode::And { dst, src1, src2 } | OpCode::Or { dst, src1, src2 } |
            OpCode::IntConcat { dst, src1, src2 } | OpCode::Has { dst, src1, src2 } |
            OpCode::SetUnion { dst, src1, src2 } | OpCode::SetIntersection { dst, src1, src2 } |
            OpCode::SetDifference { dst, src1, src2 } | OpCode::SetSymDifference { dst, src1, src2 } => {
                read!(src1); read!(src2); write!(dst);
            }
            OpCode::Not { dst, src } | OpCode::Neg { dst, src } | OpCode::CastInt { dst, src } |
            OpCode::CastFloat { dst, src } | OpCode::CastString { dst, src } | OpCode::CastBool { dst, src } |
            OpCode::EnvGet { dst, src } => {
                read!(src); write!(dst);
            }
            OpCode::GetVar { dst, .. } | OpCode::DateNow { dst } | OpCode::EnvArgs { dst } | OpCode::InputKeyWait { dst } => {
                write!(dst);
            }
            OpCode::Typeof { dst, src } => { read!(src); write!(dst); }
            OpCode::JsonParse { dst, src } => { read!(src); write!(dst); }
            OpCode::JsonFastGetPush { json_src, path_src, val_src } => { read!(json_src); read!(path_src); read!(val_src); }

            OpCode::HttpCall { dst, url_src, body_src, .. } => { read!(url_src); read!(body_src); write!(dst); }
            OpCode::JumpIfFalse { src, .. } | OpCode::JumpIfTrue { src, .. } |
            OpCode::Return { src } | OpCode::Wait { src } | OpCode::Yield { src } |
            OpCode::HaltAlert { src } | OpCode::HaltError { src } | OpCode::HaltFatal { src } |
            OpCode::Print { src } | OpCode::StrAppendVar { src, .. } => {
                read!(src);
            }
            OpCode::StrAppendLocal { local_idx, src } => {
                read!(local_idx);
                read!(src);
                write!(local_idx);
            }
            OpCode::IncLocal { reg } => { read!(reg); write!(reg); }
            OpCode::LoopNext { reg, limit_reg, .. } | OpCode::IncVarLoopNext { reg, limit_reg, .. } |
            OpCode::ArrayLoopNext { idx_reg: reg, size_reg: limit_reg, .. } => {
                read!(reg); read!(limit_reg);
            }
            OpCode::IncLocalLoopNext { inc_reg, reg, limit_reg, .. } => {
                read!(inc_reg); write!(inc_reg); read!(reg); read!(limit_reg);
            }
            OpCode::Call { dst, base, arg_count, .. } |
            OpCode::FiberCreate { dst, base, arg_count, .. } |
            OpCode::MethodCall { dst, base, arg_count, .. } |
            OpCode::MethodCallNamed { dst, base, arg_count, .. } |
            OpCode::MethodCallCustom { dst, base, arg_count, .. } => {
                read!(base);
                for i in 0..arg_count { read!(base + 1 + i); }
                write!(dst);
            }
            OpCode::ArrayInit { dst, base, count } |
            OpCode::SetInit { dst, base, count } |
            OpCode::MapInit { dst, base, count } => {
                for i in 0..count { read!(base + (i as u8)); }
                write!(dst);
            }
            OpCode::BoolArrayInit { dst } => {
                write!(dst);
            }
            OpCode::GetIndex { dst, container, index } => {
                read!(container); read!(index); write!(dst);
            }
            OpCode::SetIndex { container, index, src } |
            OpCode::StrAppendElement { container, index, src } => {
                read!(container); read!(index); read!(src);
            }
            OpCode::GetMember { dst, container, .. } => {
                read!(container); write!(dst);
            }
            OpCode::SetMember { container, src, .. } |
            OpCode::StrAppendMember { container, src, .. } => {
                read!(container); read!(src);
            }
            OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, .. } => {
                read!(tbl_reg); read!(idx_reg); write!(row_reg); write!(limit_reg);
            }
            OpCode::TerminalWrite { dst, src } => { read!(src); write!(dst); }
            _ => {}
        }
    }
    needs_init.into_iter().collect()
}

pub fn analyze_trace_locals(trace: &[crate::vm::trace::TraceOp]) -> Vec<u8> {
    analyze_locals_iter(trace.iter().map(|top| top.to_opcode()))
}

pub fn analyze_chunk_globals(bytecode: &[OpCode]) -> Vec<u32> {
    analyze_globals_iter(bytecode.iter().cloned())
}

pub fn analyze_trace_globals(trace: &[crate::vm::trace::TraceOp]) -> Vec<u32> {
    analyze_globals_iter(trace.iter().map(|top| top.to_opcode()))
}

fn analyze_globals_iter<I>(bytecode: I) -> Vec<u32>
where
    I: IntoIterator<Item = OpCode>,
{
    let mut used = HashSet::new();
    for op in bytecode {
        match op {
            OpCode::GetVar { idx, .. } |
            OpCode::SetVar { idx, .. } |
            OpCode::IncVar { idx } |
            OpCode::JsonBind { idx, .. } |
            OpCode::JsonInject { table_idx: idx, .. } |
            OpCode::IncVarLoopNext { g_idx: idx, .. } => {
                used.insert(idx);
            }
            _ => {}
        }
    }
    used.into_iter().collect()
}

fn analyze_locals_iter<I>(bytecode: I) -> Vec<u8>
where
    I: IntoIterator<Item = OpCode>,
{
    let mut used = HashSet::new();
    for op in bytecode {
        match op {
            OpCode::Move { dst, src } | OpCode::Add { dst, src1: src, src2: _ } |
            OpCode::Sub { dst, src1: src, src2: _ } | OpCode::Mul { dst, src1: src, src2: _ } |
            OpCode::Div { dst, src1: src, src2: _ } | OpCode::Mod { dst, src1: src, src2: _ } |
            OpCode::Pow { dst, src1: src, src2: _ } | OpCode::Equal { dst, src1: src, src2: _ } |
            OpCode::NotEqual { dst, src1: src, src2: _ } | OpCode::Greater { dst, src1: src, src2: _ } |
            OpCode::Less { dst, src1: src, src2: _ } | OpCode::GreaterEqual { dst, src1: src, src2: _ } |
            OpCode::LessEqual { dst, src1: src, src2: _ } | OpCode::And { dst, src1: src, src2: _ } |
            OpCode::Or { dst, src1: src, src2: _ } | OpCode::IntConcat { dst, src1: src, src2: _ } |
            OpCode::Has { dst, src1: src, src2: _ } |
            OpCode::SetUnion { dst, src1: src, src2: _ } |
            OpCode::SetIntersection { dst, src1: src, src2: _ } |
            OpCode::SetDifference { dst, src1: src, src2: _ } |
            OpCode::SetSymDifference { dst, src1: src, src2: _ } => {
                used.insert(dst); used.insert(src);
                if let OpCode::Add { src2, .. } | OpCode::Sub { src2, .. } | OpCode::Mul { src2, .. } |
                       OpCode::Div { src2, .. } | OpCode::Mod { src2, .. } | OpCode::Pow { src2, .. } |
                       OpCode::Equal { src2, .. } | OpCode::NotEqual { src2, .. } | OpCode::Greater { src2, .. } |
                       OpCode::Less { src2, .. } | OpCode::GreaterEqual { src2, .. } | OpCode::LessEqual { src2, .. } |
                       OpCode::And { src2, .. } | OpCode::Or { src2, .. } | OpCode::IntConcat { src2, .. } |
                       OpCode::Has { src2, .. } |
                       OpCode::SetUnion { src2, .. } |
                       OpCode::SetIntersection { src2, .. } |
                       OpCode::SetDifference { src2, .. } |
                       OpCode::SetSymDifference { src2, .. } = op {
                    used.insert(src2);
                }
            }
            OpCode::LoadConst { dst, .. } | OpCode::GetVar { dst, .. } |
            OpCode::Typeof { dst, .. } => {
                used.insert(dst);
                if let OpCode::Typeof { src, .. } = op {
                    used.insert(src);
                }
            }
            OpCode::Not { dst, src } | OpCode::Neg { dst, src } | OpCode::CastInt { dst, src } |
            OpCode::CastFloat { dst, src } | OpCode::CastString { dst, src } | OpCode::CastBool { dst, src } => {
                used.insert(dst); used.insert(src);
            }
            OpCode::DateNow { dst } | OpCode::EnvArgs { dst } | OpCode::JsonParse { dst, .. } => {
                used.insert(dst);
                if let OpCode::JsonParse { src, .. } = op { used.insert(src); }
            }
            OpCode::JsonFastGetPush { json_src, path_src, val_src } => { 
                used.insert(json_src); used.insert(path_src); used.insert(val_src); 
            }
            OpCode::JumpIfFalse { src, .. } | OpCode::JumpIfTrue { src, .. } |
            OpCode::Return { src } | OpCode::Wait { src } | OpCode::IncLocal { reg: src } |
            OpCode::Yield { src } | OpCode::YieldWithTarget { src, .. } |
            OpCode::HaltAlert { src } | OpCode::HaltError { src } | OpCode::HaltFatal { src } |
            OpCode::Print { src } | OpCode::StrAppendVar { src, .. } => {
                used.insert(src);
            }
            OpCode::StrAppendLocal { local_idx, src } => {
                used.insert(local_idx);
                used.insert(src);
            }
            OpCode::LoopNext { reg, limit_reg, .. } |
            OpCode::IncVarLoopNext { reg, limit_reg, .. } |
            OpCode::ArrayLoopNext { idx_reg: reg, size_reg: limit_reg, .. } => {
                used.insert(reg);
                used.insert(limit_reg);
            }
            OpCode::IncLocalLoopNext { inc_reg, reg, limit_reg, .. } => {
                used.insert(inc_reg);
                used.insert(reg);
                used.insert(limit_reg);
            }
            OpCode::Call { dst, base, arg_count, .. } |
            OpCode::FiberCreate { dst, base, arg_count, .. } |
            OpCode::MethodCall { dst, base, arg_count, .. } |
            OpCode::MethodCallNamed { dst, base, arg_count, .. } |
            OpCode::MethodCallCustom { dst, base, arg_count, .. } => {
                used.insert(dst);
                used.insert(base);
                for i in 0..arg_count {
                    used.insert((base as usize + 1 + i as usize) as u8);
                }
            }
            OpCode::ArrayInit { dst, base, count } |
            OpCode::SetInit { dst, base, count } |
            OpCode::MapInit { dst, base, count } => {
                used.insert(dst);
                for i in 0..count {
                    used.insert((base as usize + i as usize) as u8);
                }
            }
            OpCode::BoolArrayInit { dst } => {
                used.insert(dst);
            }
            OpCode::GetIndex { dst, container, index } => {
                used.insert(dst); used.insert(container); used.insert(index);
            }
            OpCode::SetIndex { container, index, src } |
            OpCode::StrAppendElement { container, index, src } => {
                used.insert(container); used.insert(index); used.insert(src);
            }
            OpCode::GetMember { dst, container, .. } => {
                used.insert(dst); used.insert(container);
            }
            OpCode::SetMember { container, src, .. } |
            OpCode::StrAppendMember { container, src, .. } => {
                used.insert(container); used.insert(src);
            }
            OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, .. } => {
                used.insert(tbl_reg); used.insert(idx_reg); used.insert(row_reg); used.insert(limit_reg);
            }
            OpCode::DatabaseInit { dst, engine_src, path_src, tables_base_reg, .. } => {
                used.insert(dst); used.insert(engine_src); used.insert(path_src); used.insert(tables_base_reg);
            }
            OpCode::SetName { src, .. } => {
                used.insert(src);
            }
            OpCode::TableInit { dst, base, row_count, col_count, .. } => {
                used.insert(dst);
                for i in 0..(row_count * col_count) {
                    used.insert((base as usize + i as usize) as u8);
                }
            }
            OpCode::RandomInt { dst, min, max, step, has_step } |
            OpCode::RandomFloat { dst, min, max, step, has_step } => {
                used.insert(dst);
                used.insert(min);
                used.insert(max);
                if has_step != 0 {
                    used.insert(step);
                }
            }
            OpCode::StoreWrite { dst, base } | OpCode::StoreRead { dst, base } |
            OpCode::StoreAppend { dst, base } | OpCode::StoreExists { dst, base } |
            OpCode::StoreDelete { dst, base } | OpCode::StoreList { dst, base } |
            OpCode::StoreIsDir { dst, base } | OpCode::StoreSize { dst, base } |
            OpCode::StoreMkdir { dst, base } | OpCode::StoreGlob { dst, base } |
            OpCode::StoreZip { dst, base } | OpCode::StoreUnzip { dst, base } => {
                used.insert(dst);
                used.insert(base);
            }
            OpCode::JsonBind { json_src, path_src, .. } => {
                used.insert(json_src);
                used.insert(path_src);
            }
            OpCode::JsonBindLocal { dst, json_src, path_src } => {
                used.insert(dst);
                used.insert(json_src);
                used.insert(path_src);
            }
            OpCode::JsonInject { json_src, mapping_src, .. } => {
                used.insert(json_src);
                used.insert(mapping_src);
            }
            OpCode::JsonInjectLocal { table_reg, json_src, mapping_src } => {
                used.insert(table_reg);
                used.insert(json_src);
                used.insert(mapping_src);
            }
            OpCode::PerfMs { dst } | OpCode::PerfUs { dst } | OpCode::PerfNs { dst } => {
                used.insert(dst);
            }
            OpCode::HttpRequest { dst, arg_src } => {
                used.insert(dst);
                used.insert(arg_src);
            }
            OpCode::HttpRespond { dst, status_src, body_src, headers_src } => {
                used.insert(dst);
                used.insert(status_src);
                used.insert(body_src);
                used.insert(headers_src);
            }
            OpCode::HttpServe { port_src, host_src, workers_src, routes_src, .. } => {
                used.insert(port_src);
                used.insert(host_src);
                used.insert(workers_src);
                used.insert(routes_src);
            }
            OpCode::EnvGet { dst, src } => {
                used.insert(dst);
                used.insert(src);
            }
            OpCode::CryptoHash { dst, pass_src, alg_src } => {
                used.insert(dst);
                used.insert(pass_src);
                used.insert(alg_src);
            }
            OpCode::CryptoVerify { dst, pass_src, hash_src, alg_src } => {
                used.insert(dst);
                used.insert(pass_src);
                used.insert(hash_src);
                used.insert(alg_src);
            }
            OpCode::CryptoToken { dst, len_src } => {
                used.insert(dst);
                used.insert(len_src);
            }
            OpCode::DecLocal { reg } => {
                used.insert(reg);
            }
            OpCode::LoopPrev { reg, limit_reg, .. } |
            OpCode::DecVarLoopPrev { reg, limit_reg, .. } => {
                used.insert(reg);
                used.insert(limit_reg);
            }
            OpCode::DecLocalLoopPrev { dec_reg, reg, limit_reg, .. } => {
                used.insert(dec_reg);
                used.insert(reg);
                used.insert(limit_reg);
            }
            OpCode::MakeClosure { dst, capture_count, capture_start, .. } => {
                used.insert(dst);
                for i in 0..capture_count {
                    used.insert((capture_start as usize + i as usize) as u8);
                }
            }
            OpCode::TerminalCursor { dst, .. } => {
                used.insert(dst);
            }
            OpCode::TerminalMove { dst, x_src, y_src } => {
                used.insert(dst);
                used.insert(x_src);
                used.insert(y_src);
            }
            OpCode::RowGet { dst, row_reg, .. } => {
                used.insert(dst);
                used.insert(row_reg);
            }
            OpCode::TablePushRow { tbl_reg, row_reg } => {
                used.insert(tbl_reg);
                used.insert(row_reg);
            }
            OpCode::TableCloneSkeleton { dst, src } => {
                used.insert(dst);
                used.insert(src);
            }
            OpCode::TableBegin { dst, .. } => {
                used.insert(dst);
            }
            OpCode::TableInitRow { tbl_dst, base, col_count } => {
                used.insert(tbl_dst);
                for i in 0..col_count {
                    used.insert((base as usize + i as usize) as u8);
                }
            }
            _ => {}
        }
    }
    used.into_iter().collect()
}

pub fn analyze_global_int_regs(bytecode: &[OpCode], constants: &[crate::vm::value::Value]) -> HashSet<u32> {
    let mut global_ints = HashSet::new();
    let mut global_non_ints = HashSet::new();
    let mut reg_is_int = [false; 256];

    for op in bytecode {
        match op {
            OpCode::LoadConst { dst, idx } => {
                if let Some(val) = constants.get(*idx as usize) {
                    reg_is_int[*dst as usize] = val.is_int();
                } else {
                    reg_is_int[*dst as usize] = false;
                }
            }
            OpCode::Move { dst, src } => {
                reg_is_int[*dst as usize] = reg_is_int[*src as usize];
            }
            OpCode::Add { dst, src1, src2 } | OpCode::Sub { dst, src1, src2 } |
            OpCode::Mul { dst, src1, src2 } | OpCode::Div { dst, src1, src2 } |
            OpCode::Mod { dst, src1, src2 } => {
                if reg_is_int[*src1 as usize] && reg_is_int[*src2 as usize] {
                    reg_is_int[*dst as usize] = true;
                } else {
                    reg_is_int[*dst as usize] = false;
                }
            }
            OpCode::CastInt { dst, src: _ } => {
                reg_is_int[*dst as usize] = true;
            }
            OpCode::Neg { dst, src } => {
                reg_is_int[*dst as usize] = reg_is_int[*src as usize];
            }
            OpCode::RandomInt { dst, .. } => {
                reg_is_int[*dst as usize] = true;
            }
            OpCode::IncLocal { reg } => {
                reg_is_int[*reg as usize] = true;
            }
            OpCode::SetVar { idx, src } => {
                if reg_is_int[*src as usize] {
                    if !global_non_ints.contains(idx) {
                        global_ints.insert(*idx);
                    }
                } else {
                    global_ints.remove(idx);
                    global_non_ints.insert(*idx);
                }
            }
            OpCode::GetVar { dst, idx } => {
                reg_is_int[*dst as usize] = global_ints.contains(idx);
            }
            OpCode::IncVar { idx } => {
                if !global_non_ints.contains(idx) {
                    global_ints.insert(*idx);
                }
            }
            
            // All other opcodes with a `dst` register: must reset the integer status to false.
            OpCode::Equal { dst, .. } | OpCode::NotEqual { dst, .. } |
            OpCode::Greater { dst, .. } | OpCode::Less { dst, .. } |
            OpCode::GreaterEqual { dst, .. } | OpCode::LessEqual { dst, .. } |
            OpCode::And { dst, .. } | OpCode::Or { dst, .. } | OpCode::Not { dst, .. } |
            OpCode::Has { dst, .. } | OpCode::Input { dst, .. } |
            OpCode::TerminalExit { dst } | OpCode::TerminalRun { dst, .. } |
            OpCode::TerminalClear { dst } | OpCode::TerminalRaw { dst } |
            OpCode::TerminalNormal { dst } | OpCode::TerminalCursor { dst, .. } |
            OpCode::TerminalMove { dst, .. } | OpCode::TerminalWrite { dst, .. } |
            OpCode::InputKey { dst } | OpCode::InputKeyWait { dst } | OpCode::InputReady { dst } |
            OpCode::Call { dst, .. } | OpCode::SetName { src: dst, .. } |
            OpCode::ArrayInit { dst, .. } | OpCode::BoolArrayInit { dst } | OpCode::SetInit { dst, .. } |
            OpCode::MapInit { dst, .. } | OpCode::TableInit { dst, .. } |
            OpCode::JsonParse { dst, .. } | OpCode::DateNow { dst } |
            OpCode::MethodCall { dst, .. } | OpCode::MethodCallCustom { dst, .. } |
            OpCode::SetUnion { dst, .. } | OpCode::SetIntersection { dst, .. } |
            OpCode::SetDifference { dst, .. } | OpCode::SetSymDifference { dst, .. } |
            OpCode::RandomChoice { dst, .. } | OpCode::IntConcat { dst, .. } |
            OpCode::SetRange { dst, .. } | OpCode::RandomFloat { dst, .. } |
            OpCode::StoreWrite { dst, .. } | OpCode::StoreRead { dst, .. } |
            OpCode::StoreAppend { dst, .. } | OpCode::StoreExists { dst, .. } |
            OpCode::StoreDelete { dst, .. } | OpCode::StoreList { dst, .. } |
            OpCode::StoreIsDir { dst, .. } | OpCode::StoreSize { dst, .. } |
            OpCode::StoreMkdir { dst, .. } | OpCode::StoreGlob { dst, .. } |
            OpCode::StoreZip { dst, .. } | OpCode::StoreUnzip { dst, .. } |
            OpCode::JsonBindLocal { dst, .. } | OpCode::JsonInjectLocal { table_reg: dst, .. } |
            OpCode::FiberCreate { dst, .. } | OpCode::YieldWithTarget { dst, .. } |
            OpCode::HttpCall { dst, .. } | OpCode::HttpRequest { dst, .. } |
            OpCode::HttpRespond { dst, .. } | OpCode::EnvGet { dst, .. } |
            OpCode::EnvArgs { dst } | OpCode::CryptoHash { dst, .. } |
            OpCode::CryptoVerify { dst, .. } | OpCode::CryptoToken { dst, .. } |
            OpCode::CastFloat { dst, .. } | OpCode::CastString { dst, .. } |
            OpCode::CastBool { dst, .. } | OpCode::Typeof { dst, .. } |
            OpCode::GetIndex { dst, .. } | OpCode::GetMember { dst, .. } |
            OpCode::RowGet { dst, .. } | OpCode::TableCloneSkeleton { dst, .. } => {
                reg_is_int[*dst as usize] = false;
            }
            
            _ => {}
        }
    }
    global_ints
}

pub fn analyze_non_ptr_regs(
    bytecode: &[OpCode],
    arity: usize,
    global_ints: &HashSet<u32>,
    constants: &[crate::vm::value::Value],
) -> HashSet<u8> {
    let mut non_ptr_regs = HashSet::new();
    for r in 0..256 {
        non_ptr_regs.insert(r as u8);
    }
    for i in 0..arity {
        non_ptr_regs.remove(&(i as u8));
    }
    let mut changed = true;
    while changed {
        changed = false;
        for op in bytecode {
            match op {
                OpCode::Input { dst, .. }
                | OpCode::TerminalExit { dst }
                | OpCode::TerminalRun { dst, .. }
                | OpCode::TerminalClear { dst }
                | OpCode::TerminalRaw { dst }
                | OpCode::TerminalNormal { dst }
                | OpCode::TerminalCursor { dst, .. }
                | OpCode::TerminalMove { dst, .. }
                | OpCode::TerminalWrite { dst, .. }
                | OpCode::InputKey { dst }
                | OpCode::InputKeyWait { dst }
                | OpCode::InputReady { dst }
                | OpCode::Call { dst, .. }
                | OpCode::ArrayInit { dst, .. }
                | OpCode::BoolArrayInit { dst }
                | OpCode::SetInit { dst, .. }
                | OpCode::MapInit { dst, .. }
                | OpCode::TableInit { dst, .. }
                | OpCode::MethodCall { dst, .. }
                | OpCode::MethodCallCustom { dst, .. }
                | OpCode::GetIndex { dst, .. }
                | OpCode::GetMember { dst, .. }
                | OpCode::RowGet { dst, .. }
                | OpCode::JsonParse { dst, .. }
                | OpCode::DateNow { dst }
                | OpCode::JsonBindLocal { dst, .. }
                | OpCode::JsonInjectLocal { table_reg: dst, .. }
                | OpCode::FiberCreate { dst, .. }
                | OpCode::YieldWithTarget { dst, .. }
                | OpCode::HttpCall { dst, .. }
                | OpCode::HttpRequest { dst, .. }
                | OpCode::HttpRespond { dst, .. }
                | OpCode::EnvGet { dst, .. }
                | OpCode::EnvArgs { dst }
                | OpCode::CryptoHash { dst, .. }
                | OpCode::CryptoVerify { dst, .. }
                | OpCode::CryptoToken { dst, .. }
                | OpCode::RandomChoice { dst, .. }
                | OpCode::SetRange { dst, .. }
                | OpCode::StoreRead { dst, .. }
                | OpCode::StoreWrite { dst, .. }
                | OpCode::StoreAppend { dst, .. }
                | OpCode::StoreExists { dst, .. }
                | OpCode::StoreDelete { dst, .. }
                | OpCode::StoreList { dst, .. }
                | OpCode::StoreIsDir { dst, .. }
                | OpCode::StoreSize { dst, .. }
                | OpCode::StoreMkdir { dst, .. }
                | OpCode::StoreGlob { dst, .. }
                | OpCode::StoreZip { dst, .. }
                | OpCode::StoreUnzip { dst, .. }
                | OpCode::DatabaseInit { dst, .. }
                | OpCode::MethodCallNamed { dst, .. }
                | OpCode::MakeClosure { dst, .. }
                | OpCode::Typeof { dst, .. }
                | OpCode::TableCloneSkeleton { dst, .. } => {
                    if non_ptr_regs.remove(dst) {
                        changed = true;
                    }
                }
                OpCode::StrAppendLocal { local_idx, .. } => {
                    if non_ptr_regs.remove(&local_idx) {
                        changed = true;
                    }
                }
                OpCode::GetVar { dst, idx } => {
                    if !global_ints.contains(idx) {
                        if non_ptr_regs.remove(dst) {
                            changed = true;
                        }
                    }
                }
                OpCode::LoadConst { dst, idx } => {
                    if let Some(val) = constants.get(*idx as usize) {
                        if val.is_ptr() {
                            if non_ptr_regs.remove(dst) {
                                changed = true;
                            }
                        }
                    } else {
                        if non_ptr_regs.remove(dst) {
                            changed = true;
                        }
                    }
                }
                OpCode::Move { dst, src } => {
                    if !non_ptr_regs.contains(src) {
                        if non_ptr_regs.remove(dst) {
                            changed = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    non_ptr_regs
}

/// Identifies global variable indices that are exclusively used as integers throughout a trace.
/// Used by the trace compiler to elide ref-counting on integer globals (e.g. LCG state, counters).
pub fn analyze_trace_global_ints(ops: &[crate::vm::trace::TraceOp]) -> HashSet<u32> {
    use crate::vm::trace::TraceOp;
    let mut global_ints: HashSet<u32> = HashSet::new();
    let mut global_non_ints: HashSet<u32> = HashSet::new();
    let mut reg_is_int = [false; 256];

    for op in ops {
        match op {
            TraceOp::LoadConst { dst, val } => {
                reg_is_int[*dst as usize] = val.is_int();
            }
            TraceOp::Move { dst, src } => {
                reg_is_int[*dst as usize] = reg_is_int[*src as usize];
            }
            TraceOp::AddInt { dst, .. } | TraceOp::SubInt { dst, .. }
            | TraceOp::MulInt { dst, .. } | TraceOp::ModInt { dst, .. }
            | TraceOp::DivInt { dst, .. } | TraceOp::PowInt { dst, .. }
            | TraceOp::IntConcat { dst, .. } | TraceOp::NegInt { dst, .. } => {
                reg_is_int[*dst as usize] = true;
            }
            TraceOp::IncLocal { reg } => {
                reg_is_int[*reg as usize] = true;
            }
            TraceOp::SetVar { idx, src } => {
                if reg_is_int[*src as usize] {
                    if !global_non_ints.contains(idx) {
                        global_ints.insert(*idx);
                    }
                } else {
                    global_ints.remove(idx);
                    global_non_ints.insert(*idx);
                }
            }
            TraceOp::GetVar { dst, idx } => {
                reg_is_int[*dst as usize] = global_ints.contains(idx);
            }
            TraceOp::IncVar { g_idx } => {
                if !global_non_ints.contains(g_idx) {
                    global_ints.insert(*g_idx);
                }
            }
            _ => {}
        }
    }
    global_ints
}

/// Identifies local registers in a trace that never hold heap pointers.
/// Used to elide dec_ref in the trace compiler's hot path.
pub fn analyze_trace_non_ptr_regs(ops: &[crate::vm::trace::TraceOp], global_ints: &HashSet<u32>) -> HashSet<u8> {
    use crate::vm::trace::TraceOp;
    let mut non_ptr: HashSet<u8> = (0u8..=255).collect();

    let mut changed = true;
    while changed {
        changed = false;
        for op in ops {
            match op {
                TraceOp::GetVar { dst, idx } => {
                    if !global_ints.contains(idx) {
                        if non_ptr.remove(dst) { changed = true; }
                    }
                }
                TraceOp::LoadConst { dst, val } => {
                    if val.is_ptr() {
                        if non_ptr.remove(dst) { changed = true; }
                    }
                }
                TraceOp::Move { dst, src } => {
                    if !non_ptr.contains(src) {
                        if non_ptr.remove(dst) { changed = true; }
                    }
                }
                TraceOp::ArrayGet { dst, .. } | TraceOp::ArrayGetIndex { dst, .. }
                | TraceOp::GetMember { dst, .. } | TraceOp::JsonBindLocal { dst, .. }
                | TraceOp::JsonBindLocalConst { dst, .. } | TraceOp::JsonParse { dst, .. }
                | TraceOp::FiberNext { dst, .. } | TraceOp::Call { dst, .. }
                | TraceOp::TableCloneSkeleton { dst, .. } | TraceOp::RowGet { dst, .. } => {
                    if non_ptr.remove(dst) { changed = true; }
                }
                _ => {}
            }
        }
    }
    non_ptr
}

/// Propagates potential pointer-containing states for all registers across all control-flow paths.
/// Used to precisely identify which registers do not hold pointer values at any given instruction.
#[inline(always)]
fn get_ptr_bit(st: &[u64; 4], reg: u8) -> bool {
    (st[(reg / 64) as usize] & (1u64 << (reg % 64))) != 0
}

#[inline(always)]
fn set_ptr_bit(st: &mut [u64; 4], reg: u8, val: bool) {
    let word = &mut st[(reg / 64) as usize];
    let mask = 1u64 << (reg % 64);
    if val {
        *word |= mask;
    } else {
        *word &= !mask;
    }
}

/// Propagates potential pointer-containing states for all registers across all control-flow paths.
/// Used to precisely identify which registers do not hold pointer values at any given instruction.
pub fn analyze_maybe_ptr_regs(
    bytecode: &[OpCode],
    global_ints: &HashSet<u32>,
    constants: &[crate::vm::value::Value],
) -> Vec<[u64; 4]> {
    let n = bytecode.len();
    let mut state = vec![[0u64; 4]; n];
    let mut in_queue = vec![false; n];
    let mut queue = std::collections::VecDeque::new();

    if n > 0 {
        queue.push_back(0);
        in_queue[0] = true;
    }

    while let Some(ip) = queue.pop_front() {
        in_queue[ip] = false;
        let op = bytecode[ip];
        let mut next_state = state[ip];

        match op {
            OpCode::Move { dst, src } => {
                let val = get_ptr_bit(&next_state, src);
                set_ptr_bit(&mut next_state, dst, val);
            }
            OpCode::LoadConst { dst, idx } => {
                let is_ptr = if let Some(val) = constants.get(idx as usize) {
                    val.is_ptr()
                } else {
                    true
                };
                set_ptr_bit(&mut next_state, dst, is_ptr);
            }
            OpCode::GetVar { dst, idx } => {
                let is_ptr = !global_ints.contains(&idx);
                set_ptr_bit(&mut next_state, dst, is_ptr);
            }
            OpCode::Add { dst, .. } | OpCode::Sub { dst, .. } |
            OpCode::Mul { dst, .. } | OpCode::Div { dst, .. } |
            OpCode::Mod { dst, .. } | OpCode::Pow { dst, .. } |
            OpCode::Equal { dst, .. } | OpCode::NotEqual { dst, .. } |
            OpCode::Greater { dst, .. } | OpCode::Less { dst, .. } |
            OpCode::GreaterEqual { dst, .. } | OpCode::LessEqual { dst, .. } |
            OpCode::And { dst, .. } | OpCode::Or { dst, .. } |
            OpCode::Not { dst, .. } | OpCode::Has { dst, .. } |
            OpCode::Neg { dst, .. } | OpCode::CastInt { dst, .. } |
            OpCode::CastFloat { dst, .. } | OpCode::CastBool { dst, .. } |
            OpCode::RandomInt { dst, .. } => {
                set_ptr_bit(&mut next_state, dst, false);
            }
            OpCode::IncLocal { reg } | OpCode::DecLocal { reg } |
            OpCode::IncLocalLoopNext { inc_reg: reg, .. } |
            OpCode::DecLocalLoopPrev { dec_reg: reg, .. } => {
                set_ptr_bit(&mut next_state, reg, false);
            }
            OpCode::LoopNext { reg, .. } | OpCode::LoopPrev { reg, .. } => {
                set_ptr_bit(&mut next_state, reg, false);
            }
            OpCode::IncVar { .. } | OpCode::DecVar { .. } => {}
            OpCode::IncVarLoopNext { reg, .. } | OpCode::DecVarLoopPrev { reg, .. } => {
                set_ptr_bit(&mut next_state, reg, false);
            }
            OpCode::ArrayLoopNext { idx_reg, .. } => {
                set_ptr_bit(&mut next_state, idx_reg, false);
            }
            OpCode::Input { dst, .. } |
            OpCode::TerminalExit { dst } | OpCode::TerminalRun { dst, .. } |
            OpCode::TerminalClear { dst } | OpCode::TerminalRaw { dst } |
            OpCode::TerminalNormal { dst } | OpCode::TerminalCursor { dst, .. } |
            OpCode::TerminalMove { dst, .. } | OpCode::TerminalWrite { dst, .. } |
            OpCode::InputKey { dst } | OpCode::InputKeyWait { dst } | OpCode::InputReady { dst } |
            OpCode::Call { dst, .. } | OpCode::SetName { src: dst, .. } |
            OpCode::EnvGet { dst, .. } | OpCode::EnvArgs { dst } |
            OpCode::CryptoHash { dst, .. } | OpCode::CryptoVerify { dst, .. } |
            OpCode::CryptoToken { dst, .. } |
            OpCode::CastString { dst, .. } |
            OpCode::ArrayInit { dst, .. } | OpCode::BoolArrayInit { dst } | OpCode::SetInit { dst, .. } |
            OpCode::MapInit { dst, .. } | OpCode::TableInit { dst, .. } |
            OpCode::MethodCall { dst, .. } | OpCode::MethodCallCustom { dst, .. } |
            OpCode::SetUnion { dst, .. } | OpCode::SetIntersection { dst, .. } |
            OpCode::SetDifference { dst, .. } | OpCode::SetSymDifference { dst, .. } |
            OpCode::RandomChoice { dst, .. } | OpCode::IntConcat { dst, .. } |
            OpCode::SetRange { dst, .. } | OpCode::RandomFloat { dst, .. } |
            OpCode::StoreWrite { dst, .. } | OpCode::StoreRead { dst, .. } |
            OpCode::StoreAppend { dst, .. } | OpCode::StoreExists { dst, .. } |
            OpCode::StoreDelete { dst, .. } | OpCode::StoreList { dst, .. } |
            OpCode::StoreIsDir { dst, .. } | OpCode::StoreSize { dst, .. } |
            OpCode::StoreMkdir { dst, .. } | OpCode::StoreGlob { dst, .. } |
            OpCode::StoreZip { dst, .. } | OpCode::StoreUnzip { dst, .. } |
            OpCode::JsonParse { dst, .. } | OpCode::DateNow { dst } |
            OpCode::JsonBindLocal { dst, .. } | OpCode::JsonInjectLocal { table_reg: dst, .. } |
            OpCode::FiberCreate { dst, .. } | OpCode::YieldWithTarget { dst, .. } |
            OpCode::HttpCall { dst, .. } | OpCode::HttpRequest { dst, .. } |
            OpCode::HttpRespond { dst, .. } |
            OpCode::DatabaseInit { dst, .. } |
            OpCode::MethodCallNamed { dst, .. } |
            OpCode::MakeClosure { dst, .. } |
            OpCode::Typeof { dst, .. } |
            OpCode::GetIndex { dst, .. } |
            OpCode::GetMember { dst, .. } |
            OpCode::RowGet { dst, .. } |
            OpCode::TableCloneSkeleton { dst, .. } => {
                set_ptr_bit(&mut next_state, dst, true);
            }
            OpCode::StrAppendLocal { local_idx, .. } => {
                set_ptr_bit(&mut next_state, local_idx, true);
            }
            OpCode::TableIter { idx_reg, row_reg, .. } => {
                set_ptr_bit(&mut next_state, idx_reg, false);
                set_ptr_bit(&mut next_state, row_reg, true);
            }
            _ => {}
        }

        let mut successors = Vec::new();
        match op {
            OpCode::Jump { target } => {
                successors.push(target as usize);
            }
            OpCode::JumpIfFalse { target, .. } | OpCode::JumpIfTrue { target, .. } => {
                successors.push(target as usize);
                successors.push(ip + 1);
            }
            OpCode::LoopNext { target, .. } | OpCode::LoopPrev { target, .. } |
            OpCode::IncLocalLoopNext { target, .. } | OpCode::DecLocalLoopPrev { target, .. } |
            OpCode::IncVarLoopNext { target, .. } | OpCode::DecVarLoopPrev { target, .. } |
            OpCode::ArrayLoopNext { target, .. } | OpCode::TableIter { target, .. } => {
                successors.push(target as usize);
                successors.push(ip + 1);
            }
            OpCode::Return { .. } | OpCode::ReturnVoid | OpCode::Halt => {}
            _ => {
                successors.push(ip + 1);
            }
        }

        for succ in successors {
            if succ < n {
                let mut changed = false;
                let next_u64 = next_state;
                let succ_state = &mut state[succ];
                let diff0 = next_u64[0] & !succ_state[0];
                let diff1 = next_u64[1] & !succ_state[1];
                let diff2 = next_u64[2] & !succ_state[2];
                let diff3 = next_u64[3] & !succ_state[3];
                if (diff0 | diff1 | diff2 | diff3) != 0 {
                    succ_state[0] |= next_u64[0];
                    succ_state[1] |= next_u64[1];
                    succ_state[2] |= next_u64[2];
                    succ_state[3] |= next_u64[3];
                    changed = true;
                }
                if changed && !in_queue[succ] {
                    queue.push_back(succ);
                    in_queue[succ] = true;
                }
            }
        }
    }

    state
}
