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
use log::debug;
use toml::Table;
use utils::{initialize_logging, Args};
mod data;

use rustc_driver::{run_compiler, Callbacks};

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix;
use std::path::Path;
use std::{env, fs};

use analysis::Analyzer;

struct AnalysisCallback {
    namespace: Option<String>,
    crate_name: String
}

impl Callbacks for AnalysisCallback {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: rustc_middle::ty::TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        let skip = ["tokio", "time", "rustix", "parquet"];

        if !skip.contains(&self.crate_name.as_ref()) {
            let mut analyzer = Analyzer::new(tcx);

            tcx.hir_visit_all_item_likes_in_crate(&mut analyzer);

            let mut out_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("scampi-persist")
                .join("data");

            if let Some(ns) = &self.namespace {
                out_dir.push(ns);
            }

            if let Ok(out) = std::env::var("SCAMPI_OUT_DIR") {
                out_dir.push(out);
            } else if let Ok(true) = fs::exists("scampi.toml") {
                let raw = fs::read_to_string("scampi.toml").expect("Could not read manifest");
                let table = raw.parse::<Table>().unwrap();

                if let Some(out) = table.get("out") {
                    out_dir.push(out.as_str().expect("Output directory must be a string"));
                } else {
                    out_dir.push(&self.crate_name);
                }
            } else {
                out_dir.push(&self.crate_name);
            }

            let fn_out_path = Path::new(&out_dir).join("functions.json");
            let invoc_out_path = Path::new(&out_dir).join("invocations.json");

            fs::create_dir_all(&out_dir).expect(&format!("Failed to create {:?}", out_dir));

            debug!("Crate name: {}", self.crate_name);
            debug!("Namespace: {:?}", self.namespace);
            debug!("Out dir: {:?}", out_dir);
            debug!("Fn out path: {:?}", fn_out_path);
            debug!("Invoc out path: {:?}", invoc_out_path);

            let fn_out_string = serde_json::to_string_pretty(&analyzer.fns)
                .expect("Failed to serialize function map");

            let invoc_out_string = serde_json::to_string_pretty(&analyzer.invocs)
                .expect("Failed to serialize invocation list");

            let mut fn_out_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&fn_out_path)
                .expect(&format!("Failed to create {:?}", fn_out_path));

            let mut invoc_out_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&invoc_out_path)
                .expect(&format!("Failed to create {:?}", invoc_out_path));

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

    let crate_name = args.name_or("-");

    let namespace = if let Ok(true) = fs::exists("scampi.toml") {
        let raw = fs::read_to_string("scampi.toml").expect("Could not read manifest!");
        let table = raw.parse::<Table>().unwrap();

        if let Some(namespace) = table["namespace"].as_str() {
            Some(String::from(namespace))
        } else {
            None
        }
    } else {
        None
    };

    // Initialize logging
    initialize_logging(&namespace, &crate_name);

    // Run the compiler
    let mut callbacks = AnalysisCallback {
        namespace,
        crate_name
    };

    let _result = run_compiler(&raw_args, &mut callbacks);
}
