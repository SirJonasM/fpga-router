# FPGA Router

This project implements an **FPGA router written in Rust**. It is designed to follow the placement stage of the physical design flow, establishing electrical connections between logic blocks like LUTs and Flip-Flops using an FPGA's fixed wiring and programmable switches. The router utilizes the **PathFinder algorithm** to resolve resource congestion and ensure signal integrity.

## Key Features

*Timing-Driven Routing**: Integrates a Static Timing Analyzer (STA) to prioritize critical paths and meet hardware performance requirements.
*Extensible Architecture**: Built with a trait-based engine, allowing for easy experimentation with different routing algorithms and timing analysis backends.
* **Multiple Solvers**:
    *Simple**: Dijkstra-based routing with support for LUT input swapping.
    *Steiner**: Approximates Steiner trees for high wire efficiency.
    *Simple Steiner**: A hybrid approach for faster Steiner-based routing.
*LUT-based Tie-off**: Automatically handles unsolvable `VCC` or `GND` signals by "borrowing" nearby LUTs as tie-off points.

## Installation

### Environment Setup
This project uses **Nix Flakes** for a reproducible development environment. To configure the Rust toolchain (including `rust-analyzer` and `clippy`) and Python 3 dependencies, run:
```bash
nix develop
```

### Building and Global Install
1.  **Build**:
    ```bash
    cargo build -p router-cli --release
    ```
2.  **Install**:
    To use the `router-cli` globally, install it to your Cargo bin directory:
    ```bash
    cargo install --path cli
    export PATH=$PATH:~/.cargo/bin
    ```

## Usage

### 1. Netlist Creation
You can generate a `net-list.json` in two ways:
*Synthetic Tests**: Use the `create-test` command to generate netlists based on LUT output percentages.
*Placement Mapping**: Map a `placement.json` from the `nextpnr-generic` placer using the provided `map_net_io.py` script.

### 2. Routing
Execute the router using the `route` command. All routing passes require a `--timings` file to establish base costs for the routing graph.

**Example:**
```bash
router-cli route \
  -o out.fasm \
  -n net-list.json \
  -g tests/data/pips_4x4.txt \
  -b tests/data/bel_4x4.txt \
  -t tests/data/timing_model.json
```

For a full list of configuration options, including `--hist-factor`, `--max-iterations`, and `--solver`, run:
```bash
router-cli route --help
```

## Performance and Compatibility
*Performance**: Current iterations on default FABulous fabrics take approximately 1–2 seconds.
*Compatibility**: Primarily tested on the `sequential_16bit_en.v` design. More complex designs may encounter errors as some placement features are not yet fully supported.
