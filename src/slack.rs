use std::{collections::HashMap, path::Path};

use serde::Deserialize;

use crate::FabricResult;

/// The raw record format expected from the Timing Team's CSV
#[derive(Debug, Deserialize)]
struct SlackRecord {
    #[serde(rename = "source_wire")]
    source_wire: String,
    #[serde(rename = "slack_ps")]
    slack_ps: f32,
}

pub struct SlackReport {
    /// Mapping of Wire Name -> Slack in picoseconds
    pub slacks: HashMap<String, f32>,
}

impl SlackReport {
    /// Parses the CSV file from the timing team
    pub fn from_file<P: AsRef<Path>>(path: P) -> FabricResult<Self> {
        let mut rdr = csv::Reader::from_path(path)?;
        let mut slacks = HashMap::new();

        for result in rdr.deserialize() {
            let record: SlackRecord = result?;
            slacks.insert(record.source_wire, record.slack_ps);
        }

        Ok(SlackReport { slacks })
    }

    /// Helper to find the worst (most negative) slack for normalization
    pub fn get_worst_slack(&self) -> f32 {
        self.slacks
            .values()
            .cloned()
            .fold(0.0, |min, val| if val < min { val } else { min })
    }
    /// Returns a criticality value between 0.0 and 1.0 for a given wire.
    /// 1.0 = This is the most critical net in the design (worst slack).
    /// 0.0 = This net meets timing or is not in the report.
    pub fn calculate_criticality(&self, source_wire: &str) -> Option<f32> {
        let worst_slack = self.get_worst_slack();

        // If worst_slack is 0 or positive, the whole design meets timing.
        // Everyone gets 0.0 criticality.
        if worst_slack >= 0.0 {
            return None
        }

        if let Some(&slack) = self.slacks.get(source_wire) {
            if slack < 0.0 {
                // Formula: (current_slack / worst_negative_slack)
                // Example: (-500 / -1000) = 0.5 criticality
                // We use .min(1.0) just in case of rounding errors
                let base_crit = (slack / worst_slack).min(1.0);

                // Optional: Sharpening exponent.
                // Using crit^3 is common in FPGA tools to make the router
                // focus HARD on the top 10% of failing nets.
                return Some(base_crit.powf(3.0));
            }
        }

        None
    }
}
