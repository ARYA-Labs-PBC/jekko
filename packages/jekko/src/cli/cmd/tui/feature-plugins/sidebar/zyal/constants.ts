export const numberFmt = new Intl.NumberFormat("en-US")
export const compactFmt = new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 1 })
export const moneyFmt = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
})
export const latencyFmt = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 })

export const NEON = {
  loops: "#FF40FF",
  tokensTotal: "#00FFFF",
  tokensIn: "#FFD000",
  tokensOut: "#00FF87",
  cache: "#7DF9FF",
  workersActive: "#BBFF00",
  workersMax: "#A0A0A0",
  uptime: "#FFD000",
  cost: "#FF1493",
  calls: "#00FFFF",
  wins: "#00FF87",
  fails: "#FF4060",
  latency: "#FF9933",
  heartbeatLive: "#00FF87",
  heartbeatStale: "#FF4060",
  separator: "#A06030",
}

export const EXIT_TONE: Record<string, { fg: string; label: string }> = {
  success: { fg: "#00FF87", label: "ZYAL SATISFIED" },
  warning: { fg: "#FFD000", label: "ZYAL PAUSED" },
  error: { fg: "#FF4060", label: "ZYAL EXITED" },
}
