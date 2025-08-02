<div align="center">
  <img src="assets/ScampiLight.png" style="width: 200px; max-width: 100%"/>
</div>

## Getting Started

### Usage
```
scampi --help
Usage: scampi --uri <Neo4j URI> --password <Neo4j password>

Options:
  -u, --uri <Neo4j URI>            The Neo4j connection string
  -p, --password <Neo4j password>  The Neo4j password
  -h, --help                       Print help
```

### Using Docker
If you want to analyze crates on your own system, you can skip this step. Otherwise, start by building the Scampi image.

```
docker build -t scampi .
```

When ready, create and run a container using the Scampi image you built. We recommend using the host network and mapping the local `workspace` directory to the one on the container.

```
docker run --name scampi --network host -v ./workspace:/workspace -it scampi
```

If you exited the container and would like to pick up where you left off, use the command below to start the same container.

```
docker start -i scampi
```

If you're already started the container and would like to interact with the shell in another terminal, use `exec`.

```
docker exec -it scampi bash
```

### Installing Crates
Scampi analyzes C usage in Rust code, so the crates you plan on analyzing will most likely require all kinds of additional libraries. For example, to build `neon` you must install the packages below.

```
apt install \
    build-essential \
    libtool \
    libreadline-dev \
    zlib1g-dev \
    flex \
    bison \
    libseccomp-dev \
    libssl-dev \
    clang \
    pkg-config \
    libpq-dev \
    cmake \
    postgresql-client \
    protobuf-compiler \
    libprotobuf-dev \
    libcurl4-openssl-dev \
    openssl \
    python3-poetry \
    lsof \
    libicu-dev
```

### Building and Installing Scampi
You cannot analyze any crates until Scampi has been built and installed. Running `cd workspace/scampi-analyze` and then `./install.sh` should do the trick.

### Analyzing Crates
Once you have cloned a crate, you are ready to analyze it. If the crate does not contain a file called `rust-toolchain.toml`, create one. Otherwise, make sure the value assigned to `channel` matches the one below.

```toml
[toolchain]
channel = "nightly-2025-02-19"
...
```

Then, follow whatever instructions they provide to build the crate for the first time. This process often involves installing additional dependencies and generating bindings.

Finally, you can analyze the crate using the `scampi` command. If you want the results to be output in JSON format, provide an output directory relative to your current one using the `-o` flag. If you would like your results to be saved in a MongoDB database, you can also specify the connection string and database name using the `-u` and `-d` flags, respectively.

You can save the results to an output directory and MongoDB, just one of the two, or neither. If you don't provide any flags, the analysis will be performed as usual but the results will be inaccessible.

```
Usage: scampi [OPTIONS]

Options:
  -o, --out-dir <DIRECTORY>       The JSON output directory
  -u, --uri-mongo <INSTANCE URI>  The MongoDB connection string
  -d, --db-mongo <DATABASE NAME>  The MongoDB database
  -h, --help                      Print help
```

## Examples
### Neon
Neon is a tricky crate to build. If you follow their instructions, there is a good chance that eventually you will run out of RAM. That's okay, though - we don't need the build to succeed. We only need to run the build up to this point to generate some artifacts. After that, you can use `scampi` to analyze the workspace per usual.

### Spacedrive
Spacedrive is also a tricky crate to build. For one, you need Node. Here is how you can install it.

```
# Download and install nvm:
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.2/install.sh | bash

# in lieu of restarting the shell
\. "$HOME/.nvm/nvm.sh"

# Download and install Node.js:
nvm install 23

# Verify the Node.js version:
node -v # Should print "v23.11.0".
nvm current # Should print "v23.11.0".

# Download and install pnpm:
corepack enable pnpm

# Verify pnpm version:
pnpm -v
```
