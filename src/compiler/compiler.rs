use crate::frontend::ast::Stmt;
use crate::vm::opcode::{OpCode, MethodKind};
use crate::vm::value::Value;
use crate::vm::opcode::Chunk;
use crate::vm::object::StringObj;
use crate::intern::{Interner, StringId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::mapping;
use super::defaults;
use super::globals;

const BUILT_INS: [&str; 5] = ["json", "date", "random", "store", "input"];

pub struct Compiler {
    pub globals: HashMap<StringId, usize>,
    pub global_types: HashMap<StringId, crate::frontend::ast::Type>,
    pub func_indices: HashMap<StringId, usize>,
    pub functions: Vec<Arc<Chunk>>,
    pub constants: Vec<Value>,
    pub string_constants: HashMap<Vec<u8>, usize>,
    pub numeric_constants: HashMap<(u64, u64), usize>,
    pub disable_inline: bool,
}

pub struct CompileContext<'a> {
    pub constants: &'a mut Vec<Value>,
    pub string_constants: &'a mut HashMap<Vec<u8>, usize>,
    pub numeric_constants: &'a mut HashMap<(u64, u64), usize>,
    pub functions: &'a mut Vec<Arc<Chunk>>,
    pub func_indices: &'a HashMap<StringId, usize>,
    pub globals: &'a HashMap<StringId, usize>,
    pub global_types: &'a HashMap<StringId, crate::frontend::ast::Type>,
    pub interner: &'a mut Interner,
}

#[derive(Debug, Clone)]
pub struct LoopFrame {
    pub start_pc: usize,
    pub breaks: Vec<usize>,
    pub continues: Vec<usize>,
    pub fiber_reg: Option<usize>,
}

pub struct FunctionCompiler {
    pub bytecode: Vec<OpCode>,
    pub spans: Vec<crate::error::Span>,
    pub scopes: Vec<HashMap<StringId, usize>>,
    pub next_local: usize,
    pub loop_stack: Vec<LoopFrame>,
    pub parent_locals: Option<HashMap<StringId, usize>>,
    pub local_types: HashMap<StringId, crate::frontend::ast::Type>,
    pub captures: Vec<StringId>,
    pub is_main: bool,
    pub is_table_lambda: bool,
    pub max_locals_used: usize,
    pub inline_stack: Vec<usize>,
    pub inline_result_locals: Vec<Option<u8>>,
    pub local_regs: HashSet<usize>,
}

impl FunctionCompiler {
    pub fn new(is_main: bool, parent_locals: Option<HashMap<StringId, usize>>) -> Self {
        Self {
            bytecode: Vec::new(),
            spans: Vec::new(),
            scopes: vec![HashMap::new()],
            next_local: 0,
            max_locals_used: 0,
            loop_stack: Vec::new(),
            parent_locals,
            local_types: HashMap::new(),
            captures: Vec::new(),
            is_main,
            is_table_lambda: false,
            inline_stack: Vec::new(),
            inline_result_locals: Vec::new(),
            local_regs: HashSet::new(),
        }
    }

    pub fn map_method_kind(&self, name: &str) -> Option<MethodKind> {
        mapping::map_method_kind(name)
    }

    pub fn get_default_value(&self, ty: &crate::frontend::ast::Type, ctx: &mut CompileContext) -> Value {
        defaults::get_default_value(ty, ctx)
    }

    pub fn sync_max_locals(&mut self) {
        if self.next_local > self.max_locals_used {
            self.max_locals_used = self.next_local;
        }
    }

    pub fn push_reg(&mut self) -> u8 {
        let reg = self.next_local as u8;
        self.next_local += 1;
        self.sync_max_locals();
        reg
    }

    pub fn pop_reg(&mut self) {
        if self.next_local > 0 {
            let reg_to_pop = self.next_local - 1;
            let is_local = self.scopes.iter().any(|scope| {
                scope.values().any(|&slot| slot == reg_to_pop)
            });
            if !is_local {
                self.next_local -= 1;
            }
        }
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            global_types: HashMap::new(),
            func_indices: HashMap::new(),
            functions: Vec::new(),
            constants: Vec::new(),
            string_constants: HashMap::new(),
            numeric_constants: HashMap::new(),
            disable_inline: false,
        }
    }

    pub fn compile(&mut self, program: &crate::frontend::ast::Program, interner: &mut Interner) -> (Chunk, Arc<Vec<Value>>, Arc<Vec<Arc<Chunk>>>) {
        let mut main_compiler = FunctionCompiler::new(true, None);
        for (i, name) in BUILT_INS.iter().enumerate() {
            let id = interner.intern(name);
            self.globals.insert(id, i);
            
            let val = Value::from_string(Arc::new(StringObj::new(name.to_string().into_bytes())));
            let key = name.to_string().into_bytes();
            let val_idx = if let Some(&idx) = self.string_constants.get(&key) {
                idx as u32
            } else {
                let idx = self.constants.len();
                self.string_constants.insert(key, idx);
                self.constants.push(val);
                idx as u32
            };
            let dst = main_compiler.push_reg();
            main_compiler.emit(OpCode::LoadConst { dst, idx: val_idx }, &crate::error::Span::default());
            main_compiler.emit(OpCode::SetVar { idx: i as u32, src: dst }, &crate::error::Span::default());
            main_compiler.pop_reg();
        }
        globals::register_globals_recursive(&program.stmts, &mut self.globals, &mut self.func_indices, &mut self.functions, &mut self.global_types, true);
        
        let mut ctx = CompileContext {
            constants: &mut self.constants,
            string_constants: &mut self.string_constants,
            numeric_constants: &mut self.numeric_constants,
            functions: &mut self.functions,
            func_indices: &self.func_indices,
            globals: &self.globals,
            global_types: &self.global_types,
            interner,
        };

        
        let mut hir_funcs = crate::hir::lower_program(program, &self.func_indices, &self.globals);
        if !self.disable_inline {
            crate::hir::run_inliner_pass(&mut hir_funcs);
        }

        for stmt in &program.stmts {
            match &stmt.kind {
                crate::frontend::ast::StmtKind::FunctionDef { name, params, body, .. } => {
                    let fid = *self.func_indices.get(name).unwrap();
                    let name_str = ctx.interner.lookup(*name).to_string();
                    if let Some(hir) = hir_funcs.get(&(fid as u32)) {
                        let chunk = crate::hir::compile_hir_to_chunk(hir, false, &mut ctx, name_str, params.len());
                        ctx.functions[fid] = Arc::new(chunk);
                    } else {
                        let chunk = compile_function_helper(params, body, false, &mut ctx, name_str);
                        ctx.functions[fid] = Arc::new(chunk);
                    }
                }
                crate::frontend::ast::StmtKind::FiberDef { name, params, body, .. } => {
                    let fid = *self.func_indices.get(name).unwrap();
                    let name_str = ctx.interner.lookup(*name).to_string();
                    if let Some(hir) = hir_funcs.get(&(fid as u32)) {
                        let chunk = crate::hir::compile_hir_to_chunk(hir, true, &mut ctx, name_str, params.len());
                        ctx.functions[fid] = Arc::new(chunk);
                    } else {
                        let chunk = compile_function_helper(params, body, true, &mut ctx, name_str);
                        ctx.functions[fid] = Arc::new(chunk);
                    }
                }
                _ => main_compiler.compile_stmt(stmt, &mut ctx),
            }
        }
        
        let has_loops = crate::vm::opcode::calculate_has_loops(&main_compiler.bytecode);
        
        main_compiler.max_locals_used = main_compiler.max_locals_used.max(main_compiler.next_local);
        let main_chunk = Chunk::new(main_compiler.bytecode, main_compiler.spans, false, main_compiler.max_locals_used, has_loops, "main".to_string(), 0);
        
        self.string_constants.clear();
        self.numeric_constants.clear();
        self.func_indices.clear();
        
        (main_chunk, Arc::new(std::mem::take(&mut self.constants)), Arc::new(std::mem::take(&mut self.functions)))
    }
}

pub fn compile_function_helper(
    params: &[(crate::frontend::ast::Type, StringId)],
    body: &[Stmt],
    is_fiber: bool,
    ctx: &mut CompileContext,
    name: String,
) -> Chunk {
    let mut compiler = FunctionCompiler::new(false, None);
    for (i, (_, param_name)) in params.iter().enumerate() {
        compiler.define_local(*param_name, i);
    }
    compiler.next_local = params.len();
    for s in body {
        compiler.compile_stmt(s, ctx);
    }
    if !compiler.bytecode.last().map_or(false, |op| {
        matches!(op, OpCode::Return { .. } | OpCode::ReturnVoid)
    }) {
        compiler.emit(OpCode::ReturnVoid, &crate::error::Span::default());
    }
    let has_loops = crate::vm::opcode::calculate_has_loops(&compiler.bytecode);

    let max_locals = compiler.max_locals_used.max(compiler.next_local);

    Chunk::new(compiler.bytecode, compiler.spans, is_fiber, max_locals.into(), has_loops, name, params.len())
}
