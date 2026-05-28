use router::NodeId;

#[derive(Copy, Clone, Debug)]
pub struct Edge {
    pub source_node: NodeId,
    pub target_node: NodeId,
    pub start_position: vello::kurbo::Point,
    pub end_position: vello::kurbo::Point,
}
