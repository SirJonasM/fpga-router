#let title-page(title: [], subtitle: [], email: str, author: [], body) = {
  set page(margin: (top: 1.5in, rest: 2in))
  set heading(numbering: "1.1.1")
  line(start: (0%, 0%), end: (8.5in, 0%), stroke: (thickness: 2pt))
  align(horizon + left)[
    #text(size: 24pt, title)\
    #v(1em)
    #text(size: 16pt, subtitle)\
    #v(1em)
    #text(size: 16pt, author)
    #linebreak()
    #v(1em)
    #linebreak()
    #link("mailto:" + email, email)
  ]
  align(bottom + left)[#datetime.today().display()]
  set page(fill: none, margin: auto)
  pagebreak()
  body
}

#show: body => title-page(
  title: [FPGA Routing],
  subtitle: [Using Steiner Trees to implement FPGA routing],
  email: "jonas.moewes@stud.uni-heidelberg.de",
  author: [Jonas Möwes],
  body,
)
#set text(size: 11pt)
#set heading(numbering: "1.1.")


= Introduction
This project investigates the critical problem of routing in Field-Programmable Gate Arrays (FPGAs), focusing on efficiently connecting required logical elements using the limited resources of the FPGA fabric. The routing process is essential for implementing a functional circuit on an FPGA.

The foundation of this work is the FabricGraph , a graph structure used to model the FPGA's routing resources, such as wires and programmable interconnection points (PIPs). The graph is implemented using an adjacency list structure (map and map_reversed) and assigns costs to nodes to guide path-finding.

== Key Areas of Evaluation

The project is structured around two main areas of investigation:

1. Path Finding Algorithms: We evaluate standard path-finding algorithms—Breadth-First Search (BFS), Depth-First Search (DFS), and Dijkstra's algorithm—based on their search effort and path quality. This comparison aims to identify the most suitable base algorithm for single-path routing in an FPGA context. The results demonstrate that Dijkstra's algorithm provides the best trade-off between path optimality and search efficiency.

2. Iterative Global Routing with PathFinder: The core of the routing solution is the PathFinder algorithm. PathFinder is a negotiation-based, iterative method that addresses congestion by updating path costs using a Present cost ($c_p$) and Historic cost ($c_h$). This project utilizes the approach from the revisited PathFinder algorithm, which showed superior results in testing.


== Minimizing Cost with Steiner Trees
To efficiently connect a signal's source to its potentially multiple sinks, the concept of Steiner Trees is introduced. A Steiner Tree minimizes the total edge length required to connect a set of terminals. This approach is implemented in the Steiner Router and is compared against a simpler approach, the Simple Router, which connects each sink individually using Dijkstra's algorithm.

The final results assess both implementations based on metrics such as Longest path length, Total wire usage, and Wire reuse per signal , providing an overall assessment of the trade-offs between path quality, resource efficiency, and routing completion success.

= Graph Implementation
The FabricGraph struct represents the FPGA routing graph used in the project. Each field serves a specific purpose in modeling the FPGA fabric and supporting routing algorithms.
```rust
pub struct FabricGraph {
    pub lut_inputs: Vec<usize>,
    pub lut_outputs: Vec<usize>,
    pub index: HashMap<Node, usize>,
    pub costs: Vec<Costs>,
    pub nodes: Vec<GraphNode>,
    pub map: Vec<Vec<Edge>>,
    pub map_reversed: Vec<Vec<Edge>>,
}
```
== Fields Description
*lut_inputs & lut_outputs:* \ 
These vectors store the indices of LUT input and output nodes. They are mainly used for generating routing plans. Routing tests use paths that start at LUT outputs (lut_outputs) and end at LUT inputs (lut_inputs).

*index:* A hashmap that maps a Node to its unique index in the graph. This allows efficient lookup of node IDs.
Example: ```rust index.get(&Node {id: "LA_O", x: 1, y: 1})``` retrieves the index of this node in the graph.

*costs:* Stores the cost associated with each node. Costs are used during routing to evaluate path quality and guide algorithms like Dijkstra or A\*.

*nodes:* A list of all nodes in the graph. Each GraphNode represents a physical or logical element in the FPGA fabric.

*map:* An adjacency list representing forward edges from each node. It stores all direct connections from a node to its neighbors.

*map_reversed:* Another adjacency list, but with edges reversed. This is useful for algorithms that need to traverse the graph backward, such as reverse searches or certain routing heuristics.

= Path Finding Algorithms
Path-finding algorithms operate on a graph structure and take as input a start node and one or more target nodes.
Their goal is to determine a sequence of connected edges that forms a valid path between the start and the destination.
Depending on the algorithm, the objective may differ, such as merely finding any valid path or finding a path that minimizes a specific cost metric (e.g., number of routing resources or estimated delay).

In the context of FPGA routing, the graph represents routing resources such as wires and programmable interconnection points (PIPs).
The efficiency of a path-finding algorithm is therefore crucial, as the routing graph grows rapidly with fabric size and routing congestion.
In this work, Breadth-First Search (BFS), Depth-First Search (DFS), and Dijkstra’s algorithm are evaluated and compared with respect to search effort and path quality.

== Results on Path Finding Algorithms
The following tables summarize the experimental results for routing from a fixed source to different destination positions on the FPGA fabric. For each algorithm, we report the number of dictionary lookups, the maximum size of the search frontier, and the final path cost.

Since BFS and DFS operate on unweighted edges, while Dijkstra’s algorithm is designed for weighted graphs, the reported path costs naturally differ. To make a fair comparison, we recalculate Dijkstra’s path cost based solely on the number of nodes along the path.
#align(center)[#grid(columns: 2, gutter: 15pt, [#figure(
  table(
    columns: 4,
    [Algorithm], [Lookups], [Max Frontier], [Cost],
    [Dijkstra], [129], [237], [6],
    [BFS], [303], [226], [6],
    [DFS], [1188], [-], [899],
  ),
  caption: [End Node LA_I0 (1,1)],
)],
 [#figure(
  table(
    columns: 4,
    [Algorithm], [Lookups], [Max Frontier], [Cost],
    [Dijkstra], [497], [1634], [10],
    [BFS], [1623], [558], [10],
    [DFS], [1158], [-], [877],
  ),
  caption: [End Node LA_I0 (3,1)],
)],
[#figure(
  table(
    columns: 4,
    [Algorithm], [Lookups], [Max Frontier], [Cost],
    [Dijkstra], [656], [3783], [14],
    [BFS], [4161], [674], [14],
    [DFS], [1546], [-], [1177],
  ),
  caption: [End Node LA_I0 (1,4)],
)],
[#figure(
  table(
    columns: 4,
    [Algorithm], [Lookups], [Max Frontier], [Cost],
    [Dijkstra], [663], [6706], [20],
    [BFS], [7091], [674], [20],
    [DFS], [2500], [-], [1837],
  ),
  caption: [End Node LA_I0 (3,4)],
)])]

== Discussion of Results
=== Comparison of Path Quality
Both BFS and Dijkstra’s algorithm consistently find paths with identical costs for all tested destinations. This is expected, as BFS produces optimal solutions when all edges have equal cost, which is effectively the case in these experiments. Dijkstra generalizes this behavior and also guarantees shortest paths, even when weighted costs are used.

In contrast, DFS produces paths with significantly higher costs. Since DFS explores paths deeply without considering cost or distance to the target, it often traverses long and inefficient routes before reaching the destination. As a result, DFS is unsuitable for FPGA routing when path quality or resource efficiency is important.

=== Search Effort and Scalability
The number of dictionary lookups increases with the distance of the destination from the source for all algorithms. However, the growth rate differs significantly:
- BFS exhibits a rapid increase in lookups as the search radius expands. This is due to its level-by-level exploration, which causes a large portion of the routing graph to be explored before reaching distant targets.
- Dijkstra’s algorithm consistently requires fewer lookups than BFS for the same destinations. By prioritizing nodes with lower accumulated cost, it avoids unnecessary exploration and converges more efficiently toward the target.
- DFS performs the worst in terms of lookup count, particularly for distant destinations. Its lack of directionality leads to extensive exploration of irrelevant branches.
These results indicate that BFS and DFS do not scale well for larger fabrics or complex routing tasks, while Dijkstra offers better scalability.

=== Frontier Size Behavior
The maximum frontier size highlights memory requirements during routing:
- Dijkstra’s algorithm shows a rapidly increasing frontier size as the destination moves farther away. This is caused by the priority queue storing many candidate nodes with similar costs.
- BFS maintains a comparatively smaller and more stable frontier size, as it only stores nodes at the current depth level.
- DFS does not maintain a meaningful frontier in the same sense and is therefore omitted from this metric.
While Dijkstra reduces total exploration effort, it does so at the cost of increased memory usage, which becomes an important consideration for large FPGA fabrics.

=== Overall Assessment
From these experiments, the following conclusions can be drawn:
- DFS is not suitable for FPGA routing due to poor path quality and excessive exploration.
- BFS guarantees shortest paths but suffers from high lookup counts and limited scalability.
- Dijkstra’s algorithm provides the best trade-off between path optimality and search efficiency, making it more appropriate for FPGA routing, especially when extended with heuristics such as A\*.
= PathFinder
== PathFinder Algorithm
The PathFinder algorithm is a negotiation-based, iterative method for routing signals on a graph to their respective sinks @test2.
A routing plan consists of a set of signals and their corresponding sinks (note that a signal may have multiple sinks).

The negotiation mechanism in PathFinder relies on two types of costs:
- Present cost ($c_p$): reflects congestion within the current iteration.
- Historic cost ($c_h$): reflects congestion accumulated over previous iterations.

*Iterative Routing Process*

In each iteration, the algorithm routes each signal from its source to its sinks.
Often this is described as that a local router is routing a signal. 
The implementation of the local router is an implementation detail that will be discussed later.
In contrast the Global Router is the one that reevaluates the costs for the next iteration.

After routing a signal, the algorithm updates the costs of all nodes used in that routing.
The cost formulas are as follows:
- *present cost:* $c_p = 1.0 plus "max"(0.0, "usage" - "capacity" ) times "present_factor" $\
- *historic cost:* $c_h = c_h plus "max"(0.0, "usage" - "capacity" ) times "historic_factor" $
- *Total node cost:* $c_n = (b plus c_h) times c_p$

where $b$ is the cost of the edge (base cost).
The updated costs are used to address two different congestion problems, as described in the original study.
In this project the capacity is always 1.

== Revisited PathFinder Algorithm
The paper “Revisiting PathFinder Routing Algorithm” proposes several improvements @test:

1. Both traditional congestion problems can be effectively handled using only the historic cost.
2. A new type of congestion, not captured by historic cost, is addressed by introducing a modified present cost. This cost is updated dynamically within each iteration to reflect instantaneous congestion.

In this project, the approach from the revisited PathFinder algorithm is used, as it produced the best results during testing.
== Steiner Trees
 A Steiner Tree is a network connecting a given set of points (called terminals) with the minimal total edge length. Unlike a standard spanning tree, a Steiner Tree may introduce additional points, called Steiner points, to reduce the overall cost of connecting the terminals.

In the context of signal routing on a graph:
- Terminals correspond to the source and sinks of a signal.
- Steiner points represent potential intermediate nodes that help reduce the total routing length or cost.

*Why Steiner Trees are interesting for routing:*
1. Cost minimization: They provide a near-optimal way to connect multiple sinks with minimal total wire length.
2. Congestion reduction: By efficiently distributing paths through shared Steiner points, they can reduce node and edge congestion.
3. Better routing flexibility: Steiner Trees allow non-terminal nodes to participate in routing, offering more routing options compared to simple shortest-path trees.
Using Steiner Trees in conjunction with algorithms like PathFinder can lead to more efficient and less congested routing solutions, especially when multiple sinks need to be connected simultaneously.
== Implementation
This Project consists of two implementations of the Local Router.
1. Simple Router: Computes a path from the source (signal) to each sink individually using Dijkstra’s algorithm. Each sink is connected independently, resulting in straightforward but potentially redundant paths.
2. Steiner Router: Approximates a Steiner tree to connect all sinks in a net efficiently. Instead of connecting sinks individually, it finds a central path and attaches sinks via shortest paths to this tree, reducing the overall wiring cost compared to the simple router.



= Results
To evaluate the routing quality, the algorithm records the following metrics:
- Longest path length: the length of the longest routed path for any signal.
- Wire reuse per signal: 
  This metric is defined as the sum of the usage counts of all nodes belonging to a signal’s routing net, divided by the total number of nodes in that net. It reflects the average number of sinks sharing each wire segment used by the signal.
- Total wire usage: the overall amount of wire used across all routed signals.

The Algorithm runs for a maximum of 10.000 iterations.

The results are summarized in the table below.\
*Green cells* indicate that the algorithm successfully completed the routing without conflicts.\
*Red cells* indicate that the algorithm failed to resolve all routing conflicts; in this case, the reported value corresponds to the number of remaining conflicts at termination.\
#figure(image("../routing-fpga/typst/results_Simple_0_1.svg"), caption: [Simple Solver with historic factor 0.1])
#figure(image("../routing-fpga/typst/results_Steiner_0_1.svg"), caption: [Steiner Solver with historic factor 0.1])
The PathFinder algorithm is able to solve almost all tested routing instances. However, it fails for three specific problem classes:
- 80 % of all possible signal configurations in which each signal has four sinks, and
- 100 % of all possible signal configurations in which each signal has three or four sinks.

For these inputs, the algorithm terminates with unresolved routing conflicts.

In contrast, the Simple Solver is able to solve these same problem instances. It produces shorter longest paths, but at the cost of increased total wire usage. This trade-off is also reflected in the wire reuse metric, which is higher for the Steiner-based approach.

This behavior is expected, as the Steiner solver explicitly constructs shared routing structures to maximize wire reuse, thereby reducing path lengths while increasing overall wire utilization.

#bibliography("works.yaml")
