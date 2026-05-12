import { useTheme } from "@tui/context/theme"

export type DaemonBannerProps = {
  run: any
}

export function formatDaemonBanner(run: any) {
  const task = run?.active_task ?? run?.task
  const pass = run?.active_pass ?? run?.pass
  const readiness = (() => {
    const value = task?.readiness_score ?? run?.readiness_score
    return typeof value === "number" ? value.toFixed(2) : "--"
  })()
  return `∞ FOREVER · ${run?.spec_json?.job?.name ?? run?.id} · iter ${run?.iteration} · ${run?.phase} · task ${
    task?.title ?? "--"
  } · pass ${pass?.pass_type ?? "--"} · ready ${readiness}`
}

export function DaemonBanner(props: DaemonBannerProps) {
  const { theme } = useTheme()
  const run = () => props.run
  return (
    <box paddingLeft={3} paddingBottom={1}>
      <text fg={theme.warning}>{formatDaemonBanner(run())}</text>
    </box>
  )
}
