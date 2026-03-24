import csv
import random
import argparse
import sys

def generate_mock_slack(fasm_path, output_path, target_ps):
    sources = set()
    try:
        with open(fasm_path, 'r') as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith('#'):
                    continue
                parts = line.split('.')
                if len(parts) >= 2:
                    source_wire = f"{parts[0]}.{parts[1]}"
                    sources.add(source_wire)
    except FileNotFoundError:
        print(f"Error: FASM file '{fasm_path}' not found.")
        sys.exit(2) # Exit code 2 for file errors

    worst_slack = float('inf')
    violation_count = 0

    with open(output_path, 'w', newline='') as csvfile:
        fieldnames = ['source_wire', 'slack_ps']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()

        for wire in sorted(sources):
            actual_delay = random.randint(2, 15) * 600
            slack = target_ps - actual_delay
            slack += random.randint(-500, 500)
            
            writer.writerow({'source_wire': wire, 'slack_ps': float(slack)})
            
            if slack < worst_slack:
                worst_slack = slack
            if slack < 0:
                violation_count += 1

    print("-" * 30)
    print(f"Worst Slack: {worst_slack} ps")
    
    if violation_count == 0:
        return True # Success
    else:
        return False # Failure

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate mock STA slack reports.")
    parser.add_argument("fasm", help="Input FASM file")
    parser.add_argument("output", help="Output CSV file")
    parser.add_argument("--target", type=int, default=5000, help="Target ps")
    args = parser.parse_args()
    
    # sys.exit(0) if passed, sys.exit(1) if failed
    if generate_mock_slack(args.fasm, args.output, args.target):
        sys.exit(0)
    else:
        sys.exit(1)
