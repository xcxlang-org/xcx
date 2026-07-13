use crate::frontend::ast::{Program, Stmt, StmtKind, Type};
use crate::intern::StringId;
use super::hir::{HirFunc, HirParam};
use super::lower_stmt::{HirFuncBuilder, lower_stmt};
use std::collections::HashMap;

pub fn lower_func(
    name: StringId,
    params: &[(Type, StringId)],
    return_type: Option<&Type>,
    body: &[Stmt],
    is_fiber: bool,
    func_indices: &HashMap<StringId, usize>,
    globals: &HashMap<StringId, usize>,
) -> HirFunc {
    let mut builder = HirFuncBuilder::new(name, is_fiber);
    builder.return_type = return_type.cloned();

    for (ty, param_name) in params {
        let local = builder.define_local(*param_name, ty.clone(), false);
        builder.params.push(HirParam {
            ty: ty.clone(),
            local,
            name: *param_name,
        });
    }

    let mut body_hir = Vec::new();
    for stmt in body {
        body_hir.extend(lower_stmt(stmt, &mut builder, func_indices, globals));
    }
    builder.body = body_hir;

    HirFunc {
        name: builder.name,
        params: builder.params,
        return_type: builder.return_type,
        body: builder.body,
        locals: builder.locals,
        is_fiber: builder.is_fiber,
    }
}

pub fn lower_program(
    program: &Program,
    func_indices: &HashMap<StringId, usize>,
    globals: &HashMap<StringId, usize>,
) -> HashMap<u32, HirFunc> {
    let mut funcs = HashMap::new();
    for stmt in &program.stmts {
        match &stmt.kind {
            StmtKind::FunctionDef { name, params, return_type, body } => {
                if let Some(&fid) = func_indices.get(name) {
                    let hir = lower_func(*name, params, return_type.as_deref(), body, false, func_indices, globals);
                    funcs.insert(fid as u32, hir);
                }
            }
            StmtKind::FiberDef { name, params, return_type, body } => {
                if let Some(&fid) = func_indices.get(name) {
                    let hir = lower_func(*name, params, return_type.as_deref(), body, true, func_indices, globals);
                    funcs.insert(fid as u32, hir);
                }
            }
            _ => {}
        }
    }
    funcs
}
