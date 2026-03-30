use std::collections::HashSet;

use crate::{FabricError, FabricResult, fabric::node::Node, netlist::NetListExternal};

pub fn net_to_fasm(expanded_nets: &NetListExternal) -> FabricResult<String> {
    let mut fasm_output = Vec::new();

    for net in &expanded_nets.plan {
        let result = net.result.as_ref().ok_or(FabricError::NetNotSolved)?;

        let mut net_lines = Vec::new();
        net_lines.push(format!("# Net {}", net.signal));

        let mut unique_segments = HashSet::new();
        for path in result.paths.values() {
            for pair in path.windows(2) {
                if let Some(line) = nodes_to_fasm_line(&pair[0], &pair[1]) {
                    unique_segments.insert(line);
                }
            }
        }

        net_lines.extend(unique_segments);

        fasm_output.push(net_lines.join("\n"));
    }

    Ok(fasm_output.join("\n\n")) // Double newline for readability between nets
}

/// Helper: Extracts ``TILE.WIRE_IN.WIRE_OUT`` from two node IDs
fn nodes_to_fasm_line(node_a: &Node, node_b: &Node) -> Option<String> {
    // u_parts[0] = Wire Name, u_parts[1] = Coordinate (X1Y1)
    if node_a.tile == node_b.tile {
        Some(format!("{}.{}.{}", node_a.tile, node_a.id, node_b.id))
    } else {
        None
    }
}
