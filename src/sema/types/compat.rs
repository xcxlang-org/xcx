use crate::frontend::ast::SetType;
use super::ty::Type;
use super::primitive;


pub fn is_compatible(expected: &Type, actual: &Type) -> bool {
    if actual == &Type::Unknown || expected == &Type::Unknown || expected == actual { return true; }
    if expected == &Type::Json || actual == &Type::Json { return true; }
    if let (Type::Builtin(id1), Type::Builtin(id2)) = (expected, actual) {
        return id1 == id2;
    }
    match (expected, actual) {
        (e, a) if primitive::is_numeric_compatible(e, a) => true,

        (Type::Array(e), Type::Array(a)) => is_compatible(e, a),
        (Type::Set(st), Type::Array(inner)) | (Type::Array(inner), Type::Set(st)) => {
            let inner_target = match st {
                SetType::N | SetType::Z => Type::Int,
                SetType::Q => Type::Float,
                SetType::S | SetType::C => Type::String,
                SetType::B => Type::Bool,
            };
            &inner_target == inner.as_ref() || inner.as_ref() == &Type::Unknown
        }
        (Type::Set(e_st), Type::Set(a_st)) => {
            let e_base = match e_st {
                SetType::N | SetType::Z => 1,
                SetType::Q => 2,
                SetType::S | SetType::C => 3,
                SetType::B => 4,
            };
            let a_base = match a_st {
                SetType::N | SetType::Z => 1,
                SetType::Q => 2,
                SetType::S | SetType::C => 3,
                SetType::B => 4,
            };
            e_base == a_base
        }
        (Type::Map(ek, ev), Type::Map(ak, av)) => {
            is_compatible(ek, ak) && is_compatible(ev, av)
        }
        (Type::Fiber(et), Type::Fiber(at)) => {
            match (et, at) {
                (None, None) => true,
                (Some(e), Some(a)) => is_compatible(e, a),
                _ => false,
            }
        }
        (Type::Table(e_table), Type::Table(a_table)) => {
            if e_table.is_empty() || a_table.is_empty() { return true; }
            if e_table.len() != a_table.len() { return false; }
            e_table.columns.iter().zip(a_table.columns.iter()).all(|(e, a)| is_compatible(&e.ty, &a.ty))
        }

        _ => false
    }
}
