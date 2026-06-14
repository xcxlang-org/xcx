use crate::vm::value::Value;

#[derive(Clone, Debug)]
pub enum TraceOp {
    LoadConst { dst: u8, val: Value },
    Move      { dst: u8, src: u8   },

    // Integer arithmetic (all operands must be NaN-boxed ints)
    AddInt { dst: u8, src1: u8, src2: u8 },
    AddFloat { dst: u8, src1: u8, src2: u8 },
    SubInt { dst: u8, src1: u8, src2: u8 },
    SubFloat { dst: u8, src1: u8, src2: u8 },
    MulInt { dst: u8, src1: u8, src2: u8 },
    MulFloat { dst: u8, src1: u8, src2: u8 },
    DivInt { dst: u8, src1: u8, src2: u8, fail_ip: usize },
    DivFloat { dst: u8, src1: u8, src2: u8, fail_ip: usize },
    ModInt { dst: u8, src1: u8, src2: u8, fail_ip: usize },
    ModFloat { dst: u8, src1: u8, src2: u8, fail_ip: usize },
    NegInt { dst: u8, src: u8 },
    NegFloat { dst: u8, src: u8 },

    IncLocal { reg: u8 },
    IncVar   { g_idx: u32 },

    GetVar { dst: u8, idx: u32 },
    SetVar { idx: u32, src: u8 },

    GuardInt   { reg: u8, ip: usize },
    GuardFloat { reg: u8, ip: usize },

    CmpInt   { dst: u8, src1: u8, src2: u8, cc: u8 },
    CmpFloat { dst: u8, src1: u8, src2: u8, cc: u8 },
    
    CastIntToFloat { dst: u8, src: u8 },

    GuardTrue  { reg: u8, fail_ip: usize },
    GuardFalse { reg: u8, fail_ip: usize },

    // Loop control
    LoopNextInt      { reg: u8, limit_reg: u8, target: u32, exit_ip: usize },
    IncVarLoopNext   { g_idx: u32, reg: u8, limit_reg: u8, target: u32, exit_ip: usize },
    IncLocalLoopNext { inc_reg: u8, reg: u8, limit_reg: u8, target: u32, exit_ip: usize },
    ArrayLoopNext    { idx_reg: u8, size_reg: u8, target: u32, exit_ip: usize },

    // Logic ops
    And { dst: u8, src1: u8, src2: u8 },
    Or  { dst: u8, src1: u8, src2: u8 },
    Not { dst: u8, src: u8 },

    Jump { target_ip: usize },

    // Random ops
    RandomInt { dst: u8, min: u8, max: u8, step: u8, has_step: u8 },
    RandomFloat { dst: u8, min: u8, max: u8, step: u8, has_step: u8, step_is_float: bool },

    PowInt   { dst: u8, src1: u8, src2: u8 },
    PowFloat { dst: u8, src1: u8, src2: u8 },
    IntConcat { dst: u8, src1: u8, src2: u8 },
    
    Has          { dst: u8, src1: u8, src2: u8 },
    RandomChoice { dst: u8, src: u8 },

    ArraySize { dst: u8, src: u8 },
    ArrayGet  { dst: u8, arr_reg: u8, idx_reg: u8, fail_ip: usize },
    ArrayPush { arr_reg: u8, val_reg: u8 },

    SetSize     { dst: u8, src: u8 },
    SetContains { dst: u8, set_reg: u8, val_reg: u8 },
    ArrayUpdate { arr_reg: u8, idx_reg: u8, val_reg: u8, fail_ip: usize },

    FiberIsDone { dst: u8, src: u8 },
    FiberNext   { dst: u8, src: u8 },

    Call { dst: u8, func_idx: u32, base: u8, arg_count: u8 },
    
    // Table ops
    RowGet { dst: u8, row_reg: u8, col_idx: u16 },
    TableIter { tbl_reg: u8, idx_reg: u8, row_reg: u8, limit_reg: u8, target: u32, exit_ip: usize },
    TablePushRow { tbl_reg: u8, row_reg: u8 },
    TableCloneSkeleton { dst: u8, src: u8 },
    TableSize { dst: u8, src: u8 },

    // JSON Bind ops
    JsonBindLocal { dst: u8, json_reg: u8, path_reg: u8 },
    JsonBindLocalConst { dst: u8, json_reg: u8, path: String },
    JsonBindGlobal { idx: u32, json_reg: u8, path_reg: u8 },
    JsonBindGlobalConst { idx: u32, json_reg: u8, path: String },

    GetMember { dst: u8, obj_reg: u8, name: String },
    StringLength { dst: u8, src: u8 },
    CastFloatToInt { dst: u8, src: u8 },
    CastBool { dst: u8, src: u8 },
    JsonParse { dst: u8, src: u8 },
    JsonFastGetPush { json_src: u8, path_src: u8, val_src: u8 },
    DateNow { dst: u8 },
    ArrayGetIndex { dst: u8, arr_reg: u8, idx_reg: u8, fail_ip: usize },
    ArraySetIndex { arr_reg: u8, idx_reg: u8, val_reg: u8, fail_ip: usize },
}

impl TraceOp {
    pub fn to_opcode(&self) -> crate::vm::opcode::OpCode {
        use crate::vm::opcode::OpCode as OC;
        match self {
            TraceOp::LoadConst { dst, .. } => OC::LoadConst { dst: *dst, idx: 0 },
            TraceOp::Move { dst, src } => OC::Move { dst: *dst, src: *src },
            TraceOp::GetVar { dst, idx } => OC::GetVar { dst: *dst, idx: *idx },
            TraceOp::SetVar { idx, src } => OC::SetVar { idx: *idx, src: *src },
            TraceOp::IncLocal { reg } => OC::IncLocal { reg: *reg },
            TraceOp::IncVar { g_idx } => OC::IncVar { idx: *g_idx },
            TraceOp::AddInt { dst, src1, src2 } | TraceOp::AddFloat { dst, src1, src2 } => OC::Add { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::SubInt { dst, src1, src2 } | TraceOp::SubFloat { dst, src1, src2 } => OC::Sub { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::MulInt { dst, src1, src2 } | TraceOp::MulFloat { dst, src1, src2 } => OC::Mul { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::DivInt { dst, src1, src2, .. } | TraceOp::DivFloat { dst, src1, src2, .. } => OC::Div { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::ModInt { dst, src1, src2, .. } | TraceOp::ModFloat { dst, src1, src2, .. } => OC::Mod { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::PowInt { dst, src1, src2 } | TraceOp::PowFloat { dst, src1, src2 } => OC::Pow { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::NegInt { dst, src } | TraceOp::NegFloat { dst, src } => OC::Neg { dst: *dst, src: *src },
            TraceOp::CmpInt { dst, src1, src2, .. } | TraceOp::CmpFloat { dst, src1, src2, .. } => OC::Equal { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::IntConcat { dst, src1, src2 } => OC::IntConcat { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::And { dst, src1, src2 } => OC::And { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::Or { dst, src1, src2 } => OC::Or { dst: *dst, src1: *src1, src2: *src2 },
            TraceOp::Not { dst, src } => OC::Not { dst: *dst, src: *src },
            TraceOp::ArraySize { dst, src } => OC::Typeof { dst: *dst, src: *src },
            TraceOp::ArrayGet { dst, arr_reg, idx_reg, .. } => OC::GetIndex { dst: *dst, container: *arr_reg, index: *idx_reg },
            TraceOp::ArrayPush { arr_reg, val_reg: _ } => OC::MethodCall { dst: 0, kind: crate::vm::opcode::MethodKind::Push, base: *arr_reg, arg_count: 1 },
            TraceOp::ArrayUpdate { arr_reg, idx_reg, val_reg, .. } => OC::SetIndex { container: *arr_reg, index: *idx_reg, src: *val_reg },
            TraceOp::GetMember { dst, obj_reg, .. } => OC::GetMember { dst: *dst, container: *obj_reg, name_idx: 0 },
            TraceOp::JsonFastGetPush { json_src, path_src, val_src } => OC::JsonFastGetPush { json_src: *json_src, path_src: *path_src, val_src: *val_src },
            _ => OC::Halt,
        }
    }
}
