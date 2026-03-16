import csv
import random
import argparse
import sys

def generate_mock_slack(fasm_path, output_path, target_ps):
    """
    Parses a FASM file and generates a mock slack report using full wire names.
    """
    # In a real scenario, the STA team would know which wires are 'sources'
    # For this mock, we will collect the first wire of every FASM line
    # as a potential net source.
    sources = set()

    try:
        with open(fasm_path, 'r') as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith('#'):
                    continue
                
                # Split 'X1Y1.E2END3.J_l_CD_BEG0' into parts
                parts = line.split('.')
                if len(parts) >= 2:
                    # Reconstruct the full wire name: Tile.Wire
                    # Example: 'X1Y1.E2END3'
                    source_wire = f"{parts[0]}.{parts[1]}"
                    sources.add(source_wire)
                    
    except FileNotFoundError:
        print(f"Error: FASM file '{fasm_path}' not found.")
        sys.exit(1)

    with open(output_path, 'w', newline='') as csvfile:
        fieldnames = ['source_wire', 'slack_ps']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()

        # We'll generate a random hop count for each source to simulate path length
        for wire in sorted(sources):
            # Simulated path length (hops) between 2 and 15
            hop_count = random.randint(2, 15)
            actual_delay = hop_count * 600
            
            # Slack calculation
            slack = target_ps - actual_delay
            slack += random.randint(-500, 500)
            
            writer.writerow({
                'source_wire': wire, 
                'slack_ps': float(slack)
            })

    print(f"Mock slack report generated at: {output_path}")
    print(f"Detected {len(sources)} unique source wires.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate mock STA slack reports from FASM output.")
    parser.add_argument("fasm", help="Input FASM file path")
    parser.add_argument("output", help="Output CSV file path")
    parser.add_argument("--target", type=int, default=5000, help="Target clock period in ps")

    args = parser.parse_args()
    generate_mock_slack(args.fasm, args.output, args.target)
