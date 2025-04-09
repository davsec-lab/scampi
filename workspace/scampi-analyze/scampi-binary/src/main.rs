use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use clap::{Arg, Parser};

#[derive(Parser)]
struct Args {
    /// The output directory.
    name: String,
}

fn main() {
    let args = Args::parse();

    let out_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("scampi-persist")
        .join("data")
        .join(args.name);

    fs::create_dir_all(&out_path.join("functions")).expect(&format!(
        "Failed to create {:?}",
        &out_path.join("functions")
    ));

    fs::create_dir_all(&out_path.join("invocations"))
        .expect("Failed to create invocation output path!");

    let mut command = Command::new("cargo")
        .arg("check")
        .arg("--keep-going")
        .env("RUSTC_WRAPPER", "scampi-driver")
        .env("SCAMPI_OUT_DIR", out_path)
        .env("SCAMPI_LOG_LEVEL", "DEBUG")
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
