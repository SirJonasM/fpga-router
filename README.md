# Build & Install

## Build
build with:
```sh
cargo build --release
```
## Install
You can install this onto your system with 
```sh
cargo install --path ./router
```

# Usage
1. Have a routing plan, (can be generated with `router create-test`)
2. solver the routing plan with `router route`
    - produces json -> use `router fasm` to generate fasm
    - produces fasm

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
# Example
Creating a simple route-plan:
```json
[
  {
    "sinks": [
      "X1Y2.LA_I0",
      "X1Y1.LA_I1"
    ],
    "signal": "X1Y1.LA_O",
    "result": null
  },
  {
    "sinks": [
      "X1Y1.LA_I2",
      "X1Y1.LA_I3"
    ],
    "signal": "X1Y2.LA_O",
    "result": null
  }
]
```

```sh
λ router route -o routing.json -r route-plan.json -g pips.txt -s simple -h 0.1 -l log.txt
0,0,0.1,Simple Solver,0,699,3159,8,0,28,1.071795,228
Success: 0
Wrote the routing into routing.json
```
=> 
```json
[
  {
    "sinks": [
      "X1Y1.LA_I1",
      "X1Y2.LA_I0"
    ],
    "signal": "X1Y1.LA_O",
    "result": {
      "paths": {
        "X1Y1.LA_I1": [
          "X1Y1.LA_O",
          "X1Y1.JE2BEG3",
          "X1Y1.JE2END3",
          "X1Y1.J2MID_ABa_BEG1",
          "X1Y1.J2MID_ABa_END1",
          "X1Y1.LA_I1"
        ],
        "X1Y2.LA_I0": [
          "X1Y1.LA_O",
          "X1Y1.JS2BEG6",
          "X1Y1.JS2END6",
          "X1Y1.S2BEG6",
          "X1Y2.S2MID6",
          "X1Y2.J2MID_ABa_BEG0",
          "X1Y2.J2MID_ABa_END0",
          "X1Y2.LA_I0"
        ]
      },
      "nodes": [
        "X1Y2.S2MID6",
        "X1Y1.J2MID_ABa_END1",
        "X1Y1.LA_I1",
        "X1Y1.JS2BEG6",
        "X1Y2.J2MID_ABa_END0",
        "X1Y1.J2MID_ABa_BEG1",
        "X1Y1.S2BEG6",
        "X1Y1.JE2END3",
        "X1Y2.LA_I0",
        "X1Y1.JS2END6",
        "X1Y2.J2MID_ABa_BEG0",
        "X1Y1.JE2BEG3",
        "X1Y1.LA_O"
      ]
    }
  },
  {
    "sinks": [
      "X1Y1.LA_I2",
      "X1Y1.LA_I3"
    ],
    "signal": "X1Y2.LA_O",
    "result": {
      "paths": {
        "X1Y1.LA_I3": [
          "X1Y2.LA_O",
          "X1Y2.JN2BEG1",
          "X1Y2.JN2END1",
          "X1Y2.N2BEG1",
          "X1Y1.N2MID1",
          "X1Y1.J2MID_ABb_BEG3",
          "X1Y1.J2MID_ABb_END3",
          "X1Y1.LA_I3"
        ],
        "X1Y1.LA_I2": [
          "X1Y2.LA_O",
          "X1Y2.JN2BEG5",
          "X1Y2.JN2END5",
          "X1Y2.N2BEG5",
          "X1Y1.N2MID5",
          "X1Y1.J2MID_ABb_BEG2",
          "X1Y1.J2MID_ABb_END2",
          "X1Y1.LA_I2"
        ]
      },
      "nodes": [
        "X1Y1.J2MID_ABb_BEG2",
        "X1Y2.JN2BEG1",
        "X1Y1.LA_I3",
        "X1Y2.N2BEG1",
        "X1Y2.JN2END5",
        "X1Y1.J2MID_ABb_BEG3",
        "X1Y1.N2MID1",
        "X1Y2.JN2BEG5",
        "X1Y1.LA_I2",
        "X1Y2.LA_O",
        "X1Y1.J2MID_ABb_END3",
        "X1Y2.JN2END1",
        "X1Y2.N2BEG5",
        "X1Y1.J2MID_ABb_END2",
        "X1Y1.N2MID5"
      ]
    }
  }
]
```
and to produce a fasm either:
```sh
λ router fasm -r routing.json -o routing.fasm
```
or:
```sh
λ router route -o routing.fasm -r route-plan.json -g pips.txt -s simple -h 0.1 -l log.txt
0,0,0.1,Simple Solver,0,699,3159,8,0,28,1.071795,226
Success: 0
Wrote the routing into routing.fasm
```

Either will produce:
```sh
λ cat routing.fasm
X1Y1.J2MID_ABa_BEG1.J2MID_ABa_END1
X1Y1.J2MID_ABa_END1.LA_I1
X1Y1.J2MID_ABb_BEG2.J2MID_ABb_END2
X1Y1.J2MID_ABb_BEG3.J2MID_ABb_END3
X1Y1.J2MID_ABb_END2.LA_I2
X1Y1.J2MID_ABb_END3.LA_I3
X1Y1.JE2BEG3.JE2END3
X1Y1.JE2END3.J2MID_ABa_BEG1
X1Y1.JS2BEG6.JS2END6
X1Y1.JS2END6.S2BEG6
X1Y1.LA_O.JE2BEG3
X1Y1.LA_O.JS2BEG6
X1Y1.N2MID1.J2MID_ABb_BEG3
X1Y1.N2MID5.J2MID_ABb_BEG2
X1Y2.J2MID_ABa_BEG0.J2MID_ABa_END0
X1Y2.J2MID_ABa_END0.LA_I0
X1Y2.JN2BEG1.JN2END1
X1Y2.JN2BEG5.JN2END5
X1Y2.JN2END1.N2BEG1
X1Y2.JN2END5.N2BEG5
X1Y2.LA_O.JN2BEG1
X1Y2.LA_O.JN2BEG5
X1Y2.S2MID6.J2MID_ABa_BEG0
```

```sh
λ diff routing2.fasm routing.fasm
```
