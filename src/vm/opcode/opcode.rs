#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodKind {
    Push, Pop, Len, Count, Size, IsEmpty, Clear, Contains, Get, Insert, Update, Delete, Find, Join, Show, Sort, Reverse,
    Add, Remove, Has, Length, Upper, Lower, Trim, IndexOf, LastIndexOf, Replace, Slice, Split, StartsWith, EndsWith,
    ToInt, ToFloat, Set, Keys, Values, Where, Year, Month, Day, Hour, Minute, Second, Format, Exists, Append, Inject, ToStr, ToJson,
    Next, Run, IsDone, Close, Begin, Commit, Rollback, Query, QueryRaw, Sync, Drop, Fetch, Save, Truncate, Exec, IsOpen, First,
    Key, Ready, Put, Stringify, Start, Result, Table, Execute, Status, Ms,
}

impl MethodKind {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Push), 1 => Some(Self::Pop), 2 => Some(Self::Len), 3 => Some(Self::Count), 4 => Some(Self::Size),
            5 => Some(Self::IsEmpty), 6 => Some(Self::Clear), 7 => Some(Self::Contains), 8 => Some(Self::Get), 9 => Some(Self::Insert),
            10 => Some(Self::Update), 11 => Some(Self::Delete), 12 => Some(Self::Find), 13 => Some(Self::Join), 14 => Some(Self::Show),
            15 => Some(Self::Sort), 16 => Some(Self::Reverse),
            17 => Some(Self::Add), 18 => Some(Self::Remove), 19 => Some(Self::Has), 20 => Some(Self::Length), 21 => Some(Self::Upper),
            22 => Some(Self::Lower), 23 => Some(Self::Trim), 24 => Some(Self::IndexOf), 25 => Some(Self::LastIndexOf), 26 => Some(Self::Replace),
            27 => Some(Self::Slice), 28 => Some(Self::Split), 29 => Some(Self::StartsWith), 30 => Some(Self::EndsWith),
            31 => Some(Self::ToInt), 32 => Some(Self::ToFloat), 33 => Some(Self::Set), 34 => Some(Self::Keys), 35 => Some(Self::Values),
            36 => Some(Self::Where), 37 => Some(Self::Year), 38 => Some(Self::Month), 39 => Some(Self::Day), 40 => Some(Self::Hour),
            41 => Some(Self::Minute), 42 => Some(Self::Second), 43 => Some(Self::Format), 44 => Some(Self::Exists), 45 => Some(Self::Append),
            46 => Some(Self::Inject), 47 => Some(Self::ToStr), 48 => Some(Self::ToJson), 49 => Some(Self::Next), 50 => Some(Self::Run),
            51 => Some(Self::IsDone), 52 => Some(Self::Close), 53 => Some(Self::Begin), 54 => Some(Self::Commit), 55 => Some(Self::Rollback),
            56 => Some(Self::Query), 57 => Some(Self::QueryRaw), 58 => Some(Self::Sync), 59 => Some(Self::Drop), 60 => Some(Self::Fetch),
            61 => Some(Self::Save), 62 => Some(Self::Truncate), 63 => Some(Self::Exec), 64 => Some(Self::IsOpen), 65 => Some(Self::First),
            66 => Some(Self::Key), 67 => Some(Self::Ready), 68 => Some(Self::Put), 69 => Some(Self::Stringify), 70 => Some(Self::Start),
            71 => Some(Self::Result), 72 => Some(Self::Table), 73 => Some(Self::Execute), 74 => Some(Self::Status), 75 => Some(Self::Ms),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeTag {
    Int,
    Float,
    String,
    Bool,
    Date,
    Array,
    BoolArray,
    Set,
    Map,
    Table,
    Function,
    Row,
    Json,
    Fiber,
    Database,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    Move { dst: u8, src: u8 },
    LoadConst { dst: u8, idx: u32 },
    
    Add { dst: u8, src1: u8, src2: u8 },
    Sub { dst: u8, src1: u8, src2: u8 },
    Mul { dst: u8, src1: u8, src2: u8 },
    Div { dst: u8, src1: u8, src2: u8 },
    Mod { dst: u8, src1: u8, src2: u8 },
    Pow { dst: u8, src1: u8, src2: u8 },
    
    Equal { dst: u8, src1: u8, src2: u8 },
    NotEqual { dst: u8, src1: u8, src2: u8 },
    Greater { dst: u8, src1: u8, src2: u8 },
    Less { dst: u8, src1: u8, src2: u8 },
    GreaterEqual { dst: u8, src1: u8, src2: u8 },
    LessEqual { dst: u8, src1: u8, src2: u8 },
    
    And { dst: u8, src1: u8, src2: u8 },
    Or { dst: u8, src1: u8, src2: u8 },
    Not { dst: u8, src: u8 },
    Has { dst: u8, src1: u8, src2: u8 },
    
    GetVar { dst: u8, idx: u32 },
    SetVar { idx: u32, src: u8 },

    Jump { target: u32 },
    JumpIfFalse { src: u8, target: u32 },
    JumpIfTrue { src: u8, target: u32 },
    
    Print { src: u8 },
    Input { dst: u8, ty: TypeTag },
    HaltAlert { src: u8 }, 
    HaltError { src: u8 }, 
    HaltFatal { src: u8 },
    TerminalExit { dst: u8 },
    TerminalRun { dst: u8, cmd_src: u8 },

    TerminalClear { dst: u8 },
    TerminalRaw { dst: u8 },
    TerminalNormal { dst: u8 },
    TerminalCursor { dst: u8, on: bool },
    TerminalMove { dst: u8, x_src: u8, y_src: u8 },
    TerminalWrite { dst: u8, src: u8 },

    InputKey { dst: u8 },
    InputKeyWait { dst: u8 },
    InputReady { dst: u8 },

    
    Call { dst: u8, func_idx: u32, base: u8, arg_count: u8 },
    Return { src: u8 },
    ReturnVoid,
    Halt,

    SetName { src: u8, name_idx: u32 },

    // Collections
    ArrayInit { dst: u8, base: u8, count: u32 },
    BoolArrayInit { dst: u8 },
    SetInit { dst: u8, base: u8, count: u32 },
    MapInit { dst: u8, base: u8, count: u32 },
    TableInit { dst: u8, skeleton_idx: u32, base: u8, row_count: u32, col_count: u32 },
    
    MethodCall { dst: u8, kind: MethodKind, base: u8, arg_count: u8 },
    MethodCallCustom { dst: u8, method_name_idx: u32, base: u8, arg_count: u8 },

    SetUnion { dst: u8, src1: u8, src2: u8 },
    SetIntersection { dst: u8, src1: u8, src2: u8 },
    SetDifference { dst: u8, src1: u8, src2: u8 },
    SetSymDifference { dst: u8, src1: u8, src2: u8 },
    RandomChoice { dst: u8, src: u8 },
    IntConcat { dst: u8, src1: u8, src2: u8 },
    SetRange { dst: u8, start: u8, end: u8, step: u8, has_step: u8 },
    RandomInt { dst: u8, min: u8, max: u8, step: u8, has_step: u8 },
    RandomFloat { dst: u8, min: u8, max: u8, step: u8, has_step: u8 },

    // Store operations
    StoreWrite { dst: u8, base: u8 }, 
    StoreRead { dst: u8, base: u8 },
    StoreAppend { dst: u8, base: u8 },
    StoreExists { dst: u8, base: u8 },
    StoreDelete { dst: u8, base: u8 },
    StoreList { dst: u8, base: u8 },
    StoreIsDir { dst: u8, base: u8 },
    StoreSize { dst: u8, base: u8 },
    StoreMkdir { dst: u8, base: u8 },
    StoreGlob { dst: u8, base: u8 },
    StoreZip { dst: u8, base: u8 },
    StoreUnzip { dst: u8, base: u8 },

    // JSON/Date
    JsonParse { dst: u8, src: u8 },
    DateNow { dst: u8 },
    PerfMs { dst: u8 },
    PerfUs { dst: u8 },
    PerfNs { dst: u8 },
    JsonBind { idx: u32, json_src: u8, path_src: u8 },
    JsonBindLocal { dst: u8, json_src: u8, path_src: u8 },
    JsonInject { table_idx: u32, json_src: u8, mapping_src: u8 },
    JsonInjectLocal { table_reg: u8, json_src: u8, mapping_src: u8 },
    JsonFastGetPush { json_src: u8, path_src: u8, val_src: u8 },

    // Fibers/Concurrency
    FiberCreate { dst: u8, func_idx: u32, base: u8, arg_count: u8 },
    Yield { src: u8 },
    YieldWithTarget { dst: u8, src: u8 },
    YieldVoid,
    Wait { src: u8 },

    // HTTP
    HttpCall { dst: u8, method_idx: u32, url_src: u8, body_src: u8 },
    HttpRequest { dst: u8, arg_src: u8 },
    HttpRespond { dst: u8, status_src: u8, body_src: u8, headers_src: u8 },
    HttpServe { func_idx: u32, port_src: u8, host_src: u8, workers_src: u8, routes_src: u8 },

    // Misc and Casts
    EnvGet { dst: u8, src: u8 },
    EnvArgs { dst: u8 },
    CryptoHash { dst: u8, pass_src: u8, alg_src: u8 },
    CryptoVerify { dst: u8, pass_src: u8, hash_src: u8, alg_src: u8 },
    CryptoToken { dst: u8, len_src: u8 },

    CastInt { dst: u8, src: u8 },
    CastFloat { dst: u8, src: u8 },
    CastString { dst: u8, src: u8 },
    CastBool { dst: u8, src: u8 },
    Neg { dst: u8, src: u8 },

    // Optimizations
    IncLocal { reg: u8 },
    DecLocal { reg: u8 },
    LoopNext { reg: u8, limit_reg: u8, target: u32 },
    LoopPrev { reg: u8, limit_reg: u8, target: u32 },
    IncLocalLoopNext { inc_reg: u8, reg: u8, limit_reg: u8, target: u32 },
    DecLocalLoopPrev { dec_reg: u8, reg: u8, limit_reg: u8, target: u32 },
    IncVar { idx: u32 },
    DecVar { idx: u32 },
    IncVarLoopNext { g_idx: u32, reg: u8, limit_reg: u8, target: u32 },
    DecVarLoopPrev { g_idx: u32, reg: u8, limit_reg: u8, target: u32 },
    StrAppendVar { var_idx: u32, src: u8 },
    StrAppendLocal { local_idx: u8, src: u8 },
    ArrayLoopNext { idx_reg: u8, size_reg: u8, target: u32 },
    DatabaseInit { dst: u8, engine_src: u8, path_src: u8, tables_base_reg: u8, table_count: u32 },
    MethodCallNamed { dst: u8, kind: MethodKind, base: u8, arg_count: u8, names_idx: u32 },
    Typeof { dst: u8, src: u8 },

    GetIndex { dst: u8, container: u8, index: u8 },
    SetIndex { container: u8, index: u8, src: u8 },
    GetMember { dst: u8, container: u8, name_idx: u32 },
    SetMember { container: u8, name_idx: u32, src: u8 },
    StrAppendMember { container: u8, name_idx: u32, src: u8 },
    StrAppendElement { container: u8, index: u8, src: u8 },

    RowGet { dst: u8, row_reg: u8, col_idx: u16 },
    TableIter { tbl_reg: u8, idx_reg: u8, row_reg: u8, limit_reg: u8, target: u32 },
    TablePushRow { tbl_reg: u8, row_reg: u8 },
    TableCloneSkeleton { dst: u8, src: u8 },
    TableBegin { dst: u8, skeleton_idx: u32 },
    TableInitRow { tbl_dst: u8, base: u8, col_count: u8 },
}

impl OpCode {
    pub fn jump_target(&self) -> Option<u32> {
        match self {
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
            OpCode::TableIter { target, .. } => Some(*target),
            _ => None,
        }
    }

    pub fn is_unconditional_jump(&self) -> bool {
        match self {
            OpCode::Jump { .. } => true,
            _ => false,
        }
    }
    
    pub fn is_return(&self) -> bool {
        match self {
            OpCode::Return { .. } | OpCode::ReturnVoid => true,
            _ => false,
        }
    }
    
    pub fn is_halt(&self) -> bool {
        match self {
            OpCode::Halt => true,
            _ => false,
        }
    }

    pub fn dst_reg(&self) -> Option<u8> {
        match self {
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
            OpCode::BoolArrayInit { dst } |
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
            OpCode::IncLocal { reg: dst } |
            OpCode::LoopNext { reg: dst, .. } |
            OpCode::IncLocalLoopNext { reg: dst, .. } |
            OpCode::IncVarLoopNext { reg: dst, .. } |
            OpCode::ArrayLoopNext { idx_reg: dst, .. } |
            OpCode::DatabaseInit { dst, .. } => Some(*dst),
            _ => None,
        }
    }

    pub fn src_regs(&self, regs: &mut Vec<u8>) {
        match self {
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
            OpCode::StrAppendVar { src, .. } |
            OpCode::StrAppendLocal { src, .. } => { regs.push(*src); }
            OpCode::ArrayLoopNext { idx_reg, size_reg, .. } => { regs.push(*idx_reg); regs.push(*size_reg); }
            
            OpCode::DatabaseInit { engine_src, path_src, tables_base_reg, .. } => { regs.push(*engine_src); regs.push(*path_src); regs.push(*tables_base_reg); }

            OpCode::GetIndex { container, index, .. } => { regs.push(*container); regs.push(*index); }
            OpCode::SetIndex { container, index, src } |
            OpCode::StrAppendElement { container, index, src } => { regs.push(*container); regs.push(*index); regs.push(*src); }
            OpCode::GetMember { container, .. } => { regs.push(*container); }
            OpCode::SetMember { container, src, .. } |
            OpCode::StrAppendMember { container, src, .. } => { regs.push(*container); regs.push(*src); }
            OpCode::RowGet { row_reg, .. } => { regs.push(*row_reg); }
            OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, .. } => { regs.push(*tbl_reg); regs.push(*idx_reg); regs.push(*row_reg); regs.push(*limit_reg); }
            OpCode::TablePushRow { tbl_reg, row_reg } => { regs.push(*tbl_reg); regs.push(*row_reg); }
            _ => {}
        }
    }
}


