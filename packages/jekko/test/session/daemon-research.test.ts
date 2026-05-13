import { describe, expect, test } from "bun:test"
import { Effect } from "effect"
import fs from "fs/promises"
import path from "path"
import { InstanceRef } from "../../src/effect/instance-ref"
import { runResearchPreflight } from "../../src/session/daemon-research"
import { tmpdir } from "../fixture/fixture"

describe("daemon research", () => {
  test("writes evidence, complete marker, and work item receipts", async () => {
    await using tmp = await tmpdir()
    const directory = tmp.path
    const paperRoot = path.join(directory, "research/knowledge/question-bank/papers")
    await fs.mkdir(paperRoot, { recursive: true })
    await fs.writeFile(
      path.join(paperRoot, "p1.json"),
      JSON.stringify({ publication_hash: "pub-1", body_hash: "body-1" }),
    )
    const events: any[] = []
    const spec = {
      job: { name: "research-smoke", objective: "smoke" },
      research: {
        version: "v1",
        paper_scan: { open_access: "required" },
        question_bank: {
          output_root: "research/knowledge/question-bank",
          work_items: [{ id: "work-1", publication_hash: "pub-1", role: "answerer" }],
        },
      },
    } as any
    const run = { id: "run-research-smoke", iteration: 0 } as any
    const result = await Effect.runPromise(
      runResearchPreflight({
        run,
        spec,
        store: {
          appendEvent: (input: any) =>
            Effect.sync(() => {
              events.push(input)
            }),
        } as any,
      }).pipe(
        Effect.provideService(InstanceRef, {
          directory,
          worktree: directory,
          project: { id: "proj", worktree: directory, sandboxes: [], time: { created: 0, updated: 0 } },
        } as any),
      ),
    )

    expect(result.paperCount).toBe(1)
    expect(result.workItems).toHaveLength(1)
    expect(await fs.readFile(result.evidencePath, "utf8")).toContain("work-1")
    expect(await fs.readFile(result.completePath, "utf8")).toContain(run.id)
    expect(events[0].eventType).toBe("research.preflight.completed")
  })
})
