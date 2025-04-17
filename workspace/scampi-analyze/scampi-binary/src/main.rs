use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use clap::{Arg, Parser};

#[derive(Parser)]
struct Args {
    /// The JSON output directory.
    #[arg(short, long, value_name = "DIRECTORY")]
    out_dir: Option<String>,

    /// The MongoDB connection string.
    #[arg(short, long, value_name = "INSTANCE URI")]
    uri_mongo: Option<String>,

    /// The MongoDB database.
    #[arg(short, long, value_name = "DATABASE NAME")]
    db_mongo: Option<String>,
}

fn main() {
    let args = Args::parse();

    let mut vars = vec![("SCAMPI_LOG_LEVEL", "DEBUG")];

    if let Some(out_dir) = &args.out_dir {
        vars.push(("SCAMPI_OUT_DIR", out_dir));
    }

    if let Some(uri_mongo) = &args.uri_mongo {
        vars.push(("SCAMPI_MONGO_URI", uri_mongo));
    }

    if let Some(db_mongo) = &args.db_mongo {
        vars.push(("SCAMPI_MONGO_DB", db_mongo));
    }

    let mut command = Command::new("cargo")
        .arg("check")
        .arg("--keep-going")
        .env("RUSTC_WRAPPER", "scampi-driver")
        .envs(vars)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(stdout) = command.stdout.take() {
        let stdout_reader = BufReader::new(stdout);
        std::thread::spawn(move || {
            stdout_reader.lines().for_each(|line| {
                if let Ok(line) = line {
                    println!("{}", line);
                }
            });
        });
    }

    let status = command.wait().unwrap();

    if !status.success() {
        eprintln!("Failed with exit code: {:?}", status.code());
    }
}
