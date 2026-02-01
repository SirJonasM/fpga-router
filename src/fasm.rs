use std::collections::HashSet;

use crate::fabric_graph::RoutingExpanded;


 /// Converts Expanded JSON-like structure to a FASM string
pub fn routing_to_fasm(expanded_nets: &[RoutingExpanded]) -> String {
    let mut fasm_lines = HashSet::new();

    for net in expanded_nets {
        if let Some(ref res) = net.result {
            for path in res.paths.values() {
                for pair in path.windows(2) {
                    if let Some(line) = nodes_to_fasm_line(&pair[0], &pair[1]) {
                        fasm_lines.insert(line);
                    }
                }
            }
        }
    }

    let mut sorted: Vec<String> = fasm_lines.into_iter().collect();
    sorted.sort();
    sorted.join("\n")
}
/// Helper: Extracts "TILE.WIRE_IN.WIRE_OUT" from two node IDs
fn nodes_to_fasm_line(u_id: &str, v_id: &str) -> Option<String> {
    let u_parts: Vec<&str> = u_id.split('.').collect();
    let v_parts: Vec<&str> = v_id.split('.').collect();

    if u_parts.len() < 2 || v_parts.len() < 2 { return None; }

    // u_parts[0] = Wire Name, u_parts[1] = Coordinate (X1Y1)
    if u_parts[0] == v_parts[0] {
        Some(format!("{}.{}.{}", u_parts[0], u_parts[1], v_parts[1]))
    } else {
        None
    }
}
