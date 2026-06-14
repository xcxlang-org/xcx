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
    ArrayLoopNext { idx_reg: u8, size_reg: u8, target: u32 },
    DatabaseInit { dst: u8, engine_src: u8, path_src: u8, tables_base_reg: u8, table_count: u32 },
    MethodCallNamed { dst: u8, kind: MethodKind, base: u8, arg_count: u8, names_idx: u32 },
    MakeClosure { dst: u8, func_idx: u16, capture_count: u16, capture_start: u8 },
    Typeof { dst: u8, src: u8 },

    GetIndex { dst: u8, container: u8, index: u8 },
    SetIndex { container: u8, index: u8, src: u8 },
    GetMember { dst: u8, container: u8, name_idx: u32 },
    SetMember { container: u8, name_idx: u32, src: u8 },

    RowGet { dst: u8, row_reg: u8, col_idx: u16 },
    TableIter { tbl_reg: u8, idx_reg: u8, row_reg: u8, limit_reg: u8, target: u32 },
    TablePushRow { tbl_reg: u8, row_reg: u8 },
    TableCloneSkeleton { dst: u8, src: u8 },
    TableBegin { dst: u8, skeleton_idx: u32 },
    TableInitRow { tbl_dst: u8, base: u8, col_count: u8 },
}

pub fn calculate_has_loops(bytecode: &[OpCode]) -> bool {
    bytecode.iter().enumerate().any(|(i, op)| {
        match op {
            OpCode::Jump { target } => (*target as usize) < i,
            OpCode::JumpIfFalse { target, .. } => (*target as usize) < i,
            OpCode::JumpIfTrue { target, .. } => (*target as usize) < i,
            OpCode::LoopNext { target, .. } => (*target as usize) < i,
            OpCode::LoopPrev { target, .. } => (*target as usize) < i,
            OpCode::IncLocalLoopNext { target, .. } => (*target as usize) < i,
            OpCode::DecLocalLoopPrev { target, .. } => (*target as usize) < i,
            OpCode::IncVarLoopNext { target, .. } => (*target as usize) < i,
            OpCode::DecVarLoopPrev { target, .. } => (*target as usize) < i,
            OpCode::ArrayLoopNext { target, .. } => (*target as usize) < i,
            OpCode::TableIter { target, .. } => (*target as usize) < i,
            _ => false,
        }
    })
}
