use crate::error::Span;
use super::error_kind::TypeErrorKind;

#[derive(Debug, PartialEq)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}
