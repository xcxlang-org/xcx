use crate::vm::opcode::{OpCode, TypeTag};
use crate::vm::value::Value as VMValue;
use std::collections::{VecDeque, HashSet};

pub fn infer_param_types(bytecode: &[OpCode], arity: usize, _constants: &[VMValue]) -> [TypeTag; 256] {
    let mut types = [TypeTag::Unknown; 256];
    for reg in 0..arity as u8 {
        for op in bytecode {
            match op {
                OpCode::Less { src1, src2, .. } | OpCode::Greater { src1, src2, .. } |
                OpCode::LessEqual { src1, src2, .. } | OpCode::GreaterEqual { src1, src2, .. } => {
                    if *src1 == reg || *src2 == reg {
                        types[reg as usize] = TypeTag::Int;
                        break;
                    }
                }
                OpCode::Add { src1, src2, .. } | OpCode::Sub { src1, src2, .. } |
                OpCode::Mul { src1, src2, .. } | OpCode::Div { src1, src2, .. } |
                OpCode::Mod { src1, src2, .. } => {
                    if *src1 == reg || *src2 == reg {
                        types[reg as usize] = TypeTag::Int;
                        break;
                    }
                }
                OpCode::IncLocal { reg: r } | OpCode::DecLocal { reg: r } => {
                    if *r == reg {
                        types[reg as usize] = TypeTag::Int;
                        break;
                    }
                }
                OpCode::JumpIfTrue { src, .. } | OpCode::JumpIfFalse { src, .. } |
                OpCode::Not { src, .. } => {
                    if *src == reg {
                        types[reg as usize] = TypeTag::Bool;
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    types
}

pub fn analyze_chunk_types(
    bytecode: &[OpCode],
    constants: &[VMValue],
    initial_types: Option<&[TypeTag; 256]>,
    arity: usize,
    self_func_idx: u32,
    bool_array_hints: &HashSet<u8>,
) -> (Vec<[TypeTag; 256]>, bool) {
    let mut uses_heap = false;

    // Identify block boundaries (for jump targets)
    let mut block_starts = HashSet::new();
    block_starts.insert(0);
    for op in bytecode {
        if let Some(target) = op.jump_target() {
            block_starts.insert(target as usize);
        }
    }

    // Types at the START of each instruction
    let mut types_at_ip: Vec<[TypeTag; 256]> = vec![[TypeTag::Unknown; 256]; bytecode.len()];
    if let Some(init) = initial_types {
        types_at_ip[0].copy_from_slice(init);
    } else {
        types_at_ip[0] = infer_param_types(bytecode, arity, constants);
    }

    let mut worklist = VecDeque::new();
    worklist.push_back(0);
    let mut in_worklist = vec![false; bytecode.len()];
    in_worklist[0] = true;
    let mut visited = vec![false; bytecode.len()];
    visited[0] = true;

    let mut global_types = std::collections::HashMap::new();

    while let Some(ip) = worklist.pop_front() {
        in_worklist[ip] = false;
        if ip >= bytecode.len() { continue; }
        
        let op = &bytecode[ip];
        let mut out_types = types_at_ip[ip];
        
        // Transition function
        match op {
            OpCode::LoadConst { dst, idx } => {
                if let Some(val) = constants.get(*idx as usize) {
                    let ty = if val.is_int() { TypeTag::Int }
                            else if val.is_float() { TypeTag::Float }
                            else if val.is_bool() { TypeTag::Bool }
                            else if val.is_string() { TypeTag::String }
                            else if val.is_date() { TypeTag::Date }
                            else if val.is_array() { TypeTag::Array }
                            else if val.is_bool_array() { TypeTag::BoolArray }
                            else if val.is_map() { TypeTag::Map }
                            else if val.is_set() { TypeTag::Set }
                            else if val.is_table() { TypeTag::Table }
                            else if val.is_func() { TypeTag::Function }
                            else if val.is_row() { TypeTag::Row }
                            else if val.is_json() { TypeTag::Json }
                            else if val.is_fiber() { TypeTag::Fiber }
                            else if val.is_db() { TypeTag::Database }
                            else { TypeTag::Unknown };
                    out_types[*dst as usize] = ty;
                }
            }
            OpCode::CastInt { dst, .. } => { out_types[*dst as usize] = TypeTag::Int; }
            OpCode::CastFloat { dst, .. } => { out_types[*dst as usize] = TypeTag::Float; }
            OpCode::CastString { dst, .. } => { out_types[*dst as usize] = TypeTag::String; }
            OpCode::CastBool { dst, .. } => { out_types[*dst as usize] = TypeTag::Bool; }
            OpCode::IncLocal { reg } | OpCode::DecLocal { reg } => { out_types[*reg as usize] = TypeTag::Int; }
            OpCode::LoopNext { reg, .. } | OpCode::LoopPrev { reg, .. } | OpCode::IncVarLoopNext { reg, .. } | 
            OpCode::DecVarLoopPrev { reg, .. } | OpCode::ArrayLoopNext { idx_reg: reg, .. } |
            OpCode::TableIter { idx_reg: reg, .. } => {
                out_types[*reg as usize] = TypeTag::Int;
            }
            OpCode::IncLocalLoopNext { inc_reg, reg, .. } => {
                out_types[*inc_reg as usize] = TypeTag::Int;
                out_types[*reg as usize] = TypeTag::Int;
            }
            OpCode::DecLocalLoopPrev { dec_reg, reg, .. } => {
                out_types[*dec_reg as usize] = TypeTag::Int;
                out_types[*reg as usize] = TypeTag::Int;
            }
            OpCode::Add { dst, src1, src2 } | OpCode::Sub { dst, src1, src2 } |
            OpCode::Mul { dst, src1, src2 } | OpCode::Div { dst, src1, src2 } |
            OpCode::Mod { dst, src1, src2 } | OpCode::Pow { dst, src1, src2 } => {
                let t1 = out_types[*src1 as usize];
                let t2 = out_types[*src2 as usize];
                let is_add = matches!(op, OpCode::Add { .. });
                let is_sub = matches!(op, OpCode::Sub { .. });
                let ty = if is_add {
                    if t1 == TypeTag::Date && t2 == TypeTag::Int {
                        TypeTag::Date
                    } else if t1 == TypeTag::Int && t2 == TypeTag::Date {
                        TypeTag::Date
                    } else if t1 == TypeTag::Int && t2 == TypeTag::Int {
                        TypeTag::Int
                    } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                        TypeTag::Float
                    } else if t1 == TypeTag::String || t2 == TypeTag::String {
                        TypeTag::String
                    } else {
                        TypeTag::Unknown
                    }
                } else if is_sub {
                    if t1 == TypeTag::Date && t2 == TypeTag::Int {
                        TypeTag::Date
                    } else if t1 == TypeTag::Date && t2 == TypeTag::Date {
                        TypeTag::Int
                    } else if t1 == TypeTag::Int && t2 == TypeTag::Int {
                        TypeTag::Int
                    } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                        TypeTag::Float
                    } else if t1 == TypeTag::String || t2 == TypeTag::String {
                        TypeTag::String
                    } else {
                        TypeTag::Unknown
                    }
                } else {
                    if t1 == TypeTag::Int && t2 == TypeTag::Int {
                        TypeTag::Int
                    } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                        TypeTag::Float
                    } else {
                        TypeTag::Unknown
                    }
                };
                out_types[*dst as usize] = ty;
            }
            OpCode::Equal { dst, .. } | OpCode::NotEqual { dst, .. } |
            OpCode::Greater { dst, .. } | OpCode::Less { dst, .. } |
            OpCode::GreaterEqual { dst, .. } | OpCode::LessEqual { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Bool;
            }
            OpCode::IntConcat { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Int;
            }
            OpCode::Move { dst, src } => {
                out_types[*dst as usize] = out_types[*src as usize];
            }
            OpCode::Neg { dst, src } => {
                out_types[*dst as usize] = out_types[*src as usize];
            }
            OpCode::Not { dst, src: _ } => {
                out_types[*dst as usize] = TypeTag::Bool;
            }
            OpCode::DateNow { dst } => {
                out_types[*dst as usize] = TypeTag::Date;
            }
            OpCode::PerfMs { dst } | OpCode::PerfUs { dst } | OpCode::PerfNs { dst } => {
                out_types[*dst as usize] = TypeTag::Int;
            }
            OpCode::ArrayInit { dst, .. } => {
                if bool_array_hints.contains(dst) {
                    out_types[*dst as usize] = TypeTag::BoolArray;
                } else {
                    out_types[*dst as usize] = TypeTag::Array;
                }
            }
            OpCode::BoolArrayInit { dst } => {
                out_types[*dst as usize] = TypeTag::BoolArray;
            }
            OpCode::SetInit { dst, .. } |
            OpCode::SetRange { dst, .. } |
            OpCode::SetUnion { dst, .. } |
            OpCode::SetIntersection { dst, .. } |
            OpCode::SetDifference { dst, .. } |
            OpCode::SetSymDifference { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Set;
            }
            OpCode::TableInit { dst, .. } | OpCode::TableCloneSkeleton { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Table;
            }
            OpCode::MapInit { dst, .. } => { out_types[*dst as usize] = TypeTag::Map; }
            OpCode::JsonParse { dst, .. } => { out_types[*dst as usize] = TypeTag::Json; }
            OpCode::MethodCall { dst, kind, base, .. } |
            OpCode::MethodCallNamed { dst, kind, base, .. } => {
               match kind {
                    crate::vm::opcode::MethodKind::Len | crate::vm::opcode::MethodKind::Size | crate::vm::opcode::MethodKind::Count | crate::vm::opcode::MethodKind::Find => {
                        out_types[*dst as usize] = TypeTag::Int;
                    }
                    crate::vm::opcode::MethodKind::Push | crate::vm::opcode::MethodKind::Set | crate::vm::opcode::MethodKind::Update => {
                        out_types[*dst as usize] = TypeTag::Bool;
                    }
                    crate::vm::opcode::MethodKind::Get if out_types[*base as usize] == TypeTag::Json => {
                        out_types[*dst as usize] = TypeTag::Json;
                    }
                    crate::vm::opcode::MethodKind::Get if out_types[*base as usize] == TypeTag::BoolArray => {
                        out_types[*dst as usize] = TypeTag::Bool;
                    }
                    crate::vm::opcode::MethodKind::Values if out_types[*base as usize] == TypeTag::Set => {
                        out_types[*dst as usize] = TypeTag::Array;
                    }
                    crate::vm::opcode::MethodKind::Where | crate::vm::opcode::MethodKind::Join => {
                        out_types[*dst as usize] = TypeTag::Table;
                    }
                    _ => { out_types[*dst as usize] = TypeTag::Unknown; }
               }
            }
            OpCode::Call { dst, func_idx, .. } => {
                if *func_idx == self_func_idx && out_types[0] == TypeTag::Int {
                    out_types[*dst as usize] = TypeTag::Int;
                } else {
                    out_types[*dst as usize] = TypeTag::Unknown;
                }
            }
            OpCode::Typeof { dst, .. } => { out_types[*dst as usize] = TypeTag::String; }
            OpCode::GetIndex { dst, container, .. } => {
                let container_ty = out_types[*container as usize];
                if container_ty == TypeTag::BoolArray {
                    out_types[*dst as usize] = TypeTag::Bool;
                } else if container_ty == TypeTag::Table {
                    out_types[*dst as usize] = TypeTag::Row;
                } else {
                    out_types[*dst as usize] = TypeTag::Unknown;
                }
            }
            OpCode::GetMember { dst, .. } | OpCode::MethodCallCustom { dst, .. } |
            OpCode::RowGet { dst, .. } | OpCode::YieldWithTarget { dst, .. } |
            OpCode::HttpCall { dst, .. } | OpCode::HttpRequest { dst, .. } |
            OpCode::JsonBindLocal { dst, .. } | OpCode::JsonInjectLocal { table_reg: dst, .. } |
            OpCode::StoreRead { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Unknown;
            }
            OpCode::FiberCreate { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Fiber;
            }
            OpCode::EnvGet { dst, .. } | OpCode::CryptoHash { dst, .. } |
            OpCode::CryptoToken { dst, .. } => {
                out_types[*dst as usize] = TypeTag::String;
            }
            OpCode::EnvArgs { dst } | OpCode::StoreList { dst, .. } |
            OpCode::StoreGlob { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Array;
            }
            OpCode::CryptoVerify { dst, .. } | OpCode::StoreExists { dst, .. } |
            OpCode::StoreIsDir { dst, .. } | OpCode::HttpRespond { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Bool;
            }
            OpCode::StoreSize { dst, .. } => {
                out_types[*dst as usize] = TypeTag::Int;
            }
            OpCode::GetVar { dst, idx } => {
                let ty = *global_types.get(idx).unwrap_or(&TypeTag::Unknown);
                out_types[*dst as usize] = ty;
            }
            OpCode::SetVar { idx, src } => {
                let ty = out_types[*src as usize];
                global_types.insert(*idx, ty);
            }
            OpCode::IncVar { idx } | OpCode::DecVar { idx } => {
                global_types.insert(*idx, TypeTag::Int);
            }
            _ => {}
        }
        
        // Track heap usage
        for t in out_types.iter() {
            match t {
                TypeTag::Int | TypeTag::Float | TypeTag::Bool | TypeTag::Unknown | TypeTag::BoolArray => {},
                _ => { uses_heap = true; break; }
            }
        }

        // Propagate to successors
        let mut successors = Vec::new();
        if let Some(target) = op.jump_target() {
            successors.push(target as usize);
            if !op.is_unconditional_jump() {
                successors.push(ip + 1);
            }
        } else if !op.is_return() && !op.is_halt() {
            successors.push(ip + 1);
        }

        for succ in successors {
            if succ >= bytecode.len() { continue; }
            let mut changed = false;
            if !visited[succ] {
                visited[succ] = true;
                types_at_ip[succ] = out_types;
                changed = true;
            } else {
                for r in 0..256 {
                    let in_t = types_at_ip[succ][r];
                    let out_t = out_types[r];
                    if in_t != out_t {
                        if in_t != TypeTag::Unknown {
                            types_at_ip[succ][r] = TypeTag::Unknown;
                            changed = true;
                        }
                    }
                }
            }
            if changed && !in_worklist[succ] {
                worklist.push_back(succ);
                in_worklist[succ] = true;
            }
        }
    }

    (types_at_ip, uses_heap)
}

