import json
import sys
import os
from collections import defaultdict

def get_physical_pin(cell_name, port_name, cells):
    cell_data = cells.get(cell_name, {})
    attrs = cell_data.get("attributes", {})
    bel = attrs.get("NEXTPNR_BEL", "")
    cell_type = cell_data.get("type")

    # 1. Handle Constant/Global Drivers
    if "_CONST1_DRV" in bel:
        tile = bel.split('/')[0]
        return f"{tile}.VCC0"
    if "_CONST0_DRV" in bel:
        tile = bel.split('/')[0]
        return f"{tile}.GND0"
    
    # Identify Clock signals (to be filtered later)
    if "clk" in cell_name.lower() or "clk" in port_name.lower():
        tile = bel.split('/')[0] if "/" in bel else "X0Y0"
        return f"{tile}.Ci0"

    # 2. Handle Standard Cells (Logic and IO)
    if not bel or "/" not in bel:
        return f"UNPLACED_{cell_name}"
        
    tile_coord, sub_bel = bel.split('/')
    
    if cell_type == "FABULOUS_LC":
        # Map Q to O as per your requirement to treat them as the same routing node
        return f"{tile_coord}.L{sub_bel}_{port_name.replace('Q', 'O')}"
    elif cell_type == "IO_1_bidirectional_frame_config_pass":
        return f"{tile_coord}.{sub_bel}_{port_name}"
    
    return f"{tile_coord}.{sub_bel}_{port_name}"

def extract_fasm_and_nets(json_path):
    if not os.path.exists(json_path):
        print(f"Error: File {json_path} not found.")
        return

    with open(json_path, 'r') as f:
        data = json.load(f)

    modules = data.get("modules", {})
    if not modules: return
    
    module_name = list(modules.keys())[0]
    cells = modules[module_name].get("cells", {})

    net_to_source = {}
    net_to_sinks = defaultdict(list)
    fasm_lines = []

    for cell_name, cell_data in cells.items():
        cell_type = cell_data.get("type")
        connections = cell_data.get("connections", {})
        port_dirs = cell_data.get("port_directions", {})
        attrs = cell_data.get("attributes", {})
        params = cell_data.get("parameters", {})

        # --- FASM GENERATION (FFs and INITs) ---
        if cell_type == "FABULOUS_LC":
            bel = attrs.get("NEXTPNR_BEL", "")
            if bel and "/" in bel:
                tile_coord, sub_bel = bel.split('/')
                
                # 1. Handle INIT string
                init_val = params.get("INIT")
                if init_val:
                    # nextpnr JSON INIT is usually a bitstring, we format it for FASM
                    fasm_lines.append(f"{tile_coord}.{sub_bel}.INIT[15:0] = 16'b{init_val}")
                
                # 2. Handle Flip-Flop Toggle
                # We check if 'FF' parameter is 1 (ignoring leading zeros in JSON bitstring)
                ff_param = params.get("FF", "0")
                if "1" in ff_param:
                    fasm_lines.append(f"{tile_coord}.{sub_bel}.FF")

        # --- NETLIST MAPPING ---
        for port_name, nids in connections.items():
            if not nids: continue
            nid = nids[0]
            
            # Use actual direction from JSON, override for IO as before
            direction = port_dirs.get(port_name)

            phys_pin = get_physical_pin(cell_name, port_name, cells)
            if direction == "output":
                net_to_source[nid] = phys_pin
            else:
                net_to_sinks[nid].append(phys_pin)

    # --- PROCESS RUST PLAN ---
    rust_plan = []
    for nid, source_phys in net_to_source.items():
        if ".Ci0" in source_phys: continue # Ignore Clocks

        sinks_phys_list = net_to_sinks.get(nid, [])
        if not sinks_phys_list: continue

        if any(m in source_phys for m in [".GND0", ".VCC0"]) and source_phys.startswith("X0Y0"):
            localized_nets = defaultdict(list)
            signal_type = source_phys.split('.')[-1]
            for sink in sinks_phys_list:
                if ".Ci0" in sink: continue
                sink_tile = sink.split('.')[0]
                localized_nets[sink_tile].append(sink)
            for tile, localized_sinks in localized_nets.items():
                rust_plan.append({"signal": f"{tile}.{signal_type}", "sinks": localized_sinks})
        else:
            filtered_sinks = [s for s in sinks_phys_list if ".Ci0" not in s]
            if filtered_sinks:
                rust_plan.append({"signal": source_phys, "sinks": filtered_sinks})

    # --- WRITE OUTPUTS ---
    with open("net-list.json", "w") as f:
        json.dump({"hash": None, "plan": rust_plan}, f, indent=4)

    with open("ffs.fasm", "w") as f:
        f.write("\n".join(fasm_lines))

    print(f"[Success] Exported {len(rust_plan)} nets to net-list.json")
    print(f"[Success] Exported {len(fasm_lines)} lines to ffs.fasm")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 script.py <path_to_json>")
    else:
        extract_fasm_and_nets(sys.argv[1])
