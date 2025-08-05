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