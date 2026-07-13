use crate::vm::object::{ArrayObj, SetObj, MapObj, TableObj, StringObj, JsonObj};
use crate::vm::value::Value;
use crate::frontend::ast::Type;
use std::sync::Arc;
use parking_lot::RwLock;
use super::compiler::CompileContext;

pub(crate) fn get_default_value(ty: &Type, ctx: &mut CompileContext) -> Value {
    match ty {
        Type::Int => Value::from_i64(0),
        Type::Float => Value::from_f64(0.0),
        Type::String => Value::from_string(Arc::new(StringObj::new(Vec::new()))),
        Type::Bool => Value::from_bool(false),
        Type::Array(inner) => {
            if **inner == Type::Bool {
                Value::from_bool_array(Arc::new(RwLock::new(crate::vm::object::bool_array_obj::BoolArrayObj::new(Vec::new()))))
            } else {
                Value::from_array(Arc::new(RwLock::new(ArrayObj::new(Vec::new()))))
            }
        }
        Type::Set(_) => Value::from_set(Arc::new(RwLock::new(SetObj::new(std::collections::BTreeSet::new())))),
        Type::Map(_, _) => Value::from_map(Arc::new(RwLock::new(MapObj::new(Vec::new())))),
        Type::Date => Value::from_date(0),
        Type::Table(cols) => {
            let vm_cols = cols.iter().map(|c| crate::vm::object::VMColumn {
                name: ctx.interner.lookup(c.name).to_string(),
                ty: c.ty.clone(),
                is_auto: c.is_auto,
                is_pk: c.is_pk,
                is_unique: c.is_unique,
            }).collect();

            Value::from_table(Arc::new(RwLock::new(
                TableObj { 
                    table_name: String::new(), 
                    columns: vm_cols, 
                    rows: Vec::new(), 
                    sql_binding: None, 
                    sql_where: None, 
                    pending_op: None 
                }
            )))
        }
        Type::Json => Value::from_json(Arc::new(JsonObj::new(crate::vm::object::JsonVal::Null))),
        Type::Builtin(_) => Value::from_string(Arc::new(StringObj::new(b"builtin".to_vec()))),
        Type::Unknown => Value::from_i64(0),
        Type::Fiber(_) => Value::from_bool(false),
        Type::Database => Value::from_bool(false),
        Type::DatabaseOperation(_, _) => Value::from_bool(false),
    }
}
