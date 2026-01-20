import React, { useEffect, useRef, useState } from "react";
import { Network, DataSet } from "vis-network/standalone";
import { getGraphResult, type JsonGraph } from "../api";
import { useParams } from "react-router-dom";

export const ResultPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const containerRef = useRef<HTMLDivElement>(null);
  const [graphData, setGraphData] = useState<JsonGraph | null>(null);
  const [network, setNetwork] = useState<Network | null>(null);

  useEffect(() => {
    if (!id) return;

    getGraphResult(Number(id))
      .then(setGraphData)
      .catch(console.error);
  }, [id]);

  useEffect(() => {
    if (!graphData || !containerRef.current) return;

    // Create vis nodes
    const nodes = new DataSet(
      graphData.nodes.map(n => {
        let baseColor = "#6699ff"; // Default
        if (n.typ === "LutInput") baseColor = "#ff6666";
        if (n.typ === "LutOutput") baseColor = "#66ff66";

        const color = n.signals.length > 1 ? "#ff9900" : baseColor; // conflicts in orange

        return {
          id: n.id,
          label: n.label,
          color: { background: color },
          signals: n.signals,
        };
      })
    );

    // Create vis edges
    const edges = new DataSet(
      graphData.edges.map(e => {
        const isConflict = e.signals.length > 1;
        return {
          id: `${e.from}-${e.to}`,
          from: e.from,
          to: e.to,
          length: 50 + e.weight * 20,
          color: { color: isConflict ? "#ff0000" : "#848484" },
          width: isConflict ? 4 : 1,
          signals: e.signals,
        };
      })
    );

    if (!network) {
      const net = new Network(containerRef.current, { nodes, edges }, {
        physics: {
          enabled: true,
          solver: "barnesHut",
          barnesHut: { gravitationalConstant: -3000, centralGravity: 0.01, springConstant: 0.04, springLength: 80 }
        },
        edges: { arrows: "to" },
        layout: { improvedLayout: false },
      });
      setNetwork(net);
    } else {
      network.setData({ nodes, edges });
    }
  }, [graphData, network]);

  return (
    <div className="p-6 bg-gray-900 min-h-screen text-gray-100">
      <h1 className="text-2xl font-bold mb-4 text-center">Graph Viewer</h1>
      <div ref={containerRef} style={{ width: "100%", height: "90vh", border: "1px solid lightgray" }} />
    </div>
  );
};
