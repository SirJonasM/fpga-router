#set page(paper: "a4")
#set list(marker: ([--], [--], [--]))
#include "cover.typ"
#counter(page).update(1)
#set page(numbering: "[1]")
#set heading(numbering: "1.")
#set text(
  font: "New Computer Modern",
  size: 12pt,
)
#show raw: set text(font: "Iosevka NF")

#set page(header: text(size: 10pt)[#grid(
  columns: (1fr, 2fr, 1fr),
  column-gutter: 1fr,
  align: (left, center, right),
  [ACF], [University Heidelberg], [Jonas Möwes],
)])
#show raw.where(block: true): block.with(fill: rgb(0,120, 255, 4%))
#show raw.where(block: false): box.with(fill: rgb(0,120,255, 4%))

= Overview
FPGA routing is the stage in the physical design flow that follows placement.
While placement determines the physical locations of logic blocks like Look-Up Tables (LUTs) and Flip-Flops, the routing process must establish the electrical connections between them using the FPGA's fixed wiring resources and programmable switches.
This task is primarily a resource-allocation problem: the router must resolve congestion where multiple signals compete for the same wire while maintaining signal integrity.

This project implements an FPGA router written in Rust, designed to handle these constraints using the PathFinder algorithm.
The tool integrates a Static Timing Analyzer (STA) to perform timing-driven routing, which prioritizes critical paths to meet hardware performance requirements.
Architected for extensibility, the system supports multiple routing strategies, including Dijkstra-based and Steiner tree approximations, to navigate the trade-offs between execution speed and wire efficiency.

= Implementation
== Project Structure
*The Engine (Library Layer)* \
The engine serves as the core logic provider. It is designed with extensibility in mind to allow for future experimentation with different routing algorithms.
- *Net Solver Trait (`RouteNet`): *
  The Net solver is implemented as a Rust Trait, making it fully extensible.
  This allows the engine to swap between different routing strategies—such as the `Simple`, `Steiner`, or `Simple Steiner` solvers—without changing the underlying engine code.
- *SlackReport Trait (`TimingAnalysis`): * 
  Similar to the solver, the Timing Analysis is abstracted as a trait, allowing for different timing analysis backends to be hooked into the routing process.
*The CLI (Application Layer)* \
The CLI acts as the front-end "driver" for the engine, handling data ingestion and execution.

== Solver
The solver is resposible for for routing individual nets.
- *Simple:*
  Utilizes Dijkstra’s algorithm for each sink. It supports LUT input swapping by routing to all possible inputs and selecting the optimal one.
- *Steiner:*
  Approximates a Steiner tree to minimize overall wire consumption. While more wire-efficient, it is significantly slower than the Simple solver.
- *Simple Steiner:*
  A hybrid approach that pre-calculates Steiner nodes to speed up the process. While faster than the standard Steiner solver, it can be less efficient as Steiner nodes do not adapt to changing congestion costs.

== Advanced features
- *Timing Driven Routing: *
  The engine integrates a Static Timing Analyzer (STA) to calculate net criticality.
  Currently, only the Simple solver supports this mode.
  

- *LUT-based Tie-off: *
  If a `VCC` or `GND` signal is unsolvable due to routing constraints, the router can "borrow" a nearby LUT to act as a tie-off point.
  This is automatically handled during FASM generation by initializing the borrowed LUTs correctly.

= Build Guide
== Dependencies and Setup
- *Environment: *
  The project leverages Nix Flakes to provide a fully reproducible development environment;
  simply running nix develop automatically configures the Rust toolchain (including rust-analyzer and clippy) and Python 3 dependencies.
- *Tools: *
  Requires *Cargo* for the Rust build and *Python* for mapping the placement to netlists.
== Building
- Build with: `cargo build -p router-cli --release`
== Installing
By default, the project is run using Cargo commands. To use the router as a standalone tool from any directory, follow these steps:
+ *Compile and Install:* \
  Run the following command from the project root to install the binary to your local Cargo bin directory: \
  `cargo install --path cli`
+ *Update System Path:*
  Ensure your shell can locate the installed binary by adding the Cargo bin folder to your `PATH`:\
  `export PATH=$PATH:~/.cargo/bin`
+ Command Simplification
  Once installed, you can replace the lengthy cargo run development commands with the direct binary call. 
  For example:
    - Development: `cargo run -p router-cli --release -- [args]`
    - Installed: `router-cli [args]`

== Executing the router
=== Net List Creation 
A `net-list` can be generated in two ways.
+ `create-test` command:
  - Generates synthetic netlists based on a percentage of LUT outputs and a specified number of sinks per output.
  - Example (30% of LUT outputs with 3 sinks each): \
    ``` 
cargo run -p router-cli --release -- create-test \
  -g tests/data/pips_4x4.txt \
  -o net-list.json \
  -d 3 -p 0.3
```
+ Mapping a `placement.json` from `nextpnr-generic` placer.
  - `nextpnr` has the option to generate a `json` (`--write placement.json`) file that contains data about the placement.
    This file can be mapped using the `map_net_io.py` script to a `net-list.json`.
  - This will also produce a `ffs.fasm` file that initilalizes the Flip-Flops.
    The router uses it by adding the `--ffs <file>` flag.
  - Example: `python map_net_io.py net-list.json`

=== Routing 
Once a netlist is prepared, the router is executed using the route command.
While the tool supports a wide range of configuration options, they generally fall into three categories:
- *Required Infrastructure:*
  Users must provide the fabric definition files (`--graph`, `--bel`) the input `--net-list` and the flip-flop FASM file  (`--ffs`).
- *Routing Strategy:* 
  The algorithm is selected via the `--solver` flag (Simple, Steiner, or Simple-Steiner).
  Users can further tune the PathFinder logic using `--hist-factor` and `--max-iterations`.
- *Timing & Constraints:* 
  A `--timings` file is required for all routing passes because the engine utilizes the timing model to establish base costs for the routing graph.
  The `--timing-driven` flag specifically activates the timing-driven router, which uses this data to calculate net criticality during an analysis pass.
*Note:* Specific arguments are self-explanatory and a comprehensive list of all available flags can be found by using the `--help` argument with any command.

*Example Execution:* \
  ```
  # First generate a net-list.json
  cargo run -p router-cli --release -- route \
  -o out.fasm \
  -n net-list.json \
  -g tests/data/pips_4x4.txt \
  -b tests/data/bel_4x4.txt  \
  -t tests/data/timing_model.json
```

= Tools and Data Used
- STA from the other team (With some changes to make it usable in this project)
- FABulous
  - The project uses the small demo fabric and the default fabric of FABulous.
  - And the placement file `placement.json` generated from the demo design: `sequential_16bit_en.v`
  - All artifacts can be found in `tests/data/`

= Further Notes
- *Performance:*
  Currently, iterations on default FABulous fabrics take 1–2 seconds.
  A proposed optimization is to implement incremental routing that only targets congested nets.
- *Compatibility: * 
  The project was primarily tested on the `sequential_16bit_en.v` design.
  As not all placement features are currently supported, errors may occur with more complex designs.
- *GitHub:* 
  #link("https://github.com/SirJonasM/fpga-router")

