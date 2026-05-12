import type { ZyalScript } from "@/agent-script/schema"

export type WorkerSummary = {
  readonly id: string
  readonly count: number
  readonly agent: string
  readonly isolation: "git_worktree" | "same_session"
}

export function workerSummaries(spec: ZyalScript): WorkerSummary[] {
  return spec.agents?.workers?.map((worker) => ({
    id: worker.id,
    count: worker.count,
    agent: worker.agent,
    isolation: worker.isolation ?? "git_worktree",
  })) ?? []
}

export function totalWorkerCount(spec: ZyalScript) {
  return workerSummaries(spec).reduce((sum, worker) => sum + worker.count, 0)
}

export * as DaemonWorkers from "./daemon-workers"

