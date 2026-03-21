import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ReactFlow,
  Background,
  Controls,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Loader2 } from "lucide-react";
import styles from "./NetworkTopology.module.css";

interface NetworkNode {
  id: string;
  label: string;
  ip: string;
  subnet: string;
  node_type: string;
}

const NODE_WIDTH = 140;
const NODE_HEIGHT = 60;
const COLS = 8;
const GAP_X = 24;
const GAP_Y = 20;

const POLL_INTERVAL_MS = 15000;

export default function NetworkTopology({ token }: { token: string }) {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function fetchTopology() {
      try {
        const res = await invoke<{ nodes: NetworkNode[]; edges: unknown[] }>(
          "get_network_topology",
          { token }
        );
        if (cancelled) return;
        const vmNodes = res.nodes.filter((n) => n.node_type === "vm");
        const flowNodes: Node[] = vmNodes.map((n, i) => {
          const col = i % COLS;
          const row = Math.floor(i / COLS);
          return {
            id: n.id,
            type: "default",
            position: {
              x: col * (NODE_WIDTH + GAP_X),
              y: row * (NODE_HEIGHT + GAP_Y),
            },
            data: {
              label: n.label || n.ip || n.id,
            },
            style: {
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              borderRadius: "var(--border-radius-sm)",
              padding: "8px 12px",
              fontSize: 12,
            },
          };
        });
        setNodes(flowNodes);
        setEdges([]);
        setError(null);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : "Failed to fetch");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    fetchTopology();
    const id = setInterval(fetchTopology, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [token, setNodes, setEdges]);

  if (loading) {
    return (
      <div className={styles.loading}>
        <Loader2 size={28} className={styles.spinner} aria-hidden />
        <p>Loading network topology…</p>
      </div>
    );
  }

  if (error) {
    return <p className={styles.error}>{error}</p>;
  }

  return (
    <div className={styles.root}>
      <h1 className={styles.title}>Network Topology</h1>
      <p className={styles.subtitle}>VMs only. No connections.</p>
      <div className={styles.flowContainer}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          fitView
        >
          <Background />
          <Controls />
        </ReactFlow>
      </div>
    </div>
  );
}
