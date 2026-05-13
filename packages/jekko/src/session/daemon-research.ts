import { mkdir, readdir, readFile, stat, writeFile } from "fs/promises"
import path from "path"
import { createHash } from "crypto"
import { Effect } from "effect"
import { InstanceState } from "@/effect/instance-state"
import { resolveInstanceRoot } from "@/project/instance-root"
import type { ZyalScript } from "@/agent-script/schema"
import type { DaemonStore } from "./daemon-store"

export type ResearchWorkItem = {
  readonly id: string
  readonly publication_hash: string
  readonly paper_path?: string
  readonly challenge_path?: string
  readonly role?: string
}

export type ResearchPreflightResult = {
  readonly enabled: boolean
  readonly artifactRoot: string
  readonly evidencePath: string
  readonly completePath: string
  readonly paperCount: number
  readonly challengeCount: number
  readonly rejectedCount: number
  readonly duplicateCount: number
  readonly workItems: ResearchWorkItem[]
}

export function hasResearchPipeline(spec: ZyalScript): boolean {
  const research = spec.research
  return !!(
    research?.paper_scan ||
    research?.full_text ||
    research?.dedupe ||
    research?.context_packing ||
    research?.question_bank ||
    research?.agent_trials ||
    research?.audit
  )
}

export function researchArtifactRoot(rootDir: string, spec: ZyalScript) {
  return path.resolve(rootDir, "target", "openqg", "research", spec.job.name, "latest")
}

export function questionBankRoot(rootDir: string, spec: ZyalScript) {
  return path.resolve(rootDir, spec.research?.question_bank?.output_root ?? "research/knowledge/question-bank")
}

export function runResearchPreflight(input: {
  run: DaemonStore.RunInfo
  spec: ZyalScript
  store: DaemonStore.Interface
}) {
  return Effect.gen(function* () {
    const rootCtx = yield* InstanceState.context
    const rootDir = resolveInstanceRoot(rootCtx)
    const artifactRoot = researchArtifactRoot(rootDir, input.spec)
    const bankRoot = questionBankRoot(rootDir, input.spec)
    const papersRoot = path.resolve(rootDir, input.spec.research?.question_bank?.papers_root ?? path.join(bankRoot, "papers"))
    const challengesRoot = path.resolve(rootDir, input.spec.research?.question_bank?.challenges_root ?? path.join(bankRoot, "challenges"))
    const rejectedRoot = path.resolve(rootDir, input.spec.research?.question_bank?.rejected_root ?? path.join(bankRoot, "rejected"))

    yield* Effect.promise(() => mkdir(artifactRoot, { recursive: true }))
    yield* Effect.promise(() => mkdir(path.join(artifactRoot, "receipts"), { recursive: true }))

    const papers = yield* Effect.promise(() => collectJsonFiles(papersRoot))
    const challenges = yield* Effect.promise(() => collectJsonFiles(challengesRoot))
    const rejected = yield* Effect.promise(() => collectJsonFiles(rejectedRoot))
    const duplicateCount = yield* Effect.promise(() => countDuplicatePublicationHashes(papers))
    const explicitWorkItems = input.spec.research?.question_bank?.work_items ?? []
    const workItems = explicitWorkItems.length > 0
      ? explicitWorkItems
      : challenges.map((challengePath, index) => ({
          id: `challenge-${index + 1}`,
          publication_hash: "unknown",
          challenge_path: path.relative(rootDir, challengePath),
          role: "answerer",
        }))

    const evidence = {
      run_id: input.run.id,
      zyal_job: input.spec.job.name,
      artifact_root: path.relative(rootDir, artifactRoot),
      question_bank_root: path.relative(rootDir, bankRoot),
      paper_count: papers.length,
      challenge_count: challenges.length,
      rejected_count: rejected.length,
      duplicate_count: duplicateCount,
      paper_scan: input.spec.research?.paper_scan ?? null,
      full_text: input.spec.research?.full_text ?? null,
      dedupe: input.spec.research?.dedupe ?? null,
      context_packing: input.spec.research?.context_packing ?? null,
      question_bank: {
        ...(input.spec.research?.question_bank ?? {}),
        work_items: workItems,
      },
      route_metadata_required: true,
      generated_at: new Date().toISOString(),
    }
    const evidencePath = path.join(artifactRoot, "evidence.json")
    const completePath = path.join(artifactRoot, "complete.ok")
    const workItemsPath = path.join(artifactRoot, "work-items.json")
    yield* Effect.promise(() => writeFile(evidencePath, JSON.stringify(evidence, null, 2) + "\n"))
    yield* Effect.promise(() => writeFile(workItemsPath, JSON.stringify({ work_items: workItems }, null, 2) + "\n"))
    yield* Effect.promise(() => writeFile(completePath, `${input.run.id}\n`))

    yield* input.store.appendEvent({
      runID: input.run.id,
      iteration: input.run.iteration,
      eventType: "research.preflight.completed",
      payload: {
        artifact_root: artifactRoot,
        evidence_path: evidencePath,
        complete_path: completePath,
        paper_count: papers.length,
        challenge_count: challenges.length,
        rejected_count: rejected.length,
        duplicate_count: duplicateCount,
        work_item_count: workItems.length,
      },
    })

    return {
      enabled: true,
      artifactRoot,
      evidencePath,
      completePath,
      paperCount: papers.length,
      challengeCount: challenges.length,
      rejectedCount: rejected.length,
      duplicateCount,
      workItems,
    } satisfies ResearchPreflightResult
  })
}

async function collectJsonFiles(root: string): Promise<string[]> {
  try {
    const info = await stat(root)
    if (!info.isDirectory()) return []
  } catch {
    return []
  }
  const out: string[] = []
  await walk(root, out)
  out.sort()
  return out
}

async function walk(root: string, out: string[]) {
  const entries = await readdir(root, { withFileTypes: true })
  for (const entry of entries) {
    const next = path.join(root, entry.name)
    if (entry.isDirectory()) {
      await walk(next, out)
    } else if (entry.isFile() && entry.name.endsWith(".json")) {
      out.push(next)
    }
  }
}

async function countDuplicatePublicationHashes(files: readonly string[]) {
  const seen = new Set<string>()
  let duplicates = 0
  for (const file of files) {
    const text = await readFile(file, "utf8")
    let hash: string | undefined
    try {
      const parsed = JSON.parse(text) as Record<string, unknown>
      hash = typeof parsed.publication_hash === "string" ? parsed.publication_hash : undefined
    } catch {
      hash = createHash("sha256").update(text).digest("hex")
    }
    if (!hash) continue
    if (seen.has(hash)) duplicates += 1
    seen.add(hash)
  }
  return duplicates
}
