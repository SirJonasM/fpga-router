# Build & Install
## Build
build with:
```sh
cargo build --release
```
## Install
You can install this onto your system with 
```sh
cargo install --path .
```

# Usage
## Overview
```sh
λ router --help
FPGA Routing Utility

Usage: router <COMMAND>

Commands:
  create-test  Creates a test route_plan
  route        Starts the router
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
## create-test
```sh
λ router create-test --help
Creates a test route_plan

Usage: router create-test [OPTIONS] --output <OUTPUT> --graph <GRAPH> --destinations <DESTINATIONS>

Options:
  -o, --output <OUTPUT>
  -g, --graph <GRAPH>
  -d, --destinations <DESTINATIONS>
  -p, --percentage <PERCENTAGE>      [default: 0.2]
  -h, --help                         Print help
  ```
## route
```sh
λ router route --help
Starts the router

Usage: router route [OPTIONS] --output <OUTPUT> --routing-list <ROUTING_LIST> --graph <GRAPH>

Options:
  -o, --output <OUTPUT>
  -r, --routing-list <ROUTING_LIST>
  -g, --graph <GRAPH>
  -s, --solver <SOLVER>              [default: simple] [possible values: simple, steiner, simple-steiner]
  -h, --hist-factor <HIST_FACTOR>    [default: 0.1]
  -l, --log-file <LOG_FILE>
  -h, --help                         Print help
  ```
