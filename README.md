<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/ScampiDark.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/ScampiLight.png">
    <img alt="Scampi" src="assets/ScampiLight.png" style="width: 250px;">
  </picture>
</div>

## Getting Started

### Installation
First, clone Scampi.

```
git clone https://github.com/davsec-lab/scampi.git
```

Then, `cd` into the root directory and run `install.sh`, the installation script.

### Database Setup

It doesn't matter where your Neo4j database is running - Scampi only needs the URI and password. However, a great way to start is running it locally using Docker.

```
docker run \
    --restart always \
    --publish=7474:7474 --publish=7687:7687 \
    neo4j:2025.07.0
```

Once the container is up, visit `http://localhost:7474`. Right now, the default user is `neo4j` and so is the password. After signing in for the first time, you will be prompted to reset your password. Remember it!

Read [this](https://neo4j.com/docs/operations-manual/current/docker/introduction/) article to learn more about using Neo4j and Docker. Alternatively, create a free graph database on [AuraDB](https://neo4j.com/product/auradb/).


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

For example, if you set up Neo4j locally, the command might look something like...

```
scampi --uri bolt://localhost:7687 --password Qwerty12!
```

Note that some crates require additional setup, such as packages that need to be installed. In many such cases, there will be an installation script or at least some setup instructions. Follow those instructions _before_ running Scampi to generate necessary build artifacts.

## Example Queries

Find all C functions and the arguments they accept.

**Query**
```
MATCH (c:C)-[r:ACCEPTS]->(param:Param)
RETURN c, r, param
```

**Result**
![](/assets/example-screenshots/c-functions-and-arguments.png)

<hr/>

Find all C functions that take arguments that are mutable pointers.

**Query**
```
MATCH (c:C)-[r:ACCEPTS]->(param:Param)
WHERE param.is_mutable_ptr = true
RETURN c, r, param
```

**Result**
![](/assets/example-screenshots/c-functions-accepting-mut-ptrs.png)

<hr/>

Find every C function and the (Rust) function that immediately calls it.

**Query**
```
MATCH p = ()-[:CALLS]->(c:C)
RETURN p
```

**Result**
![](/assets/example-screenshots/c-calls-from-rust.png)


<hr/>

Find every call chain between length 1 and 3 that ends in a C function.

**Query**
```
MATCH p = ()-[:CALLS*1..3]->(c:C)
RETURN p
LIMIT 100
```

**Result**
![](/assets/example-screenshots/c-call-chains.png)

<hr/>

Count how many C functions there are and return a sample of five.

**Query**
```
MATCH (c:C)
RETURN count(c) as total_c_functions, collect(c.name)[0..5] as sample_function_names
```

**Result**
![](/assets/example-screenshots/c-function-count.png)