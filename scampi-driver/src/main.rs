#![feature(rustc_private)]
#![feature(impl_trait_in_fn_trait_return)]

extern crate rustc_abi;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_type_ir;
extern crate rustc_lint;
extern crate rustc_session;

mod lint;
mod analysis;
mod data;
mod utils;

use rustc_driver::{run_compiler, Callbacks};
use std::env;

use analysis::Analyzer;
use utils::{initialize_logging, Args};

use crate::lint::{RawPtrConstructor, RAW_PTR_IN_CONSTRUCTOR};

struct AnalysisCallback {
    crate_name: String,
}

impl Callbacks for AnalysisCallback {
    fn config(&mut self, _config: &mut rustc_interface::interface::Config) {
        _config.register_lints = Some(Box::new(|_session, lint_store| {
            // This is the new home for your registration logic
            lint_store.register_late_pass(
                |_| Box::new(RawPtrConstructor)
            );
        }));
    }

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: rustc_middle::ty::TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        let skip = ["tokio", "time", "rustix", "parquet", "build_script_build", "find_msvc_tools", "shlex", "cc"];

        if !skip.contains(&self.crate_name.as_ref()) {
            let mut analyzer = Analyzer::new(tcx, self.crate_name.clone());
            tcx.hir_visit_all_item_likes_in_crate(&mut analyzer);
            analyzer.finalize();
        }

        rustc_driver::Compilation::Continue
    }
}

fn main() {
    // Collect the arguments passed to us by Cargo
    let raw_args: Vec<String> = env::args().skip(1).collect();

    // Parse the arguments into a nicer form
    let args = Args::from_raw(&raw_args);

    let crate_name = args.name_or("-");

    // Initialize logging
    initialize_logging(&crate_name);

    // Run the compiler
    let mut callbacks = AnalysisCallback { crate_name };

    run_compiler(&raw_args, &mut callbacks);
}
