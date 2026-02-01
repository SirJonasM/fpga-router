use std::error::Error;
use std::process::Command;

use std::{
    fs::File,
    io::Write,
};

use crate::IterationResult;


pub fn generate_typst_table(
    filename: &str,
    percentages: &[usize],
    dest_counts: &[usize],
    results: &[Result<IterationResult, IterationResult>],
) -> std::io::Result<()> {
    let mut file = File::create(filename)?;

    // 1. Write Header
    writeln!(file, "#set page(width: auto, height: auto, margin: 1cm)")?;
    writeln!(file, "#align(center)[")?;

    for (r_idx, res) in results.iter().enumerate() {
            match res {
                Ok(res) => {
                    // Green cell for success
                    write!(
                        file,
                        "#let p{}_d{} = table.cell(
  fill: green.lighten(60%),
)[Iterations: {}  Longest Path: {} wire reuse: {}  total wire use: {}]\n",
                        res.test_case.percentage, res.test_case.dst, res.iteration, res.longest_path_cost, res.wire_reuse, res.total_wire_use
                    )?;
                }
                Err(res) => {
                    // Red cell for failure
                    write!(
                        file,
                        "#let p{}_d{} = table.cell(
  fill: red.lighten(60%),
)[{}]\n",
                        res.test_case.percentage, res.test_case.dst, res.conflicts
                    )?;
                }
        }
    }

    // 2. Start Table
    // Columns: 1 for Label + N for dest counts
    let num_cols = dest_counts.len() + 1;
    writeln!(file, "  #table(")?;
    writeln!(file, "    columns: ({}),", "auto, ".repeat(num_cols))?;
    writeln!(file, "    inset: 10pt,")?;
    writeln!(file, "    align: center + horizon,")?;

    // 3. Table Header Row
    write!(file, "    [*Load*]")?; // Top left corner
    for d in dest_counts {
        write!(file, ", [*{} Dest*]", d)?;
    }
    writeln!(file, ",")?;

    // 4. Data Rows
    for (r_idx, perc) in percentages.iter().enumerate() {
        // Row Label (Percentage)
        write!(file, "    [*{:.0}%*]", perc )?;

        // Results
        for c_idx in 0..dest_counts.len() {
            write!(file, ", p{}_d{}", perc, c_idx + 1)?;
        }
        writeln!(file, ",")?;
    }

    // 5. Close Table
    writeln!(file, "  )")?;
    writeln!(file, "]")?;

    Ok(())
}

/// Executes the typst compilation command.
pub fn compile_typst(filename: &str) -> Result<(), Box<dyn Error>> {
    println!("Compiling Typst file: typst compile {} -f svg", filename);

    let output = Command::new("typst")
        .arg("compile")
        .arg(filename)
        .arg("-f")
        .arg("svg")
        .output()?; // The '?' propagates any I/O error when running the command

    if output.status.success() {
        Ok(())
    } else {
        // Collect stderr for diagnostic output if the command fails
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Typst command failed with status: {}\n{}", output.status, stderr).into())
    }
}
