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
= Build Guide
== Nix
- It is a nix project so using nix shell and cargo build should be sufficient
== Custom
- dependencies: 
  - Cargo
  - Python (only to map a `placement.json` to a `net-list.json` that the router uses)
= Tools and Data Used
- Sta from the other team
- FABulous - `pips.txt` and `bel.txt`
- nextpnr-genereric - `placement.json`
= Further Notes
#lorem(100)

