use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashSet, VecDeque},
};

use crate::{node::Edge, FabricGraph};

impl FabricGraph {
    pub fn breadth_first_search(&self, start: usize, end: usize) -> Option<(usize, usize, usize)> {
        let mut max_frontier = 0usize;
        let mut lookups = 0usize;

        let n = self.nodes.len();
        let mut visited = vec![false; n];
        let mut prev: Vec<Option<usize>> = vec![None; n];

        let mut queue = VecDeque::new();

        visited[start] = true;
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            max_frontier = max_frontier.max(queue.len());

            // Count node expansion
            lookups += 1;

            if node == end {
                // Reconstruct path
                let mut path_length = 0;
                let mut current = Some(end);

                while let Some(idx) = current {
                    path_length += 1;
                    current = prev[idx];
                }

                return Some((lookups, max_frontier, path_length));
            }

            for edge in &self.map[node] {
                let next = edge.node_id;
                if !visited[next] {
                    visited[next] = true;
                    prev[next] = Some(node);
                    queue.push_back(next);
                }
            }
        }

        None
    }

    pub fn depth_first_search(&self, start: usize, end: usize) -> Option<(usize, usize)> {
        let mut visited = HashSet::new();
        let mut lookups = 0usize;
        let mut max_frontier = 0usize;
        let mut path = Vec::new();

        fn dfs(
            graph: &Vec<Vec<Edge>>,
            current: usize,
            end: usize,
            visited: &mut HashSet<usize>,
            path: &mut Vec<usize>,
            lookups: &mut usize,
            max_frontier: &mut usize,
        ) -> bool {
            // Node expansion
            *lookups += 1;

            visited.insert(current);
            path.push(current);

            // Frontier = recursion depth
            *max_frontier = (*max_frontier).max(path.len());

            if current == end {
                return true;
            }

            for edge in &graph[current] {
                let next = edge.node_id;
                if !visited.contains(&next) && dfs(graph, next, end, visited, path, lookups, max_frontier) {
                    return true;
                }
            }

            path.pop();
            false
        }

        if dfs(
            &self.map,
            start,
            end,
            &mut visited,
            &mut path,
            &mut lookups,
            &mut max_frontier,
        ) {
            Some((lookups, max_frontier))
        } else {
            None
        }
    }
    pub fn dijkstra_verbose(&self, start: usize, end: usize) -> Option<(usize, usize, usize)> {
        let mut max_frontier = 0usize;
        let mut lookups = 0usize;

        let n = self.nodes.len();

        let mut dist: Vec<f32> = vec![f32::MAX; n];
        let mut prev: Vec<Option<usize>> = vec![None; n];

        let mut heap = BinaryHeap::new();

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
            lookups += 1;

            // Reached destination → reconstruct path
            if position == end {
                let mut path_indices = Vec::new();
                let mut current = Some(end);

                while let Some(idx) = current {
                    path_indices.push(idx);
                    current = prev[idx];
                }

                path_indices.reverse();

                return Some((max_frontier, lookups, path_indices.len()));
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
