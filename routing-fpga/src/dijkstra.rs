use std::{cmp::Ordering, collections::BinaryHeap};

use crate::fabric_graph::FabricGraph;

impl FabricGraph {
    pub fn dijkstra(&self, start: usize, end: usize) -> Option<(Vec<usize>, f32)> {
        let n = self.nodes.len();

        let mut dist: Vec<f32> = vec![f32::MAX; n];
        let mut prev: Vec<Option<usize>> = vec![None; n]; // <-- store predecessors

        let mut heap = BinaryHeap::new();

        let mut max_frontier = 0usize;

        dist[start] = 0.0;
        heap.push(State {
            cost: 0.0,
            position: start,
        });

        while let Some(State { cost, position }) = heap.pop() {
            // Track frontier growth
            if heap.len() > max_frontier {
                max_frontier = heap.len();
            }

            // If popped outdated distance, skip
            if cost > dist[position] {
                continue;
            }

            // Reached destination → reconstruct path
            if position == end {
                let mut path_indices = Vec::new();
                let mut current = Some(end);

                while let Some(idx) = current {
                    path_indices.push(idx);
                    current = prev[idx];
                }

                path_indices.reverse();

                return Some((path_indices, cost));
            }

            // Expand adjacency list
            for edge in &self.map[position] {
                let base_cost = edge.cost;
                let next_cost = cost + self.costs[edge.node_id].calc_costs(base_cost);
                let next_pos = edge.node_id;

                if next_cost < dist[next_pos] {
                    dist[next_pos] = next_cost;
                    prev[next_pos] = Some(position); 
                    heap.push(State {
                        cost: next_cost,
                        position: next_pos,
                    });
                }
            }
        }

        None
    }

    pub fn dijkstra_all(&self, start: usize) -> Vec<f32>{
        let n = self.nodes.len();

        let mut dist: Vec<f32> = vec![f32::MAX; n];
        let mut heap = BinaryHeap::new();

        dist[start] = 0.0;
        heap.push(State {
            cost: 0.0,
            position: start,
        });

        while let Some(State { cost, position }) = heap.pop() {
            if cost > dist[position] {
                continue;
            }

            for edge in &self.map_reversed[position] {
                let base_cost = edge.cost;
                let next_cost = cost + self.costs[edge.node_id].calc_costs(base_cost);

                let next_pos = edge.node_id;

                if next_cost < dist[next_pos] {
                    dist[next_pos] = next_cost;
                    heap.push(State {
                        cost: next_cost,
                        position: next_pos,
                    });
                }
            }
        }

        dist
    }

    

}
// PriorityQueue state
#[derive(Clone)]
struct State {
    cost: f32,
    position: usize,
}
impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.cost.to_bits() == other.cost.to_bits()
    }
}

impl Eq for State {}
// Implement ordering so BinaryHeap acts as min-heap
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // total ordering: treat NaN as +∞
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Greater)
    }
}
impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
