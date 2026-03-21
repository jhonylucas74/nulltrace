import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import ForceGraph2D from "react-force-graph-2d";
import { Loader2 } from "lucide-react";
import styles from "./NetworkTopology.module.css";

interface NetworkNode {
  id: string;
  label: string;
  ip: string;
  subnet: string;
  node_type: string;
}

interface NetworkEdge {
  source_id: string;
  target_id: string;
  same_subnet: boolean;
}

interface GraphNode {
  id: string;
  label: string;
  ip: string;
  subnet: string;
  node_type: string;
}

interface GraphLink {
  source: string;
  target: string;
  same_subnet?: boolean;
}

const POLL_INTERVAL_MS = 15000;

// Explicit hex colors (canvas does not resolve CSS variables)
const NODE_COLORS: Record<string, string> = {
  vm: "#89b4fa",
  router: "#cba6f7",
  subnet: "#6c7086",
};

const NODE_RADIUS: Record<string, number> = {
  vm: 18,
  router: 22,
  subnet: 14,
};

// Simulation uses larger radius for spacing (prevents overlap)
const SIMULATION_RADIUS_MULTIPLIER = 3;

const LABEL_COLOR = "#cdd6f4";
const LABEL_FONT_SIZE = 18;
const LINK_COLOR = "rgba(108, 112, 134, 0.5)";
const BG_COLOR = "#181825";

// Force layout: spread nodes apart
const LINK_DISTANCE = 180;
const CHARGE_STRENGTH = -500;

export default function NetworkTopology({ token }: { token: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const graphRef = useRef<{ d3Force?: (n: string) => { distance: (v: number) => void; strength: (v: number) => void }; zoomToFit?: (d?: number) => void }>(null);
  const hasZoomedToFit = useRef(false);
  const [dimensions, setDimensions] = useState({ width: 800, height: 500 });
  const [nodes, setNodes] = useState<NetworkNode[]>([]);
  const [edges, setEdges] = useState<NetworkEdge[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      for (const e of entries) {
        const { width, height } = e.contentRect;
        if (width > 0 && height > 0) setDimensions({ width, height });
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    hasZoomedToFit.current = false;
  }, [nodes, edges]);

  useEffect(() => {
    const g = graphRef.current;
    if (!g?.d3Force) return;
    g.d3Force("link").distance(LINK_DISTANCE);
    g.d3Force("charge").strength(CHARGE_STRENGTH);
  }, [nodes, edges]);

  useEffect(() => {
    let cancelled = false;
    async function fetchTopology() {
      try {
        const res = await invoke<{ nodes: NetworkNode[]; edges: NetworkEdge[] }>(
          "get_network_topology",
          { token }
        );
        if (cancelled) return;
        setNodes(res.nodes);
        setEdges(res.edges);
        setError(null);
      } catch (err) {
        if (!cancelled)
          setError(err instanceof Error ? err.message : "Failed to fetch");
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
  }, [token]);

  const graphData = useMemo(() => {
    const graphNodes: GraphNode[] = nodes.map((n) => ({
      id: n.id,
      label: n.label || n.ip || n.id,
      ip: n.ip,
      subnet: n.subnet,
      node_type: n.node_type,
    }));
    const graphLinks: GraphLink[] = edges.map((e) => ({
      source: e.source_id,
      target: e.target_id,
      same_subnet: e.same_subnet,
    }));
    return { nodes: graphNodes, links: graphLinks };
  }, [nodes, edges]);

  const nodeColor = useCallback(
    (node: GraphNode) => NODE_COLORS[node.node_type] ?? "#6c7086",
    []
  );

  const nodeRadius = useCallback(
    (node: GraphNode) => NODE_RADIUS[node.node_type] ?? 12,
    []
  );

  const nodeValForSimulation = useCallback(
    (node: GraphNode) => (NODE_RADIUS[node.node_type] ?? 12) * SIMULATION_RADIUS_MULTIPLIER,
    []
  );

  const nodeLabel = useCallback(
    (node: GraphNode) => {
      const parts = [node.label];
      if (node.ip) parts.push(`IP: ${node.ip}`);
      if (node.subnet) parts.push(`Subnet: ${node.subnet}`);
      return parts.join("\n");
    },
    []
  );

  const nodeCanvasObject = useCallback(
    (node: GraphNode & { x?: number; y?: number }, ctx: CanvasRenderingContext2D, globalScale: number) => {
      const radius = nodeRadius(node);
      const color = nodeColor(node);
      const label = node.label || node.ip || node.id;

      // Draw circle
      ctx.beginPath();
      ctx.arc(node.x ?? 0, node.y ?? 0, radius, 0, 2 * Math.PI);
      ctx.fillStyle = color;
      ctx.fill();
      ctx.strokeStyle = "rgba(255,255,255,0.2)";
      ctx.lineWidth = 1 / globalScale;
      ctx.stroke();

      // Label scales with zoom (like nodes): smaller when zoomed out, larger when zoomed in
      const fontSize = Math.max(10, LABEL_FONT_SIZE * globalScale);
      ctx.font = `500 ${fontSize}px Inter, system-ui, -apple-system, sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillStyle = LABEL_COLOR;
      const labelY = (node.y ?? 0) + radius + fontSize / 2 + 2;
      ctx.fillText(label, Math.round(node.x ?? 0), Math.round(labelY));
    },
    [nodeColor, nodeRadius]
  );

  const linkColor = useCallback(() => LINK_COLOR, []);

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

  if (graphData.nodes.length === 0) {
    return (
      <div className={styles.root}>
        <h1 className={styles.title}>Network Topology</h1>
        <p className={styles.subtitle}>No network nodes.</p>
        <div className={styles.graphContainer} />
      </div>
    );
  }

  return (
    <div className={styles.root}>
      <h1 className={styles.title}>Network Topology</h1>
      <p className={styles.subtitle}>
        VMs, routers, and subnets. Force-directed graph.
      </p>
      <div ref={containerRef} className={styles.graphContainer}>
        <ForceGraph2D
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          ref={graphRef as any}
          graphData={graphData}
          width={dimensions.width}
          height={dimensions.height}
          backgroundColor={BG_COLOR}
          nodeCanvasObject={nodeCanvasObject}
          nodeLabel={nodeLabel}
          linkColor={linkColor}
          nodeVal={nodeValForSimulation}
          onEngineStop={() => {
            if (!hasZoomedToFit.current && graphRef.current?.zoomToFit) {
              hasZoomedToFit.current = true;
              graphRef.current.zoomToFit(400);
            }
          }}
          enableNodeDrag
          enableZoomInteraction
          enablePanInteraction
        />
      </div>
    </div>
  );
}
