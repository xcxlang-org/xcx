use crate::intern::StringId;
use super::expr::Expr;

// Represents a positional or named argument in a function or method call.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    Positional(Expr),
    Named(StringId, Expr),
}

impl Argument {
    // Returns a reference to the expression associated with the argument.
    pub fn expr(&self) -> &Expr {
        match self {
            Argument::Positional(e) => e,
            Argument::Named(_, e) => e,
        }
    }

    // Returns a mutable reference to the expression associated with the argument.
    pub fn expr_mut(&mut self) -> &mut Expr {
        match self {
            Argument::Positional(e) => e,
            Argument::Named(_, e) => e,
        }
    }
}
