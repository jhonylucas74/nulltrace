import { MOCK_PROCESSES } from "./systemMonitorData";

export type PacketProtocol = "TCP" | "UDP" | "HTTP" | "HTTPS" | "DNS" | "ICMP" | "SSH" | "FTP";
export type PacketStatus = "normal" | "suspicious" | "critical";

export interface Packet {
  id: number;
  timestamp: number;
  protocol: PacketProtocol;
  source: string;
  destination: string;
  size: number;
  status: PacketStatus;
  pid: number;
  processName: string;
  info: string;
  /** HTTP/application headers (one per line, "Name: value"). */
  headers?: string[];
  /** Request/response body (e.g. HTTP body, JSON). */
  body?: string;
  /** Raw payload as hex dump (optional). */
  rawPayload?: string;
}

const EXTERNAL_IPS = [
  "104.21.55.2", "172.67.198.1", "8.8.8.8", "8.8.4.4", "192.168.1.1",
  "142.250.74.46", "31.13.79.35", "52.222.158.53", "13.224.103.66"
];

/** Local/listening IP shown in the Packet app UI. */
export const PACKET_LISTEN_IP = "192.168.1.105";
const LOCAL_IP = PACKET_LISTEN_IP;

// Correlate protocols with realistic ports and info
const PROTOCOL_INFO: Record<PacketProtocol, { ports: number[], templates: string[] }> = {
  TCP: { ports: [80, 443, 8080, 3000], templates: ["ACK", "SYN", "FIN", "PSH, ACK", "RST"] },
  UDP: { ports: [53, 67, 68, 123], templates: ["Len=48", "Len=126", "Len=512"] },
  HTTP: { ports: [80, 8080], templates: ["GET /index.html", "POST /api/login", "GET /favicon.ico", "HTTP/1.1 200 OK"] },
  HTTPS: { ports: [443], templates: ["Client Hello", "Server Hello", "Application Data", "Encrypted Alert"] },
  DNS: { ports: [53], templates: ["Standard query A google.com", "Standard query AAAA facebook.com", "Standard response A 172.217.16.46"] },
  ICMP: { ports: [], templates: ["Echo (ping) request", "Echo (ping) reply", "Destination unreachable"] },
  SSH: { ports: [22], templates: ["Encrypted packet (len=48)", "Encrypted packet (len=124)"] },
  FTP: { ports: [21], templates: ["Request: USER admin", "Response: 331 Password required"] }
};

let nextId = 1;

export function generatePacket(): Packet {
  // Pick a random process to attribute traffic to
  const process = MOCK_PROCESSES[Math.floor(Math.random() * MOCK_PROCESSES.length)];
  
  // Decide if traffic is inbound or outbound
  const isOutbound = Math.random() > 0.4;
  const source = isOutbound ? LOCAL_IP : EXTERNAL_IPS[Math.floor(Math.random() * EXTERNAL_IPS.length)];
  const destination = isOutbound ? EXTERNAL_IPS[Math.floor(Math.random() * EXTERNAL_IPS.length)] : LOCAL_IP;

  // Pick a protocol likely for this process? For now random but weighted could be better.
  const protocols: PacketProtocol[] = ["TCP", "TCP", "UDP", "HTTP", "HTTPS", "HTTPS", "DNS"];
  if (Math.random() < 0.05) protocols.push("ICMP");
  if (Math.random() < 0.05) protocols.push("SSH");
  if (Math.random() < 0.02) protocols.push("FTP");
  
  const protocol = protocols[Math.floor(Math.random() * protocols.length)];
  const infoData = PROTOCOL_INFO[protocol];
  const info = infoData.templates[Math.floor(Math.random() * infoData.templates.length)];

  // Determine status
  let status: PacketStatus = "normal";
  if (protocol === "FTP" || (protocol === "SSH" && Math.random() < 0.2)) status = "suspicious";
  if (info.includes("RST") || info.includes("unreachable")) status = "suspicious";
  if (Math.random() < 0.01) status = "critical";

  const size = Math.floor(Math.random() * 1400) + 60;
  const base: Packet = {
    id: nextId++,
    timestamp: Date.now(),
    protocol,
    source,
    destination,
    size,
    status,
    pid: process.pid,
    processName: process.name,
    info,
  };

  // Add headers + body for HTTP; payload for others
  if (protocol === "HTTP") {
    const isRequest = info.startsWith("GET") || info.startsWith("POST");
    base.headers = isRequest
      ? [
          "Host: api.example.com",
          "User-Agent: Mozilla/5.0 (compatible; Nulltrace/1.0)",
          "Accept: application/json",
          "Accept-Language: en-US,en;q=0.9",
          "Connection: keep-alive",
          "Content-Type: application/json",
        ]
      : [
          "Content-Type: application/json",
          "Cache-Control: no-cache",
          "Connection: keep-alive",
          "Date: " + new Date().toUTCString(),
        ];
    if (info.includes("POST") || info.includes("200")) {
      base.body = isRequest
        ? '{"username":"user","password":"••••••••","remember":true}'
        : '{"success":true,"data":{"id":42,"name":"Item"}}';
    }
  }

  if (protocol === "HTTPS") {
    base.headers = ["Content-Type: application/octet-stream"];
    base.rawPayload = "16 03 01 00 a1 01 00 00 9d 03 03 5b 3f ... (encrypted)";
  }

  if (protocol === "TCP" || protocol === "UDP") {
    base.rawPayload = Array.from({ length: Math.min(16, Math.floor(size / 4)) }, () =>
      Math.floor(Math.random() * 256).toString(16).padStart(2, "0")
    ).join(" ") + (size > 64 ? " ..." : "");
  }

  return base;
}
