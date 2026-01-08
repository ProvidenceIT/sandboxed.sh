"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { cn } from "@/lib/utils";
import { getValidJwt } from "@/lib/auth";
import { getRuntimeApiBase } from "@/lib/settings";
import { Cpu, MemoryStick, Wifi, Activity } from "lucide-react";

interface SystemMetrics {
  cpu_percent: number;
  cpu_cores: number[];
  memory_used: number;
  memory_total: number;
  memory_percent: number;
  network_rx_bytes_per_sec: number;
  network_tx_bytes_per_sec: number;
  timestamp_ms: number;
}

interface SystemMonitorProps {
  className?: string;
  intervalMs?: number;
}

type ConnectionState = "connecting" | "connected" | "disconnected" | "error";

// Format bytes to human-readable string
function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(decimals)) + " " + sizes[i];
}

// Format bytes per second to human-readable string
function formatBytesPerSec(bytes: number): string {
  return formatBytes(bytes) + "/s";
}

// Mini sparkline component for history
function Sparkline({
  data,
  max = 100,
  color = "cyan",
  height = 24,
}: {
  data: number[];
  max?: number;
  color?: string;
  height?: number;
}) {
  const width = data.length * 2;
  const points = data
    .map((v, i) => {
      const x = i * 2;
      const y = height - (v / max) * height;
      return `${x},${y}`;
    })
    .join(" ");

  const colorMap: Record<string, string> = {
    cyan: "rgb(34, 211, 238)",
    green: "rgb(74, 222, 128)",
    orange: "rgb(251, 146, 60)",
    purple: "rgb(192, 132, 252)",
  };

  return (
    <svg
      width={width}
      height={height}
      className="opacity-60"
      viewBox={`0 0 ${width} ${height}`}
    >
      <polyline
        points={points}
        fill="none"
        stroke={colorMap[color] || colorMap.cyan}
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// Progress bar component
function MetricBar({
  value,
  max = 100,
  color = "cyan",
  showPercent = true,
}: {
  value: number;
  max?: number;
  color?: string;
  showPercent?: boolean;
}) {
  const percent = Math.min((value / max) * 100, 100);

  const colorMap: Record<string, string> = {
    cyan: "bg-cyan-400",
    green: "bg-green-400",
    orange: "bg-orange-400",
    purple: "bg-purple-400",
  };

  const bgColorMap: Record<string, string> = {
    cyan: "bg-cyan-400/20",
    green: "bg-green-400/20",
    orange: "bg-orange-400/20",
    purple: "bg-purple-400/20",
  };

  return (
    <div className="flex items-center gap-2 w-full">
      <div className={cn("flex-1 h-2 rounded-full", bgColorMap[color] || bgColorMap.cyan)}>
        <div
          className={cn(
            "h-full rounded-full transition-all duration-300",
            colorMap[color] || colorMap.cyan
          )}
          style={{ width: `${percent}%` }}
        />
      </div>
      {showPercent && (
        <span className="text-xs text-white/60 w-10 text-right font-mono">
          {percent.toFixed(0)}%
        </span>
      )}
    </div>
  );
}

// Individual metric card
function MetricCard({
  icon: Icon,
  label,
  value,
  subValue,
  bar,
  sparkline,
  color = "cyan",
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  subValue?: string;
  bar?: { value: number; max?: number };
  sparkline?: { data: number[]; max?: number };
  color?: string;
}) {
  const textColorMap: Record<string, string> = {
    cyan: "text-cyan-400",
    green: "text-green-400",
    orange: "text-orange-400",
    purple: "text-purple-400",
  };

  return (
    <div className="flex flex-col gap-2 p-3 rounded-lg bg-white/[0.02] border border-white/[0.04]">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Icon className={cn("h-4 w-4", textColorMap[color] || textColorMap.cyan)} />
          <span className="text-xs text-white/50 uppercase tracking-wide">{label}</span>
        </div>
        {sparkline && sparkline.data.length > 1 && (
          <Sparkline data={sparkline.data} max={sparkline.max} color={color} height={20} />
        )}
      </div>
      <div className="flex items-baseline gap-2">
        <span className={cn("text-lg font-semibold font-mono", textColorMap[color] || textColorMap.cyan)}>
          {value}
        </span>
        {subValue && <span className="text-xs text-white/40">{subValue}</span>}
      </div>
      {bar && <MetricBar value={bar.value} max={bar.max} color={color} />}
    </div>
  );
}

export function SystemMonitor({ className, intervalMs = 1000 }: SystemMonitorProps) {
  const [connectionState, setConnectionState] = useState<ConnectionState>("connecting");
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);
  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [memoryHistory, setMemoryHistory] = useState<number[]>([]);
  const [networkRxHistory, setNetworkRxHistory] = useState<number[]>([]);
  const [networkTxHistory, setNetworkTxHistory] = useState<number[]>([]);

  const wsRef = useRef<WebSocket | null>(null);
  const connectionIdRef = useRef(0);
  const maxHistory = 30; // Keep last 30 data points

  // Build WebSocket URL
  const buildWsUrl = useCallback(() => {
    const baseUrl = getRuntimeApiBase();
    const wsUrl = baseUrl
      .replace("https://", "wss://")
      .replace("http://", "ws://");

    const params = new URLSearchParams({
      interval_ms: intervalMs.toString(),
    });

    return `${wsUrl}/api/monitoring/ws?${params}`;
  }, [intervalMs]);

  // Connect to WebSocket
  const connect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
    }

    connectionIdRef.current += 1;
    const thisConnectionId = connectionIdRef.current;

    setConnectionState("connecting");

    const url = buildWsUrl();
    const jwt = getValidJwt();
    const token = jwt?.token ?? null;

    const protocols = token ? ["openagent", `jwt.${token}`] : ["openagent"];
    const ws = new WebSocket(url, protocols);

    ws.onopen = () => {
      if (connectionIdRef.current !== thisConnectionId) return;
      setConnectionState("connected");
    };

    ws.onmessage = (event) => {
      if (connectionIdRef.current !== thisConnectionId) return;
      if (typeof event.data === "string") {
        try {
          const data: SystemMetrics = JSON.parse(event.data);
          setMetrics(data);

          // Update histories
          setCpuHistory((prev) => {
            const next = [...prev, data.cpu_percent];
            return next.slice(-maxHistory);
          });
          setMemoryHistory((prev) => {
            const next = [...prev, data.memory_percent];
            return next.slice(-maxHistory);
          });
          setNetworkRxHistory((prev) => {
            const next = [...prev, data.network_rx_bytes_per_sec];
            return next.slice(-maxHistory);
          });
          setNetworkTxHistory((prev) => {
            const next = [...prev, data.network_tx_bytes_per_sec];
            return next.slice(-maxHistory);
          });
        } catch {
          // Ignore parse errors
        }
      }
    };

    ws.onerror = () => {
      if (connectionIdRef.current !== thisConnectionId) return;
      setConnectionState("error");
    };

    ws.onclose = () => {
      if (connectionIdRef.current !== thisConnectionId) return;
      setConnectionState("disconnected");
    };

    wsRef.current = ws;
  }, [buildWsUrl]);

  // Connect on mount, reconnect on disconnect
  useEffect(() => {
    connect();

    return () => {
      connectionIdRef.current += 1;
      wsRef.current?.close();
    };
  }, [connect]);

  // Auto-reconnect on disconnect
  useEffect(() => {
    if (connectionState === "disconnected" || connectionState === "error") {
      const timeout = setTimeout(() => {
        connect();
      }, 2000);
      return () => clearTimeout(timeout);
    }
  }, [connectionState, connect]);

  // Calculate max for network sparklines (for scaling)
  const maxNetworkRate = Math.max(
    ...networkRxHistory,
    ...networkTxHistory,
    1024 // Min 1KB/s scale
  );

  return (
    <div className={cn("grid grid-cols-3 gap-3", className)}>
      {/* CPU */}
      <MetricCard
        icon={Cpu}
        label="CPU"
        value={metrics ? `${metrics.cpu_percent.toFixed(1)}%` : "--"}
        subValue={metrics ? `${metrics.cpu_cores.length} cores` : undefined}
        bar={metrics ? { value: metrics.cpu_percent, max: 100 } : undefined}
        sparkline={{ data: cpuHistory, max: 100 }}
        color="cyan"
      />

      {/* Memory */}
      <MetricCard
        icon={MemoryStick}
        label="Memory"
        value={metrics ? `${metrics.memory_percent.toFixed(1)}%` : "--"}
        subValue={
          metrics
            ? `${formatBytes(metrics.memory_used)} / ${formatBytes(metrics.memory_total)}`
            : undefined
        }
        bar={metrics ? { value: metrics.memory_percent, max: 100 } : undefined}
        sparkline={{ data: memoryHistory, max: 100 }}
        color="green"
      />

      {/* Network */}
      <MetricCard
        icon={Wifi}
        label="Network"
        value={
          metrics
            ? `${formatBytesPerSec(metrics.network_rx_bytes_per_sec)}`
            : "--"
        }
        subValue={
          metrics
            ? `${formatBytesPerSec(metrics.network_tx_bytes_per_sec)}`
            : undefined
        }
        sparkline={{ data: networkRxHistory, max: maxNetworkRate }}
        color="orange"
      />

      {/* Connection indicator */}
      {connectionState !== "connected" && (
        <div className="col-span-3 flex items-center justify-center gap-2 py-2 text-xs text-white/40">
          <Activity className="h-3 w-3 animate-pulse" />
          {connectionState === "connecting"
            ? "Connecting..."
            : connectionState === "error"
            ? "Connection error - retrying..."
            : "Disconnected - reconnecting..."}
        </div>
      )}
    </div>
  );
}
