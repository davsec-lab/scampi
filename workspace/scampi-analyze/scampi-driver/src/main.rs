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

mod analysis;
mod utils;
use utils::{initialize_logging, Args};
mod data;

use rustc_driver::{run_compiler, Callbacks};

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use analysis::Analyzer;

struct AnalysisCallback<'a> {
    crate_name: &'a str,
}

impl<'a> Callbacks for AnalysisCallback<'a> {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: rustc_middle::ty::TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        let skip = ["tokio", "time", "rustix", "parquet"];

        if !skip.contains(&self.crate_name) {
            let mut analyzer = Analyzer::new(tcx);

            tcx.hir_visit_all_item_likes_in_crate(&mut analyzer);

            let out_dir = std::env::var("SCAMPI_OUT_DIR").expect("Output directory not provided!");

            let fn_out_path = Path::new(&out_dir)
                .join("functions")
                .join(self.crate_name)
                .with_extension("json");

            let invoc_out_path = Path::new(&out_dir)
                .join("invocations")
                .join(self.crate_name)
                .with_extension("json");

            let fn_out_string = serde_json::to_string_pretty(&analyzer.fns)
                .expect("Failed to serialize function map!");

            let invoc_out_string = serde_json::to_string_pretty(&analyzer.invocs)
                .expect("Failed to serialize invocation list!");

            let mut fn_out_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&fn_out_path)
                .expect("Failed to create function output file!");

            let mut invoc_out_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&invoc_out_path)
                .expect("Failed to create invocation output file!");

            fn_out_file
                .write_all(fn_out_string.as_bytes())
                .expect("Failed to write to function output file!");

            invoc_out_file
                .write_all(invoc_out_string.as_bytes())
                .expect("Failed to write to invocation output file!");
        }

        rustc_driver::Compilation::Continue
    }
}

fn main() {
    // Collect the arguments passed to us by Cargo
    let raw_args: Vec<String> = env::args().skip(1).collect();

    // Parse the arguments into a nicer form
    let args = Args::from_raw(&raw_args);

    // Initialize logging
    initialize_logging(&args);

    // Run the compiler
    let mut callbacks = AnalysisCallback {
        crate_name: &args.name_or("-"),
    };

    let _result = run_compiler(&raw_args, &mut callbacks);
}
