use std::collections::HashMap;
use crate::frontend::ast::{Stmt, StmtKind};
use crate::intern::StringId;
use crate::vm::opcode::Chunk;
use std::sync::Arc;

pub(crate) fn register_globals_recursive(
    stmts: &[Stmt],
    globals: &mut HashMap<StringId, usize>,
    func_indices: &mut HashMap<StringId, usize>,
    functions: &mut Vec<Arc<Chunk>>,
    is_main_script: bool,
) {
    fn register_global(globals: &mut HashMap<StringId, usize>, name: StringId) {
        if !globals.contains_key(&name) {
            let idx = globals.len();
            globals.insert(name, idx);
        }
    }

    for stmt in stmts {
        match &stmt.kind {
            StmtKind::FunctionDef { name, body, .. } => {
                let idx = functions.len();
                func_indices.insert(*name, idx);
                functions.push(Arc::new(Chunk::new(Vec::new(), Vec::new(), false, 0, false, "skeleton".to_string(), 0)));
                register_globals_recursive(body, globals, func_indices, functions, false);
            }
            StmtKind::FiberDef { name, body, .. } => {
                let idx = functions.len();
                func_indices.insert(*name, idx);
                functions.push(Arc::new(Chunk::new(Vec::new(), Vec::new(), true, 0, false, "skeleton".to_string(), 0)));
                register_globals_recursive(body, globals, func_indices, functions, false);
            }
            StmtKind::VarDecl { name, .. } if is_main_script => {
                register_global(globals, *name);
            }
            StmtKind::FiberDecl { name, .. } if is_main_script => {
                register_global(globals, *name);
            }
            StmtKind::DatabaseDecl { name, .. } if is_main_script => {
                register_global(globals, *name);
            }
            StmtKind::If { then_branch, else_ifs, else_branch, .. } => {
                register_globals_recursive(then_branch, globals, func_indices, functions, false);
                for (_, elif_branch) in else_ifs {
                    register_globals_recursive(elif_branch, globals, func_indices, functions, false);
                }
                if let Some(eb) = else_branch {
                    register_globals_recursive(eb, globals, func_indices, functions, false);
                }
            }
            StmtKind::While { body, .. } => {
                register_globals_recursive(body, globals, func_indices, functions, false);
            }
            StmtKind::For { body, .. } => {
                register_globals_recursive(body, globals, func_indices, functions, false);
            }
            _ => {}
        }
    }
}
