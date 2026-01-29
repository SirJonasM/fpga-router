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
  The format of the route-plan is the following:
  ```json
[
    {
        "sinks": [
            "X1Y4.LD_I2",
            "X3Y4.LD_I1",
            "X3Y3.LG_I3",
            "X3Y3.LE_I2"
        ],
        "signal": "X1Y4.LH_O",
        "result": null
    }
]
  ```
## route
```sh
λ router route --help
Starts the router

Usage: router route [OPTIONS] --output <OUTPUT> --routing-list <ROUTING_LIST> --graph <GRAPH>

Options:
  -o, --output <OUTPUT>
          Can be `json` or `fasm`
  -r, --routing-list <ROUTING_LIST>

  -g, --graph <GRAPH>

  -s, --solver <SOLVER>
          [default: simple] [possible values: simple, steiner, simple-steiner]
  -h, --hist-factor <HIST_FACTOR>
          [default: 0.1]
  -L, --logger <LOGGER>
          [default: terminal] [possible values: no, terminal, file]
  -l, --log-file <LOG_FILE>

  -i, --max-iterations <MAX_ITERATIONS>
          [default: 2000]
  -h, --help
          Print help
```json
[
    {
        "sinks": [
            "X3Y3.LE_I2",
            "X3Y3.LG_I3",
            "X1Y4.LD_I2",
            "X3Y4.LD_I1"
        ],
        "signal": "X1Y4.LH_O",
        "result": {
            "paths": {
                "X3Y4.LD_I1": [
                    "X1Y4.LH_O",
                    ...
                    "X3Y4.LD_I1"
                ],
                "X3Y3.LE_I2": [
                    "X3Y3.LE_I2"
                    ...
                    "X3Y3.LE_I2"
                ],
                "X3Y3.LG_I3": [
                    "X1Y4.LH_O",
                    ...
                    "X3Y3.LG_I3"
                ],
                "X1Y4.LD_I2": [
                    "X1Y4.LH_O",
                    ...
                    "X1Y4.LD_I2"
                ]
          },
          "nodes": [
            "X3Y4.J2END_CD_BEG1",
            ...
            "X3Y3.N1END2"
          ]
    }
]
```
## FASM
```sh
λ router fasm --help
parses the router output to fasm

Usage: router fasm --output <OUTPUT> --routing <ROUTING>

Options:
  -o, --output <OUTPUT>
  -r, --routing <ROUTING>
  -h, --help               Print help
```
