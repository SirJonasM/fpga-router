use std::{collections::HashMap, path::Path};

use serde::Deserialize;

use crate::{FabricError, FabricGraph, FabricResult, graph::node::NodeId};

/// The raw record format expected from the Timing Team's CSV
#[derive(Debug, Deserialize)]
struct SlackRecord {
    source_wire: String,
    slack_ps: f32,
}

pub struct SlackReport {
    /// Mapping of Wire Name -> Slack in picoseconds
    pub slacks: HashMap<NodeId, f32>,
    pub worst_slack: (NodeId, f32),
}

impl SlackReport {
    /// Parses the CSV file from the timing team
    ///
    /// # Errors
    /// When it cannot deserialize the CSV file
    pub fn from_file<P: AsRef<Path>>(path: P, graph: &FabricGraph) -> FabricResult<Self> {
        let mut rdr = csv::Reader::from_path(path)?;
        let mut slacks = HashMap::new();
        let mut worst_slack = (String::new(), f32::INFINITY);

        for result in rdr.deserialize() {
            let record: SlackRecord = result?;
            let source = graph
                .get_node_id(&record.source_wire)
                .ok_or_else(||FabricError::InvalidStringNodeId(record.source_wire.clone()))?;
            slacks.insert(*source, record.slack_ps);
            if worst_slack.1 > record.slack_ps {
                worst_slack = (record.source_wire, record.slack_ps);
            }
        }

        let worst_slack_id = *graph
            .get_node_id(&worst_slack.0)
            .ok_or(FabricError::InvalidStringNodeId(worst_slack.0))?;
        let worst_slack = (worst_slack_id, worst_slack.1);
        Ok(Self { slacks, worst_slack })
    }

    /// Returns a criticality value between 0.0 and 1.0 for a given wire.
    /// 1.0 = This is the most critical net in the design (worst slack).
    /// 0.0 = This net meets timing or is not in the report.
    #[must_use]
    pub fn calculate_criticality(&self, source_wire: &NodeId) -> f32 {
        let worst_slack = self.worst_slack.1;

        if worst_slack >= 0.0 {
            return 0.0;
        }

        if let Some(&slack) = self.slacks.get(source_wire)
            && slack < 0.0
        {
            let base_crit = (slack / worst_slack).min(1.0);

            return base_crit.powi(3);
        }

        0.0
    }
}
