//! Parity suite: the same program must produce identical stdout (and exit
//! status) in JIT mode and `--no-jit` mode. A divergence is a codegen bug
//! by definition — see tests/parity/lang_parity.rs for the programs.

#[path = "../common/mod.rs"]
mod common;

mod lang_parity;
