#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Parser Errors (100-199)
    E101, // Unexpected token
    E102, // Missing semicolon
    E103, // Malformed expression
    
    // Semantic Errors (200-299)
    E201, // Variable already defined
    E202, // Variable not found
    E203, // Type mismatch
    E204, // Function not found
    
    // VM / Runtime Errors (300-399)
    E301, // Stack overflow (Future use)
    E302, // Invalid OpCode
    E303, // Table Schema mismatch
    E304, // Index out of bounds
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
