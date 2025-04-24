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
mod data;
mod utils;

use log::warn;
use mongodb::bson::{self, doc, Document};
use mongodb::sync::{Client, Collection};
use toml::Table;
use utils::{initialize_logging, Args};

use rustc_driver::{run_compiler, Callbacks};

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};

use analysis::Analyzer;

struct MongoConfig {
    uri: String,
    db: String,
}

struct AnalysisCallback {
    mongo_config: Option<MongoConfig>,
    output_dir: Option<PathBuf>,
    workspace: Option<String>,
    crate_name: String,
}

impl AnalysisCallback {
    fn output_json(&self, analyzer: &Analyzer) {
        let Some(mut out_dir) = self.output_dir.clone() else {
            warn!("Output (JSON) configuration was not provided!");
            return;
        };

        if let Some(ns) = &self.workspace {
            out_dir.push(ns);
        }

        out_dir.push(&self.crate_name);

        let fn_out_path = out_dir.join("functions.json");
        let invoc_out_path = out_dir.join("invocations.json");

        fs::create_dir_all(&out_dir).expect(&format!("Failed to create {:?}", out_dir));

        let fn_out_string =
            serde_json::to_string_pretty(&analyzer.fns).expect("Failed to serialize function map");

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

    fn output_mongodb(&self, analyzer: &Analyzer) {
        let Some(config) = &self.mongo_config else {
            warn!("Output (MongoDB) configuration was not provided!");
            return;
        };

        let client = Client::with_uri_str(&config.uri).unwrap();
        let db = client.database(&config.db);

        let crates_collection: Collection<Document> = db.collection("crates");
        let fns_collection: Collection<Document> = db.collection("functions");
        let invocs_collection: Collection<Document> = db.collection("invocations");

        // Upsert this crate to the `crates` collection
        let query = doc! {
            "workspace": self.workspace.clone(),
            "name": self.crate_name.clone()
        };

        let update = doc! {
            "$set": query.clone()
        };

        let _ = crates_collection
            .update_one(query, update)
            .upsert(true)
            .run();

        // Upsert every function we encountered to the `functions` collection
        for (fn_name, fn_data) in &analyzer.fns {
            let mut query = bson::to_document(fn_data).unwrap();
            query.insert("name", fn_name);

            let update = doc! {
                "$set": query.clone()
            };

            let _ = fns_collection.update_one(query, update).upsert(true).run();
        }

        // Upsert every invocation we encountered to the `invocations` collection
        for invoc in &analyzer.invocs {
            let query = bson::to_document(invoc).unwrap();
            let update = doc! {
                "$set": query.clone()
            };

            let _ = invocs_collection
                .update_one(query, update)
                .upsert(true)
                .run();
        }
    }
}

impl Callbacks for AnalysisCallback {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: rustc_middle::ty::TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        let skip = ["tokio", "time", "rustix", "parquet"];

        if !skip.contains(&self.crate_name.as_ref()) {
            let mut analyzer = Analyzer::new(tcx, self.workspace.clone(), self.crate_name.clone());

            tcx.hir_visit_all_item_likes_in_crate(&mut analyzer);

            self.output_json(&analyzer);
            self.output_mongodb(&analyzer);
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

    let workspace = if let Ok(true) = fs::exists("scampi.toml") {
        let raw = fs::read_to_string("scampi.toml").expect("Could not read Cargo manifest");
        let table = raw.parse::<Table>().unwrap();

        if let Some(entry) = table.get("workspace") {
            if let Some(workspace) = entry.as_str() {
                Some(String::from(workspace))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Initialize logging
    initialize_logging(&crate_name);

    let uri = std::env::var("SCAMPI_MONGO_URI");
    let db = std::env::var("SCAMPI_MONGO_DB");
    let out = std::env::var("SCAMPI_OUT_DIR");

    let mongo_config = match (uri, db) {
        (Ok(uri), Ok(db)) => Some(MongoConfig { uri, db }),
        _ => None,
    };

    // Run the compiler
    let mut callbacks = AnalysisCallback {
        mongo_config,
        output_dir: out.map_or(None, |out| Some(PathBuf::from(out))),
        workspace,
        crate_name,
    };

    let _ = run_compiler(&raw_args, &mut callbacks);
}
