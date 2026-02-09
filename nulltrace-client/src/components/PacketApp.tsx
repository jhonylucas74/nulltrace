import { useState, useEffect, useRef, useMemo } from "react";
import { Play, Pause, XCircle, Search, Filter, ArrowLeft } from "lucide-react";
import { generatePacket, PACKET_LISTEN_IP, type Packet, type PacketProtocol, type PacketStatus } from "../lib/packetData";
import styles from "./PacketApp.module.css";

const MAX_PACKETS = 1000;

const PROTOCOLS: PacketProtocol[] = ["TCP", "UDP", "HTTP", "HTTPS", "DNS", "ICMP", "SSH", "FTP"];
const STATUSES: PacketStatus[] = ["normal", "suspicious", "critical"];

export default function PacketApp() {
  const [packets, setPackets] = useState<Packet[]>([]);
  const [isRunning, setIsRunning] = useState(true);
  const [filterText, setFilterText] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [detailView, setDetailView] = useState(false);
  const [filterProtocols, setFilterProtocols] = useState<Set<PacketProtocol>>(new Set());
  const [filterStatuses, setFilterStatuses] = useState<Set<PacketStatus>>(new Set());
  const [filterPanelOpen, setFilterPanelOpen] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const filterPanelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isRunning) return;
    const interval = setInterval(() => {
      const count = Math.floor(Math.random() * 4) + 1;
      const newPackets: Packet[] = [];
      for (let i = 0; i < count; i++) {
        newPackets.push(generatePacket());
      }
      setPackets((prev) => {
        const updated = [...prev, ...newPackets];
        if (updated.length > MAX_PACKETS) {
          return updated.slice(updated.length - MAX_PACKETS);
        }
        return updated;
      });
    }, 800);
    return () => clearInterval(interval);
  }, [isRunning]);

  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [packets, autoScroll]);

  useEffect(() => {
    if (!filterPanelOpen) return;
    function handleClickOutside(e: MouseEvent) {
      if (filterPanelRef.current && !filterPanelRef.current.contains(e.target as Node)) {
        setFilterPanelOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [filterPanelOpen]);

  const handleScroll = () => {
    if (!scrollRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    setAutoScroll(scrollHeight - scrollTop - clientHeight < 50);
  };

  const filteredPackets = useMemo(() => {
    let list = packets;
    if (filterText.trim()) {
      const lower = filterText.trim().toLowerCase();
      list = list.filter(
        (p) =>
          p.protocol.toLowerCase().includes(lower) ||
          p.source.includes(lower) ||
          p.destination.includes(lower) ||
          p.info.toLowerCase().includes(lower) ||
          p.processName.toLowerCase().includes(lower)
      );
    }
    if (filterProtocols.size > 0) {
      list = list.filter((p) => filterProtocols.has(p.protocol));
    }
    if (filterStatuses.size > 0) {
      list = list.filter((p) => filterStatuses.has(p.status));
    }
    return list;
  }, [packets, filterText, filterProtocols, filterStatuses]);

  const toggleProtocol = (proto: PacketProtocol) => {
    setFilterProtocols((prev) => {
      const next = new Set(prev);
      if (next.has(proto)) next.delete(proto);
      else next.add(proto);
      return next;
    });
  };

  const toggleStatus = (status: PacketStatus) => {
    setFilterStatuses((prev) => {
      const next = new Set(prev);
      if (next.has(status)) next.delete(status);
      else next.add(status);
      return next;
    });
  };

  const selectedPacket = selectedId != null ? packets.find((p) => p.id === selectedId) : null;

  const openDetail = (id: number) => {
    setSelectedId(id);
    setDetailView(true);
  };

  const closeDetail = () => {
    setDetailView(false);
    setSelectedId(null);
  };

  return (
    <div className={styles.app}>
      {!detailView && (
        <>
      <div className={styles.header}>
        <span className={styles.listenInfo}>
          Listening on <span className={styles.listenValue}>{PACKET_LISTEN_IP}</span>
        </span>
        <button
          className={`${styles.controlButton} ${isRunning ? styles.active : ""}`}
          onClick={() => setIsRunning(!isRunning)}
          title={isRunning ? "Stop Capture" : "Start Capture"}
        >
          {isRunning ? <Pause size={14} /> : <Play size={14} />}
          {isRunning ? "Stop" : "Start"}
        </button>
        <button className={styles.controlButton} onClick={() => setPackets([])} title="Clear Buffer">
          <XCircle size={14} />
          Clear
        </button>
        <div className={styles.searchBar}>
          <Search size={14} className={styles.searchIcon} aria-hidden />
          <input
            type="text"
            className={styles.searchInput}
            placeholder="Filter (e.g. tcp, 192.168...)"
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
            aria-label="Filter packets"
          />
        </div>
        <div className={styles.filterWrap} ref={filterPanelRef}>
          <button
            type="button"
            className={styles.controlButton}
            onClick={() => setFilterPanelOpen((o) => !o)}
            title="Filter by protocol and status"
            aria-expanded={filterPanelOpen}
          >
            <Filter size={14} />
            Filters
          </button>
          {filterPanelOpen && (
            <div className={styles.filterPanel}>
              <p className={styles.filterPanelTitle}>Protocol</p>
              <div className={styles.filterGroup}>
                {PROTOCOLS.map((proto) => (
                  <label key={proto} className={styles.filterLabel}>
                    <input
                      type="checkbox"
                      checked={filterProtocols.has(proto)}
                      onChange={() => toggleProtocol(proto)}
                    />
                    {proto}
                  </label>
                ))}
              </div>
              <p className={styles.filterPanelTitle}>Status</p>
              <div className={styles.filterGroup}>
                {STATUSES.map((status) => (
                  <label key={status} className={styles.filterLabel}>
                    <input
                      type="checkbox"
                      checked={filterStatuses.has(status)}
                      onChange={() => toggleStatus(status)}
                    />
                    {status.charAt(0).toUpperCase() + status.slice(1)}
                  </label>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      <div className={styles.packetListContainer} ref={scrollRef} onScroll={handleScroll}>
        <table className={styles.packetTable}>
          <thead>
            <tr>
              <th className={styles.colId}>No.</th>
              <th className={styles.colTime}>Time</th>
              <th className={styles.colSource}>Source</th>
              <th className={styles.colDest}>Destination</th>
              <th className={styles.colProto}>Protocol</th>
              <th className={styles.colLen}>Length</th>
              <th className={styles.colInfo}>Info</th>
            </tr>
          </thead>
          <tbody>
            {filteredPackets.length === 0 ? (
              <tr>
                <td colSpan={7} className={styles.emptyCell}>
                  {packets.length === 0
                    ? "No packets captured"
                    : "No packets match filter"}
                </td>
              </tr>
            ) : (
              filteredPackets.map((p) => {
                const statusClass =
                  p.status === "critical"
                    ? styles.statusCritical
                    : p.status === "suspicious"
                      ? styles.statusSuspicious
                      : styles.statusNormal;
                const timeString =
                  new Date(p.timestamp).toLocaleTimeString([], {
                    hour12: false,
                    hour: "2-digit",
                    minute: "2-digit",
                    second: "2-digit",
                  }) +
                  "." +
                  (p.timestamp % 1000).toString().padStart(3, "0");
                return (
                  <tr
                    key={p.id}
                    className={`${styles.packetRow} ${selectedId === p.id ? styles.selected : ""}`}
                    onClick={() => openDetail(p.id)}
                  >
                    <td className={`${styles.colId} ${statusClass}`}>{p.id}</td>
                    <td className={styles.colTime}>{timeString}</td>
                    <td className={styles.colSource} title={p.source}>
                      {p.source}
                    </td>
                    <td className={styles.colDest} title={p.destination}>
                      {p.destination}
                    </td>
                    <td className={styles.colProto}>{p.protocol}</td>
                    <td className={styles.colLen}>{p.size}</td>
                    <td className={styles.colInfo} title={`PID: ${p.pid} (${p.processName}) - ${p.info}`}>
                      <span className={styles.colInfoProcess}>
                        [{p.processName}:{p.pid}]
                      </span>
                      {p.info}
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      <div className={styles.statusBar}>
        <span>Packets: {packets.length}</span>
        <span>Displayed: {filteredPackets.length}</span>
        <span>{isRunning ? "Capturing…" : "Paused"}</span>
      </div>
        </>
      )}

      {detailView && selectedPacket && (
        <div className={styles.detailPage}>
          <div className={styles.detailHeader}>
            <button type="button" className={styles.backBtn} onClick={closeDetail} aria-label="Back to list">
              <ArrowLeft size={18} />
              Back
            </button>
            <h2 className={styles.detailTitle}>Packet #{selectedPacket.id} — {selectedPacket.protocol}</h2>
          </div>
          <div className={styles.detailScroll}>
            <section className={styles.detailSection}>
              <h3 className={styles.detailSectionTitle}>Summary</h3>
              <div className={styles.detailGrid}>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>No.</span>
                  <span className={styles.detailValue}>{selectedPacket.id}</span>
                </div>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>Time</span>
                  <span className={styles.detailValue}>
                    {new Date(selectedPacket.timestamp).toLocaleString()}
                  </span>
                </div>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>Source</span>
                  <span className={`${styles.detailValue} ${styles.mono}`}>{selectedPacket.source}</span>
                </div>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>Destination</span>
                  <span className={`${styles.detailValue} ${styles.mono}`}>{selectedPacket.destination}</span>
                </div>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>Protocol</span>
                  <span className={styles.detailValue}>{selectedPacket.protocol}</span>
                </div>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>Length</span>
                  <span className={styles.detailValue}>{selectedPacket.size} bytes</span>
                </div>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>Status</span>
                  <span className={styles.detailValue}>{selectedPacket.status}</span>
                </div>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>Process</span>
                  <span className={styles.detailValue}>
                    {selectedPacket.processName} (PID {selectedPacket.pid})
                  </span>
                </div>
                <div className={styles.detailRow}>
                  <span className={styles.detailLabel}>Info</span>
                  <span className={`${styles.detailValue} ${styles.mono}`}>{selectedPacket.info}</span>
                </div>
              </div>
            </section>

            {selectedPacket.headers && selectedPacket.headers.length > 0 && (
              <section className={styles.detailSection}>
                <h3 className={styles.detailSectionTitle}>Headers</h3>
                <pre className={styles.detailPre}>
                  {selectedPacket.headers.map((h) => h + "\n").join("")}
                </pre>
              </section>
            )}

            {selectedPacket.body && (
              <section className={styles.detailSection}>
                <h3 className={styles.detailSectionTitle}>Body</h3>
                <pre className={styles.detailPre}>{selectedPacket.body}</pre>
              </section>
            )}

            {selectedPacket.rawPayload && (
              <section className={styles.detailSection}>
                <h3 className={styles.detailSectionTitle}>Raw payload</h3>
                <pre className={styles.detailPre}>{selectedPacket.rawPayload}</pre>
              </section>
            )}

            {!selectedPacket.headers?.length && !selectedPacket.body && !selectedPacket.rawPayload && (
              <section className={styles.detailSection}>
                <p className={styles.detailMuted}>No additional content for this packet.</p>
              </section>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
