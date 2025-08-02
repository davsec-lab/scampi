<div align="center">
  <img src="assets/ScampiLight.png" style="width: 200px; max-width: 100%"/>
</div>

## Getting Started

### Installation
First, clone Scampi.

```
git clone https://github.com/davsec-lab/scampi.git
```

Then, `cd` into the root directory and run `install.sh`, the installation script.

### Usage
Clone the crate you are interested in analyzing and make sure `rust-toolchain.toml` contains the fields below. Create the file if it doesn't exist.

```
[toolchain]
channel = "nightly-2025-02-19"
...
```

You are ready to analyze with Scampi!

```
scampi --help
Usage: scampi --uri <Neo4j URI> --password <Neo4j password>

Options:
  -u, --uri <Neo4j URI>            The Neo4j connection string
  -p, --password <Neo4j password>  The Neo4j password
  -h, --help                       Print help
```
