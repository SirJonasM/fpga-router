use regex::Regex;
use std::{fs::{self, File}, io::BufReader};

use crate::{Configuration, Flop, Node, Pip, TimingConstraints, TimingModel};

pub fn pips_parser(file_path: &str) -> Vec<Pip> {
    let pips_file_content = fs::read_to_string(file_path).unwrap_or_else(|_| "".to_string());
    let mut pips: Vec<Pip> = Vec::new();
    // Regex to match PIPs like X1Y0,N1END3,X1Y0,S1BEG0
    let pip_regex = match Regex::new(r"^X(\d+)Y(\d+),([^,]+),X(\d+)Y(\d+),([^,]+),(\d+(?:\.\d+)?)")
    {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };

    for line in pips_file_content.lines() {
        if line.starts_with('#') || line.is_empty() || line.contains('=') {
            continue;
        }
        if let Some(caps) = pip_regex.captures(line) {
            let src_x: u32 = caps[1].parse().unwrap_or(0);
            let src_y: u32 = caps[2].parse().unwrap_or(0);
            let src_pin = caps[3].to_string();
            let dst_x: u32 = caps[4].parse().unwrap_or(0);
            let dst_y: u32 = caps[5].parse().unwrap_or(0);
            let dst_pin = caps[6].to_string();
            let delay: f64 = caps[7].parse().unwrap_or(0.0);

            pips.push(Pip {
                src: Node {
                    tile: (src_x, src_y),
                    pin: src_pin,
                },
                dst: Node {
                    tile: (dst_x, dst_y),
                    pin: dst_pin,
                },
                delay,
            });
        }
    }
    pips
}

pub fn fasm_parser(file_path: &str) -> Result<(Vec<Configuration>, Vec<Flop>), ()> {
    let fasm_file_content = fs::read_to_string(file_path).unwrap_or_else(|_| "".to_string());
    fasm_parser_string(&fasm_file_content)
}

pub fn fasm_parser_string(content: &str) -> Result<(Vec<Configuration>, Vec<Flop>), ()> {
    let mut configurations: Vec<Configuration> = Vec::new();
    let mut flops: Vec<Flop> = Vec::new();

    let pip_regex = match Regex::new(r"^X(\d+)Y(\d+)\.([a-zA-Z0-9_\[\]:]+)\.([a-zA-Z0-9_\[\]:]+)$")
    {
        Ok(re) => re,
        Err(_) => return Err(()),
    };
    let ff_regex = match Regex::new(r"^X(\d+)Y(\d+)\.([a-zA-Z0-9_]+)\.FF$") {
        Ok(re) => re,
        Err(_) => return Err(()),
    };

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() || line.contains('=') {
            continue;
        }

        if let Some(caps) = ff_regex.captures(line) {
            let x: u32 = caps[1].parse().unwrap_or(0);
            let y: u32 = caps[2].parse().unwrap_or(0);
            let lut_name = caps[3].to_string();
            // Extract the letter (e.g., 'B' in "LB" or just "B")
            let lut_char = if lut_name.len() > 1 && lut_name.starts_with('L') {
                lut_name.chars().nth(1).unwrap_or('A')
            } else {
                lut_name.chars().next().unwrap_or('A')
            };
            let lut = lut_char as u32 - 'A' as u32;

            flops.push(Flop { tile: (x, y), lut });
        } else if let Some(caps) = pip_regex.captures(line) {
            let x: u32 = caps[1].parse().unwrap_or(0);
            let y: u32 = caps[2].parse().unwrap_or(0);
            let src_wire = caps[3].to_string();
            let dst_wire = caps[4].to_string();

            configurations.push(Configuration {
                tile: (x, y),
                src_pin: src_wire,
                dst_pin: dst_wire,
            });
        }
    }

    Ok((configurations, flops))
}

pub fn parse_timing_model(file_path: &str) -> TimingModel {
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    serde_json::from_reader(reader).unwrap()
}

pub fn parse_all_timing_models(dir_path: &str) -> Vec<(String, TimingModel)> {
    let mut models = Vec::new();
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let model = parse_timing_model(path.to_str().unwrap());
                models.push((name, model));
            }
        }
    } else {
        log::error!("Failed to read timing model directory: {}", dir_path);
    }
    models.sort_by(|a, b| a.0.cmp(&b.0));
    models
}

pub fn parse_timing_constraints(file_path: &str) -> TimingConstraints {
    let constraints_file_content = fs::read_to_string(file_path).unwrap_or_else(|_| "".to_string());
    serde_json::from_str(&constraints_file_content).unwrap_or(TimingConstraints {
        setup_time: 50.0,
        hold_time: 10.0,
        clk_period: 20.0,
    })
}

pub fn parse_all_timing_constraints(dir_path: &str) -> Vec<(String, TimingConstraints)> {
    let mut constraints_list = Vec::new();
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let constraints = parse_timing_constraints(path.to_str().unwrap());
                constraints_list.push((name, constraints));
            }
        }
    } else {
        log::error!("Failed to read timing constraints directory: {}", dir_path);
    }
    constraints_list.sort_by(|a, b| a.0.cmp(&b.0));
    constraints_list
}
