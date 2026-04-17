use regex::Regex;
use std::{collections::HashMap, collections::HashSet};

use crate::{Configuration, Connection, Edge, Flop, Node, Pip, TimingModel};

pub fn make_fabric(pips: &[Pip]) -> HashMap<Node, HashMap<Node, Edge>> {
    let mut fabric: HashMap<Node, HashMap<Node, Edge>> = HashMap::new();
    for pip in pips {
        fabric
            .entry(pip.src.clone())
            .or_default()
            .insert(pip.dst.clone(), Edge { delay: pip.delay });
    }
    fabric
}

pub fn build_design(
    pips: &[Pip],
    configurations: &[Configuration],
    flops: &[Flop],
    timing_model: &TimingModel,
) -> HashMap<Node, HashMap<Node, Connection>> {
    let mut design: HashMap<Node, HashMap<Node, Connection>> = HashMap::new();
    let mut active_nodes: HashSet<Node> = HashSet::new();

    let pip_delay = timing_model.pip_delay;
    let lut_delay = timing_model.lut_delay;
    let fanout_delay = timing_model.fanout_delay;
    let clock_to_output_delay = timing_model.clock_to_output_delay;
    let clock_tree_delay = timing_model.clock_tree_delay;

    // 1. Add tile-internal pips enabled in FASM (Programmable)
    for config in configurations {
        if let Some(pip) = pips.iter().find(|pip| {
            pip.src.tile == config.tile
                && pip.src.pin == config.src_pin
                && pip.dst.pin == config.dst_pin
        }) {
            design
                .entry(pip.src.clone())
                .or_default()
                .insert(pip.dst.clone(), Connection { delay: pip_delay });
            active_nodes.insert(pip.src.clone());
            active_nodes.insert(pip.dst.clone());
        } else {
            log::warn!(
                "PIP not found for configuration: {:?} {} -> {}",
                config.tile,
                config.src_pin,
                config.dst_pin
            );
        }
    }

    // 2. Add internal connections for combinational LUTs and Flops
    // Identify current sinks to find where routing enters a LUT
    let mut all_destinations = HashSet::new();
    for destinations in design.values() {
        for dst in destinations.keys() {
            all_destinations.insert(dst.clone());
        }
    }

    let mut all_sinks = HashSet::new();
    for dst in &all_destinations {
        if !design.contains_key(dst) {
            all_sinks.insert(dst.clone());
        }
    }

    let lut_input_regex = match Regex::new(r"L([A-H])_I\d+") {
        Ok(re) => re,
        Err(_) => return design,
    };

    for sink in all_sinks {
        if let Some(caps) = lut_input_regex.captures(&sink.pin) {
            let lut_char = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Some(first_char) = lut_char.chars().next() {
                let lut_idx = first_char as u32 - 'A' as u32;

                // Check if this LUT has a used flip-flop in the same tile
                let is_flopped = flops
                    .iter()
                    .any(|flop| flop.tile == sink.tile && flop.lut == lut_idx);

                if !is_flopped {
                    // Combinational
                    let output_node = Node {
                        tile: sink.tile,
                        pin: format!("L{}_O", first_char),
                    };
                    design
                        .entry(sink.clone())
                        .or_default()
                        .insert(output_node.clone(), Connection { delay: lut_delay });

                    active_nodes.insert(sink.clone());
                    active_nodes.insert(output_node);
                } else {
                    // To Flop
                    let ff_d_node = Node {
                        tile: sink.tile,
                        pin: format!("L{}_FF_D", first_char),
                    };
                    design
                        .entry(sink.clone())
                        .or_default()
                        .insert(ff_d_node.clone(), Connection { delay: lut_delay });

                    active_nodes.insert(sink.clone());
                    active_nodes.insert(ff_d_node);
                }
            }
        }
    }

    // Add logic sources (Flip-Flop Outputs) to active nodes to catch outgoing routing
    for flop in flops {
        let l_out = Node {
            tile: flop.tile,
            pin: format!("L{}_O", (flop.lut as u8 + b'A') as char),
        };
        active_nodes.insert(l_out);
    }

    // 3. Iteratively add Fixed Routing (tile-external pips) that touches active nodes
    // Filter fixed pips first for speed
    let fixed_pips: Vec<&Pip> = pips.iter().filter(|p| p.src.tile != p.dst.tile).collect();

    let mut changed = true;
    while changed {
        changed = false;
        for pip in &fixed_pips {
            // Check existence to avoid re-adding
            // Note: design.entry...or_default() is cheap but checking contains_key is cheaper

            // Check if already in design
            if let Some(d) = design.get(&pip.src) {
                if d.contains_key(&pip.dst) {
                    continue;
                }
            }

            // Connection Logic:
            // Connect if Source is Active (Drivers -> routing)
            // OR if Dest is Active (Routing -> Inputs)
            let src_active = active_nodes.contains(&pip.src);
            let dst_active = active_nodes.contains(&pip.dst);

            if src_active || dst_active {
                design
                    .entry(pip.src.clone())
                    .or_default()
                    .insert(pip.dst.clone(), Connection { delay: pip_delay });

                // Activate potential new neighbors
                if !src_active {
                    active_nodes.insert(pip.src.clone());
                    changed = true;
                }
                if !dst_active {
                    active_nodes.insert(pip.dst.clone());
                    changed = true;
                }
            }
        }
    }

    if fanout_delay > 0.0 {
        for destinations in design.values_mut() {
            let fanout = destinations.len();
            if fanout > 1 {
                let extra_delay = (fanout as f64 - 1.0) * fanout_delay;
                for conn in destinations.values_mut() {
                    conn.delay += extra_delay;
                }
            }
        }
    }

    if clock_to_output_delay > 0.0 {
        let mut flopped_outputs = HashSet::new();
        for flop in flops {
            flopped_outputs.insert(Node {
                tile: flop.tile,
                pin: format!("L{}_O", (flop.lut as u8 + b'A') as char),
            });
        }
        for (src, destinations) in design.iter_mut() {
            if flopped_outputs.contains(src) {
                let clk_delay = clock_tree_delay * clock_tree_delay_base(src);
                let total_initial_delay = clock_to_output_delay + clk_delay;
                for conn in destinations.values_mut() {
                    conn.delay += total_initial_delay;
                }
            }
        }
    }

    // Add Virtual Sinks for Capture Latency subtraction
    if clock_tree_delay > 0.0 {
        let mut new_edges = Vec::new();

        // Find all FF_D nodes.
        let mut ff_d_nodes = HashSet::new();
        for destinations in design.values() {
            for dst in destinations.keys() {
                if dst.pin.contains("FF_D") {
                    ff_d_nodes.insert(dst.clone());
                }
            }
        }

        for ff_d in ff_d_nodes {
            let sink_delay = clock_tree_delay * clock_tree_delay_base(&ff_d);
            let virtual_sink = Node {
                tile: ff_d.tile,
                pin: ff_d.pin.replace("FF_D", "FF_SINK"),
            };
            new_edges.push((ff_d, virtual_sink, -sink_delay));
        }

        for (src, dst, delay) in new_edges {
            design
                .entry(src)
                .or_default()
                .insert(dst, Connection { delay });
        }
    }

    design
}

fn clock_tree_delay_base(node: &Node) -> f64 {
    node.tile.0 as f64 + node.tile.1 as f64
}

// Convert a Node into a stable string key for JSON maps
pub fn node_key(node: &Node) -> String {
    format!("X{}Y{}.{}", node.tile.0, node.tile.1, node.pin)
}

// Convert the design map into a JSON-friendly map with String keys
pub fn design_to_json_map(
    design: &HashMap<Node, HashMap<Node, Connection>>,
) -> HashMap<String, HashMap<String, Connection>> {
    let mut out: HashMap<String, HashMap<String, Connection>> = HashMap::new();
    for (src, dsts) in design {
        let src_key = node_key(src);
        let mut inner: HashMap<String, Connection> = HashMap::new();
        for (dst, conn) in dsts {
            inner.insert(node_key(dst), conn.clone());
        }
        out.insert(src_key, inner);
    }
    out
}

pub fn design_stats(design: &HashMap<Node, HashMap<Node, Connection>>, flops: &[Flop]) {
    let total_nodes = design.len();
    let total_connections: usize = design.values().map(|d| d.len()).sum();
    log::info!("Design Statistics:");
    log::info!("  Total Active Nodes: {}", total_nodes);
    log::info!("  Total Connections: {}", total_connections);
    let mut in_degree: HashMap<Node, usize> = HashMap::new();
    let mut all_nodes: std::collections::HashSet<Node> = std::collections::HashSet::new();

    for (src, connections) in design {
        all_nodes.insert(src.clone());
        in_degree.entry(src.clone()).or_insert(0);
        for dst in connections.keys() {
            all_nodes.insert(dst.clone());
            *in_degree.entry(dst.clone()).or_insert(0) += 1;
        }
    }

    // 2. Identify source nodes (in-degree 0 in the provided graph)
    // Note: In a routed netlist, true sources are graph sources.
    let source_nodes: Vec<Node> = all_nodes
        .iter()
        .filter(|node| *in_degree.get(node).unwrap_or(&0) == 0)
        .cloned()
        .collect();

    // 3. Identify flop source nodes (outputs of used FFs/LUTs)
    let flop_sources: Vec<Node> = all_nodes
        .iter()
        .filter(|src| {
            flops.iter().any(|flop| {
                flop.tile == src.tile
                    && src
                        .pin
                        .contains(&format!("L{}_O", (flop.lut as u8 + b'A') as char))
            })
        })
        .cloned()
        .collect();

    log::info!("Total nodes in graph: {}", all_nodes.len());
    log::info!("Graph source nodes (indeg=0): {}", source_nodes.len());

    // Convert detailed list log::infos to debug logs
    if log::log_enabled!(log::Level::Debug) {
        for node in &source_nodes {
            log::debug!("  - X{}Y{}.{}", node.tile.0, node.tile.1, node.pin);
        }
    }

    log::info!("Flopped logic sources (FF/LUT out): {}", flop_sources.len());

    if log::log_enabled!(log::Level::Debug) {
        for node in &flop_sources {
            log::debug!("  - X{}Y{}.{}", node.tile.0, node.tile.1, node.pin);
        }
    }

    // Identify Sink Nodes (Nodes pointing nowhere)
    let sink_nodes: Vec<Node> = all_nodes
        .iter()
        .filter(|node| !design.contains_key(node))
        .cloned()
        .collect();

    log::info!("Graph sink nodes (outdeg=0): {}", sink_nodes.len());

    if log::log_enabled!(log::Level::Debug) {
        for node in &sink_nodes {
            log::debug!("  - X{}Y{}.{}", node.tile.0, node.tile.1, node.pin);
        }
    }

    // Identify Flopped Sink Nodes (FF Data Inputs)
    let flopped_sinks: Vec<Node> = sink_nodes
        .iter()
        .filter(|node| node.pin.contains("FF_D"))
        .cloned()
        .collect();

    log::info!("Flopped sink nodes (FF Data In): {}", flopped_sinks.len());

    if log::log_enabled!(log::Level::Debug) {
        for node in &flopped_sinks {
            log::debug!("  - X{}Y{}.{}", node.tile.0, node.tile.1, node.pin);
        }
    }
}
