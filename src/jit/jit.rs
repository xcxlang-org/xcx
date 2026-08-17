use cranelift::prelude::*;
use cranelift_jit::JITModule;
use cranelift_module::Module;

use super::builder::create_jit_builder;

pub struct JIT {
    pub(crate) module: JITModule,
    pub(crate) ctx: codegen::Context,
    pub(crate) in_progress: std::collections::HashSet<usize>,
}

impl JIT {
    pub fn new() -> Self {
        let builder = create_jit_builder();
        let module = JITModule::new(builder);
        let ctx = module.make_context();

        Self {
            module,
            ctx,
            in_progress: std::collections::HashSet::new(),
        }
    }
}
