use std::collections::HashSet;

use crate::{FabricError, FabricResult, netlist::NetListExternal};

pub fn net_to_fasm(expanded_nets: &NetListExternal) -> FabricResult<String> {
    let mut fasm_lines = expanded_nets
        .plan
        .iter()
        .map(|a| a.result.as_ref().ok_or(FabricError::NetNotSolved))
        .collect::<FabricResult<Vec<_>>>()?
        .into_iter()
        .flat_map(|a| a.paths.values())
        .flat_map(|a| a.windows(2))
        .map(|pair| nodes_to_fasm_line(&pair[0], &pair[1]))
        .collect::<FabricResult<HashSet<Option<String>>>>()?
        .into_iter().flatten()
        .collect::<Vec<String>>();
    fasm_lines.sort();
    Ok(fasm_lines.join("\n"))
}

/// Helper: Extracts ``TILE.WIRE_IN.WIRE_OUT`` from two node IDs
fn nodes_to_fasm_line(u_id: &str, v_id: &str) -> FabricResult<Option<String>> {
    let u_parts: Vec<&str> = u_id.split('.').collect();
    let v_parts: Vec<&str> = v_id.split('.').collect();

    if u_parts.len() < 2 {
        return Err(FabricError::InvalidStringNodeId(u_id.to_string()));
    }
    if v_parts.len() < 2 {
        return Err(FabricError::InvalidStringNodeId(v_id.to_string()));
    }

    // u_parts[0] = Wire Name, u_parts[1] = Coordinate (X1Y1)
    if u_parts[0] == v_parts[0] {
        Ok(Some(format!("{}.{}.{}", u_parts[0], u_parts[1], v_parts[1])))
    } else {
        Ok(None)
    }
}
