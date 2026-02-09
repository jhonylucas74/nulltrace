/**
 * Mock traceroute data for the TraceRoute app. Fictional hops only; no real network.
 */

export interface TracerouteHop {
  ip: string;
  city: string;
  country: string;
  /** ISO 3166-1 alpha-2 country code for flag display (e.g. "BR", "US"). */
  countryCode?: string;
  lat: number;
  lng: number;
  latencyMs: number;
  hopIndex: number;
}

/** Default path: player (origin) through a few hops to a destination. */
const DEFAULT_TRACE: TracerouteHop[] = [
  { ip: "10.0.1.1", city: "Local", country: "Home", countryCode: "BR", lat: -23.55, lng: -46.63, latencyMs: 0, hopIndex: 1 },
  { ip: "192.168.100.1", city: "Gateway", country: "ISP", countryCode: "BR", lat: -23.52, lng: -46.58, latencyMs: 12, hopIndex: 2 },
  { ip: "172.16.0.42", city: "Backbone A", country: "Transit", countryCode: "BR", lat: -15.78, lng: -47.92, latencyMs: 28, hopIndex: 3 },
  { ip: "203.0.113.10", city: "Exchange Point", country: "Peering", countryCode: "CO", lat: 4.71, lng: -74.07, latencyMs: 85, hopIndex: 4 },
  { ip: "198.51.100.5", city: "Core Router", country: "Backbone", countryCode: "ES", lat: 40.42, lng: -3.70, latencyMs: 142, hopIndex: 5 },
  { ip: "192.0.2.100", city: "Edge Node", country: "Destination", countryCode: "GB", lat: 51.51, lng: -0.13, latencyMs: 168, hopIndex: 6 },
];

/** Alternate path (e.g. when user searches a different "target"). */
const ALTERNATE_TRACE: TracerouteHop[] = [
  { ip: "10.0.1.1", city: "Local", country: "Home", countryCode: "BR", lat: -23.55, lng: -46.63, latencyMs: 0, hopIndex: 1 },
  { ip: "192.168.100.1", city: "Gateway", country: "ISP", countryCode: "BR", lat: -23.52, lng: -46.58, latencyMs: 11, hopIndex: 2 },
  { ip: "172.16.0.100", city: "Backbone B", country: "Transit", countryCode: "AR", lat: -34.60, lng: -58.38, latencyMs: 35, hopIndex: 3 },
  { ip: "203.0.113.20", city: "Exchange South", country: "Peering", countryCode: "CL", lat: -33.45, lng: -70.67, latencyMs: 98, hopIndex: 4 },
  { ip: "198.51.100.22", city: "Pacific Link", country: "Backbone", countryCode: "JP", lat: 35.68, lng: 139.69, latencyMs: 245, hopIndex: 5 },
  { ip: "192.0.2.200", city: "Target Host", country: "Destination", countryCode: "US", lat: 37.77, lng: -122.42, latencyMs: 278, hopIndex: 6 },
];

/**
 * Returns a mock traceroute path for the given IP. If IP matches a known prefix or
 * is in one of the preset paths, returns that path; otherwise returns default path.
 */
export function getTraceForIp(ip: string): TracerouteHop[] {
  const trimmed = ip.trim();
  if (!trimmed) return DEFAULT_TRACE;
  const lower = trimmed.toLowerCase();
  if (lower.includes("200") || lower.includes("192.0.2.200")) return ALTERNATE_TRACE;
  if (lower.includes("100") || lower.includes("192.0.2.100")) return DEFAULT_TRACE;
  return DEFAULT_TRACE;
}

export function getDefaultTrace(): TracerouteHop[] {
  return DEFAULT_TRACE;
}
