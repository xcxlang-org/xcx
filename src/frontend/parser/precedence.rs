// Binding power levels for the XCX Pratt parser.
// The token-to-level mapping lives in Parser::current_precedence (pratt.rs).
#[derive(Debug, PartialOrd, PartialEq, Clone, Copy)]
pub enum Precedence {
    Lowest,
    Lambda,      // ->
    Assignment,  // =
    LogicalOr,   // OR, ||
    LogicalAnd,  // AND, &&
    Equals,      // == !=
    LessGreater, // > < >= <= HAS
    Sum,         // + -
    SetOp,       // UNION, INTERSECTION, DIFFERENCE, SYMMETRIC_DIFFERENCE, ∪, ∩, \, ⊕
    Product,     // * / %
    Power,       // ^
    Prefix,      // -x
    Concatenation, // ::
    Call,        // f(x)
    AsPrec,      // as
}
