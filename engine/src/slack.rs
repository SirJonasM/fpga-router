use std::collections::HashMap;

use crate::graph::node::NodeId;

pub struct SlackReport {
    /// Mapping of NodeId -> Slack in picoseconds
    pub slacks: HashMap<(NodeId, NodeId), f32>,
    /// Mapping of NodeId -> Criticality (0.0 to 1.0)
    pub criticalities: HashMap<(NodeId, NodeId), f32>,
    pub worst_slack: ((NodeId, NodeId), f32),
}
