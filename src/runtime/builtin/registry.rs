// Builtin Service Registry
// This module provides the central registration for all VM-provided methods.


pub mod json {
    pub use crate::runtime::builtin::json::access::*;
    pub use crate::runtime::builtin::json::inject::*;
}

pub mod math {
    pub use crate::runtime::builtin::math::random::*;
    pub use crate::runtime::builtin::math::pow::*;
}
pub mod string { pub use crate::runtime::builtin::string::*; }
pub mod io {
    pub use crate::runtime::builtin::io::print::*;
    pub use crate::runtime::builtin::io::input::*;
    pub use crate::runtime::builtin::io::terminal::*;
}
pub mod net {
    pub use crate::runtime::builtin::net::client::*;
    pub use crate::runtime::builtin::net::server::*;
    pub use crate::runtime::builtin::net::respond::*;
}
pub mod crypto {
    pub use crate::runtime::builtin::crypto::*;
}
pub mod store {
    pub use crate::runtime::builtin::store::read_write::*;
    pub use crate::runtime::builtin::store::fs_ops::*;
}


// Internal engine helpers


