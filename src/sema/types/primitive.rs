use super::ty::Type;

pub fn is_numeric_compatible(ty1: &Type, ty2: &Type) -> bool {
    match (ty1, ty2) {
        (Type::Int, Type::Float) | (Type::Float, Type::Int) | (Type::Int, Type::Date) | (Type::Date, Type::Int) | (Type::Int, Type::Int) | (Type::Float, Type::Float) | (Type::Date, Type::Date) => true,
        _ => false,
    }
}
