use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use clap::Parser;

#[derive(Parser)]
struct Args {
    /// The Neo4j connection string.
    #[arg(short, long, value_name = "Neo4j URI")]
    uri: String,

    /// The Neo4j password.
    #[arg(short, long, value_name = "Neo4j password")]
    password: String,
}

fn main() {
    let args = Args::parse();

    let vars = vec![
        ("SCAMPI_LOG_LEVEL", "DEBUG"),
        ("SCAMPI_NEO4J_URI", &args.uri),
        ("SCAMPI_NEO4J_PASSWORD", &args.password),
    ];

    let mut command = Command::new("cargo")
        .arg("run")
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
