use std::collections::{HashMap, HashSet, VecDeque};
use crate::{Connection, Flop, Node, TimingNode};

#[derive(Debug)]
pub struct TimingAnalysisResult {
    pub min_paths: Vec<(f64, f64, Node, Node, Vec<TimingNode>)>,
    pub max_paths: Vec<(f64, f64, Node, Node, Vec<TimingNode>)>,
}

pub fn perform_timing_analysis(
    design: &HashMap<Node, HashMap<Node, Connection>>,
    _flops: &[Flop], // Keeping for signature, but using graph-based discovery
) -> TimingAnalysisResult {
    // 1. Map all nodes and track in-degrees/out-degrees
    let mut in_degree: HashMap<Node, usize> = HashMap::new();
    let mut out_degree: HashMap<Node, usize> = HashMap::new();
    let mut all_nodes: HashSet<Node> = HashSet::new();


    for (src, connections) in design {
        all_nodes.insert(src.clone());
        out_degree.insert(src.clone(), connections.len());
        in_degree.entry(src.clone()).or_insert(0);
        for dst in connections.keys() {
            all_nodes.insert(dst.clone());
            *in_degree.entry(dst.clone()).or_insert(0) += 1;
        }
    }

    // 2. Identify Sources: Nodes with 0 in-degree (Start of timing arcs)
    let timing_sources: Vec<Node> = all_nodes
        .iter()
        .filter(|node| *in_degree.get(node).unwrap_or(&0) == 0)
        .cloned()
        .collect();

    // 3. Identify Sinks: Nodes with 0 out-degree (End of timing arcs)
    let timing_sinks: Vec<Node> = all_nodes
        .iter()
        .filter(|node| *out_degree.get(node).unwrap_or(&0) == 0)
        .cloned()
        .collect();

    // 4. Propagation (SPFA/Relaxation)
    let mut max_delay: HashMap<Node, f64> = HashMap::new();
    let mut min_delay: HashMap<Node, f64> = HashMap::new();
    let mut max_predecessor: HashMap<Node, Node> = HashMap::new();
    let mut min_predecessor: HashMap<Node, Node> = HashMap::new();

    let mut queue: VecDeque<Node> = VecDeque::new();
    let mut in_queue: HashSet<Node> = HashSet::new();
    let mut update_count: HashMap<Node, usize> = HashMap::new();

    for src in &timing_sources {
        max_delay.insert(src.clone(), 0.0);
        min_delay.insert(src.clone(), 0.0);
        queue.push_back(src.clone());
        in_queue.insert(src.clone());
    }

    let node_limit = all_nodes.len().max(100);

    while let Some(u) = queue.pop_front() {
        in_queue.remove(&u);
        let u_max = *max_delay.get(&u).unwrap_or(&-1.0);
        let u_min = *min_delay.get(&u).unwrap_or(&f64::INFINITY);

        if let Some(connections) = design.get(&u) {
            for (v, conn) in connections {
                let mut changed = false;

                // Max Delay
                if u_max >= 0.0 {
                    let new_max = u_max + conn.delay;
                    if new_max > *max_delay.get(v).unwrap_or(&-1.0) {
                        max_delay.insert(v.clone(), new_max);
                        max_predecessor.insert(v.clone(), u.clone());
                        changed = true;
                    }
                }

                // Min Delay
                if u_min != f64::INFINITY {
                    let new_min = u_min + conn.delay;
                    if new_min < *min_delay.get(v).unwrap_or(&f64::INFINITY) {
                        min_delay.insert(v.clone(), new_min);
                        min_predecessor.insert(v.clone(), u.clone());
                        changed = true;
                    }
                }

                if changed {
                    let count = update_count.entry(v.clone()).or_insert(0);
                    *count += 1;
                    if *count <= node_limit && !in_queue.contains(v) {
                        queue.push_back(v.clone());
                        in_queue.insert(v.clone());
                    }
                }
            }
        }
    }

    // 5. Build Resulting Paths
    let mut max_paths = Vec::new();
    let min_paths = Vec::new();

    for sink in timing_sinks {
        if let Some(&arrival_max) = max_delay.get(&sink) {
            let mut path = Vec::new();
            let mut curr = Some(sink.clone());
            while let Some(p) = curr {
                path.push(TimingNode::from_node(&p, 
                    *max_delay.get(&p).unwrap_or(&0.0), 
                    *min_delay.get(&p).unwrap_or(&0.0)
                ));
                curr = max_predecessor.get(&p).cloned();
            }
            path.reverse();
            let src = path.first().map(|n| Node { tile: n.tile, pin: n.pin.clone() }).unwrap_or(sink.clone());
            let arrival_min = *min_delay.get(&sink).unwrap_or(&0.0);
            max_paths.push((arrival_min, arrival_max, src, sink.clone(), path));
        }
    }

    TimingAnalysisResult { min_paths, max_paths }
}

impl TimingAnalysisResult {
    pub fn to_slack_csv(&self, target_period_ps: f64) -> String {
        let mut csv = String::from("source_node,sink_node,slack_ps,criticality\n");
        let max_arrival = self.max_paths.iter().map(|p| p.1).fold(0.0, f64::max); 
        for (_, arrival, src, sink, path) in &self.max_paths {
            let slack = target_period_ps - arrival;
            let criticality = (arrival / max_arrival).clamp(0.0, 1.0);

            // MAP THE SINK:
            // If the sink is a 'FF_SINK', the router actually needs to know 
            // the wire-end (the LUT input) that fed it.
            let router_sink = if sink.pin.contains("FF_SINK") {
                // Get the node immediately BEFORE the FF_SINK in the timing path
                path.iter().rev().nth(2).map(|tn| format!("X{}Y{}.{}", tn.tile.0, tn.tile.1, tn.pin))
                    .unwrap_or_else(|| format!("X{}Y{}.{}", sink.tile.0, sink.tile.1, sink.pin))
            } else {
                format!("X{}Y{}.{}", sink.tile.0, sink.tile.1, sink.pin)
            };

            let source_str = format!("X{}Y{}.{}", src.tile.0, src.tile.1, src.pin);
            
            csv.push_str(&format!(
                "{},{},{:.1},{:.3}\n", 
                source_str, 
                router_sink, 
                slack, 
                criticality
            ));
        }
        csv
    }
}
