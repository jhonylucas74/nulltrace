import { useState, useCallback, useRef, useEffect, useLayoutEffect } from "react";
import { Search, MapPin, Globe, Layers } from "lucide-react";
import {
  ComposableMap,
  Geographies,
  Geography,
  Line,
  Marker,
} from "react-simple-maps";
import { getDefaultTrace, type TracerouteHop } from "../lib/tracerouteData";
import Modal from "./Modal";
import styles from "./TraceRouteApp.module.css";

const GEO_URL =
  "https://cdn.jsdelivr.net/npm/world-atlas@2/countries-110m.json";

/** Coordinates for mock US result (Washington DC, USA). Stored as lat/lng; Marker uses [lng, lat]. */
const MOCK_USA_LAT = 38.9072;
const MOCK_USA_LNG = -77.0369;

export type PinnedIp = {
  ip: string;
  city: string;
  country: string;
  /** ISO 3166-1 alpha-2 country code for flag display. */
  countryCode?: string;
  region: string;
  isp: string;
  lat: number;
  lng: number;
};

/** Converts ISO 3166-1 alpha-2 country code to flag emoji. */
function countryCodeToFlag(code: string): string {
  return code
    .toUpperCase()
    .slice(0, 2)
    .replace(/./g, (c) => String.fromCodePoint(0x1f1e6 - 65 + c.charCodeAt(0)));
}

/** Mock IP lookup result: always returns a fictional US location. */
function getMockIpLookupResult(ip: string): PinnedIp {
  return {
    ip: ip.trim() || "192.0.2.1",
    country: "United States",
    countryCode: "US",
    region: "East Coast",
    city: "Washington",
    isp: "Nulltrace backbone",
    lat: MOCK_USA_LAT,
    lng: MOCK_USA_LNG,
  };
}

export default function TraceRouteApp() {
  const [trace] = useState<TracerouteHop[]>(getDefaultTrace);
  const [hoveredHop, setHoveredHop] = useState<TracerouteHop | null>(null);
  const [tooltipPos, setTooltipPos] = useState<{ x: number; y: number } | null>(null);
  const [tooltipStyle, setTooltipStyle] = useState<{ left: number; top: number; above: boolean } | null>(null);
  const mapWrapRef = useRef<HTMLDivElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [mapSize, setMapSize] = useState({ width: 800, height: 500 });

  const [ipLookupOpen, setIpLookupOpen] = useState(false);
  const [ipLookupInput, setIpLookupInput] = useState("");
  const [ipLookupStep, setIpLookupStep] = useState<"form" | "result">("form");
  const [ipLookupResult, setIpLookupResult] = useState<PinnedIp | null>(null);
  const [pinnedIps, setPinnedIps] = useState<PinnedIp[]>([]);
  const [hoveredPin, setHoveredPin] = useState<PinnedIp | null>(null);
  const [labelsVisible, setLabelsVisible] = useState(false);

  useEffect(() => {
    const el = mapWrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const { width, height } = entries[0].contentRect;
      setMapSize({ width: Math.max(400, width), height: Math.max(300, height) });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const handleIpLookupConfirm = useCallback(() => {
    const result = getMockIpLookupResult(ipLookupInput);
    setIpLookupResult(result);
    setIpLookupStep("result");
  }, [ipLookupInput]);

  const handleIpLookupClose = useCallback(() => {
    setIpLookupOpen(false);
    setIpLookupStep("form");
    setIpLookupInput("");
    setIpLookupResult(null);
  }, []);

  const handlePinOnMap = useCallback(() => {
    if (!ipLookupResult) return;
    setPinnedIps((prev) => [...prev, ipLookupResult]);
    handleIpLookupClose();
  }, [ipLookupResult, handleIpLookupClose]);

  const handleNodeMouseEnter = useCallback(
    (hop: TracerouteHop) => (e: React.MouseEvent) => {
      const mapEl = mapWrapRef.current;
      if (!mapEl) return;
      const rect = mapEl.getBoundingClientRect();
      setHoveredHop(hop);
      setTooltipPos({ x: e.clientX - rect.left, y: e.clientY - rect.top });
      setTooltipStyle(null);
    },
    []
  );

  const handleNodeMouseLeave = useCallback(() => {
    setHoveredHop(null);
    setTooltipPos(null);
    setTooltipStyle(null);
  }, []);

  const handlePinMouseEnter = useCallback(
    (pin: PinnedIp) => (e: React.MouseEvent) => {
      const mapEl = mapWrapRef.current;
      if (!mapEl) return;
      const rect = mapEl.getBoundingClientRect();
      setHoveredPin(pin);
      setTooltipPos({ x: e.clientX - rect.left, y: e.clientY - rect.top });
      setTooltipStyle(null);
    },
    []
  );

  const handlePinMouseLeave = useCallback(() => {
    setHoveredPin(null);
    setTooltipPos(null);
    setTooltipStyle(null);
  }, []);

  useLayoutEffect(() => {
    if (!tooltipPos || !tooltipRef.current || !mapWrapRef.current) return;
    const tip = tooltipRef.current;
    const map = mapWrapRef.current;
    const mapRect = map.getBoundingClientRect();
    const tipRect = tip.getBoundingClientRect();
    const mapW = mapRect.width;
    const mapH = mapRect.height;
    const tipW = tipRect.width;
    const tipH = tipRect.height;
    const gap = 10;
    // Position above the point; transform (-50%, -100%) puts tooltip bottom-center at (left, top)
    let left = Math.max(tipW / 2, Math.min(mapW - tipW / 2, tooltipPos.x));
    let top = Math.max(tipH, Math.min(mapH, tooltipPos.y - gap));
    setTooltipStyle({ left, top, above: true });
  }, [tooltipPos, hoveredHop, hoveredPin]);

  return (
    <div className={styles.app}>
      <div className={styles.toolbar}>
        <button
          type="button"
          className={labelsVisible ? `${styles.toolbarBtn} ${styles.toolbarBtnActive}` : styles.toolbarBtn}
          onClick={() => setLabelsVisible((v) => !v)}
          title={labelsVisible ? "Hide IP labels" : "Show IP labels"}
          aria-label={labelsVisible ? "Hide IP labels" : "Show IP labels"}
          aria-pressed={labelsVisible}
        >
          <Layers size={18} />
          <span className={styles.toolbarLabel}>Labels</span>
        </button>
        <button
          type="button"
          className={styles.toolbarBtn}
          onClick={() => setIpLookupOpen(true)}
          title="IP Lookup"
          aria-label="IP Lookup"
        >
          <Search size={18} />
          <span className={styles.toolbarLabel}>IP Lookup</span>
        </button>
      </div>

      <div className={styles.mapWrap} ref={mapWrapRef}>
        <ComposableMap
          width={mapSize.width}
          height={mapSize.height}
          projection="geoMercator"
          projectionConfig={{
            scale: 140,
            center: [0, 20],
          }}
        >
          <Geographies geography={GEO_URL}>
            {({ geographies }: { geographies: Array<{ rsmKey: string }> }) =>
              geographies.map((geo: { rsmKey: string }) => (
                <Geography
                  key={geo.rsmKey}
                  geography={geo}
                  style={{
                    default: { outline: "none" },
                    hover: { outline: "none" },
                    pressed: { outline: "none" },
                  }}
                />
              ))
            }
          </Geographies>
          {trace.length > 1 &&
            trace.slice(0, -1).map((hop, i) => {
              const next = trace[i + 1];
              return (
                <Line
                  key={`line-${i}`}
                  from={[hop.lng, hop.lat]}
                  to={[next.lng, next.lat]}
                  className={styles.routeLine}
                  stroke="var(--accent)"
                  strokeWidth={2}
                />
              );
            })}
          {trace.map((hop) => (
            <Marker
              key={hop.ip}
              coordinates={[hop.lng, hop.lat]}
              onMouseEnter={handleNodeMouseEnter(hop)}
              onMouseLeave={handleNodeMouseLeave}
            >
              <g className={styles.node}>
                <circle
                  r={hop.hopIndex === 1 ? 6 : 5}
                  className={styles.nodeCircle}
                />
              </g>
              {labelsVisible && (
                <g className={styles.mapLabelWrap} transform="translate(10, -6)">
                  <text className={styles.mapLabel} dy="0.35em">
                    {hop.countryCode ? `${countryCodeToFlag(hop.countryCode)} ` : ""}{hop.ip}
                  </text>
                </g>
              )}
            </Marker>
          ))}
          {pinnedIps.map((pin) => (
            <Marker
              key={`pin-${pin.ip}-${pin.lat}-${pin.lng}`}
              coordinates={[pin.lng, pin.lat]}
              onMouseEnter={handlePinMouseEnter(pin)}
              onMouseLeave={handlePinMouseLeave}
            >
              <g className={styles.pinNode} transform="translate(0, -10)">
                <MapPin size={20} strokeWidth={2} />
              </g>
              {labelsVisible && (
                <g className={styles.mapLabelWrap} transform="translate(10, -14)">
                  <text className={styles.mapLabel} dy="0.35em">
                    {pin.countryCode ? `${countryCodeToFlag(pin.countryCode)} ` : ""}{pin.ip}
                  </text>
                </g>
              )}
            </Marker>
          ))}
        </ComposableMap>
        {(hoveredHop || hoveredPin) && tooltipPos && (() => {
          const isHop = !!hoveredHop;
          const item = hoveredHop ?? hoveredPin!;
          const countryCode = isHop ? hoveredHop!.countryCode : hoveredPin!.countryCode;
          const flag = countryCode ? countryCodeToFlag(countryCode) : null;
          const style = tooltipStyle ?? {
            left: tooltipPos.x,
            top: tooltipPos.y - 10,
            above: true as boolean,
          };
          return (
            <div
              ref={tooltipRef}
              className={styles.tooltip}
              style={{
                left: style.left,
                top: style.top,
                transform: "translate(-50%, -100%)",
              }}
            >
              <div className={styles.tooltipHeader}>
                {flag ? (
                  <span className={styles.tooltipFlag} aria-hidden="true">{flag}</span>
                ) : (
                  <Globe size={18} className={styles.tooltipGlobe} aria-hidden="true" />
                )}
                <span className={styles.tooltipIp}>{item.ip}</span>
              </div>
              <div className={styles.tooltipMeta}>
                {isHop
                  ? `${hoveredHop!.city}, ${hoveredHop!.country}${hoveredHop!.latencyMs > 0 ? ` · ${hoveredHop!.latencyMs} ms` : ""}`
                  : `${hoveredPin!.city}, ${hoveredPin!.region}, ${hoveredPin!.country} (pinned)`}
              </div>
            </div>
          );
        })()}
      </div>

      <Modal
        open={ipLookupOpen}
        onClose={handleIpLookupClose}
        title="IP Lookup"
        secondaryButton={
          ipLookupStep === "form"
            ? { label: "Cancel", onClick: handleIpLookupClose }
            : undefined
        }
        primaryButton={
          ipLookupStep === "form"
            ? { label: "Look up", onClick: handleIpLookupConfirm }
            : { label: "Close", onClick: handleIpLookupClose }
        }
      >
        {ipLookupStep === "form" ? (
          <>
            <p className={styles.modalHint}>Enter the IP address you want to look up.</p>
            <label className={styles.modalLabel} htmlFor="traceroute-ip-input">
              IP address
            </label>
            <input
              id="traceroute-ip-input"
              type="text"
              className={styles.modalInput}
              placeholder="e.g. 192.0.2.1"
              value={ipLookupInput}
              onChange={(e) => setIpLookupInput(e.target.value)}
              aria-label="IP address"
            />
          </>
        ) : ipLookupResult ? (
          <div className={styles.lookupResult}>
            <div className={styles.lookupResultRow}>
              <span className={styles.lookupResultLabel}>IP</span>
              <span className={styles.lookupResultValue}>{ipLookupResult.ip}</span>
            </div>
            <div className={styles.lookupResultRow}>
              <span className={styles.lookupResultLabel}>Location</span>
              <span className={styles.lookupResultValue}>
                {ipLookupResult.city}, {ipLookupResult.region}, {ipLookupResult.country}
              </span>
            </div>
            <div className={styles.lookupResultRow}>
              <span className={styles.lookupResultLabel}>ISP</span>
              <span className={styles.lookupResultValue}>{ipLookupResult.isp}</span>
            </div>
            <button
              type="button"
              className={styles.pinOnMapBtn}
              onClick={handlePinOnMap}
              aria-label="Pin IP on map"
            >
              <MapPin size={18} strokeWidth={2.5} />
              <span className={styles.pinOnMapBtnLabel}>Pin on map</span>
            </button>
          </div>
        ) : null}
      </Modal>
    </div>
  );
}
