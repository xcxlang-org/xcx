pub mod check;
pub mod symbol;
pub mod types;
pub mod error;

pub use check::checker::Checker;
pub use symbol::symbol_table::SymbolTable;

#[cfg(test)]
mod tests;
