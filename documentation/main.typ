#set page(paper: "a4")
#include "cover.typ"
#counter(page).update(1)
#set page(numbering: "1.")
#set heading(numbering: "1.")
#set text(
  font: "New Computer Modern",
  size: 12pt,
)

#set page(header: text(size: 10pt)[#grid(
  columns: (1fr, 2fr, 1fr),
  column-gutter: 1fr,
  align: (left, center, right),
  [ACF], [University Heidelberg], [Jonas Möwes],
)])

= Overview
This project implements a FPGA router.
= Implementation
- engine: Implements the `path_finder`, `Net solver` and
- cli: Uses the engine to route a given `netlist` for an given `Fabric` defined by `bel.txt` and `pips.txt`.
== PathFinder

== Solver
The solver is the algorithm that is used to route a single net.
=== Simple
- Solves each net by running dijkstra for each sink
- Uses LUT input swapping by routing to all possible inputs on the LUT and choosing the best performing one.
=== Steiner
- Approximates a steiner tree for signal and sinks as terminals.
- Calculates a base path to one sink. 
- Goes then over the path again and finds nodes from which another sink can be connected with the lowest cost. This makes the net overall cheaper in terms of overall wire consumption but the algorithm is much slower.
=== Simple Steiner
- Pre calculates the steiner nodes. Nodes that are used to hook up the other nodes from the base path. Saves them for the actual routing of the net.
- When net needs to be solved it just calculates each path by using the steiner nodes.
- This is much faster then the steiner solver but also not as efficient as normaly the steiner nodes change when the congestion cost changes.
- Also sometimes the pre computation makes the routing impossible. But this issue gets tackled by running the precomputation again if the solver does not progress.
== Timing Driven
- the engine can be run with a static timing analyzer hooked up. The cli uses the STA from the other team to do that. 
- The STA calculates a criticallity that gets used in the net routing (currently only the simple solver supports that). 

== LUT-based Tie-off
- The router first checks if all nets are solvable while not caring about congestion.
- If a net is not solvable it also checks if the signal is a VCC or GND signal. If so it tries to find a LUT near the sink and routes from there marking the LUT as borrowed and leaving a mark for HIGH or LOW.
- When generating the `fasm` it goes over all LUTs and if it is borrowed it generates FASM that initializes the LUT properly.
= Build Guid
== Dependencies
=== Nix
- It is a nix project so using nix shell and cargo build should be sufficient
=== Custom
- dependencies:
  - Cargo
  - Python (only to map a `placement.json` to a `net-list.json` that the router uses)
== Building
- `cargo build -p router-cli --release`
== Installing
- The router can be installed using cargo and then used by adding the binary to PATH. all commands that run the binary with `cargo run -p router-cli --release` can then be replaced using the binary with `router-cli`
- `cargo install --path cli`
== Running
First a net-list is needed. There are 2 ways to generate one:
- create-test command:
  - The cli provides a `create-test` command which generates a test-net-list with two parameters:
    - percentage: The percentage of LUT-Outputs that the fabric contains that will be used in the net-list.
    - destinations: How manny sinks each LUT-Output is connected to.
  - Example: `router-cli create-test -g tests/data/pips_8x8.txt -o net-list.json -d 3 -p 0.3`
- mapping a `placement.json` from `nextpnr-generic` placer.
  - `nextpnr` has the option to generate json file that contains some data about the placement. This can be mapped using the `map_net_io.py` script to a net-list.json
  - `python map_net_io.py net-list.json`
  - This will also produce a `ffs.fasm` file that gets combined with the router output fasm and sets the Flip-Flops.
Now the router can be run using the `net-list.json` and the `route` command.
There are several arguments that can be passed to the router:
- files: output, bel, pips and net-list file
  - timings file: To make the Static Timing Analysis there is a timings argument which sets the timing model and the timing constraints. It is similar to the STA timing files except that its combined into one.
  - ffs file: the file that was produced using the `map_net_io.py` script.
- hist-factor: sets the historic factor of the path-finding algorithm
- solver: the solver for the net that is used there are 3 implements each with advantages and disadvantages:  
- simple: 
  - simple-steiner:
  - steiner:
- max-iterations: how many iterations the router does before failing
- timing-driven: activates the timing driven router.

= Tools and Data Used
- Sta from the other team
- FABulous - `pips.txt` and `bel.txt`
- nextpnr-genereric - `placement.json`
= Further Notes
