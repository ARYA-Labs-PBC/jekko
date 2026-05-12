// Palette constants extracted from zyal.tsx for code shape compliance

export const numberFmt = new Intl.NumberFormat("en-US")
export const compactFmt = new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 1 })
export const moneyFmt = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
})
export const latencyFmt = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
})

// High-contrast neon palette for the sidebar's number values. Tuned to pop
// against the `jekko-gold` flash background (deep amber). All values are
// truecolor hex; opentui upgrades them via the COLORTERM=truecolor terminal
// hint that the TUI sets at boot. Each metric reads a different color so the
// eye can scan the panel by category instead of parsing labels.
export const NEON = {
  loops: "#FF40FF",      // magenta — loop counter
  tokensTotal: "#00FFFF", // cyan — headline aggregate
  tokensIn: "#FFD000",    // amber — input side
  tokensOut: "#00FF87",   // green — output side
  cache: "#7DF9FF",       // electric blue — cache reads/writes
  workersActive: "#BBFF00", // lime — fleet seats in use
  workersMax: "#A0A0A0",    // dim grey — capacity
  uptime: "#FFD000",        // amber — wall clock
  cost: "#FF1493",          // hot pink — money draws attention
  calls: "#00FFFF",         // cyan — request count
  wins: "#00FF87",          // green — success
  fails: "#FF4060",         // red — failure
  latency: "#FF9933",       // orange — milliseconds
  heartbeatLive: "#00FF87", // green — fresh
  heartbeatStale: "#FF4060", // red — late
  separator: "#A06030",      // muted gold
}

// Exit-tone palette. Same neon language as the live-metrics panel so the
// banner reads as a continuation, not a different feature. All bold.
export const EXIT_TONE: Record<string, { fg: string; label: string }> = {
  success: { fg: "#00FF87", label: "ZYAL SATISFIED" },
  warning: { fg: "#FFD000", label: "ZYAL PAUSED" },
  error: { fg: "#FF4060", label: "ZYAL EXITED" },
}