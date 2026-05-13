import { describe, expect, test } from "bun:test"
import fs from "fs"
import os from "os"
import path from "path"
import {
  __readHistoryFileForTests,
  useJankuraiHistory,
} from "../../../src/cli/cmd/tui/context/jankurai-history"

function tempFile(content: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "jankurai-history-"))
  const file = path.join(dir, "score-history.jsonl")
  fs.writeFileSync(file, content)
  return file
}

describe("jankurai-history.__readHistoryFileForTests", () => {
  test("empty file yields empty history", () => {
    __readHistoryFileForTests(tempFile(""))
    expect(useJankuraiHistory()()).toEqual([])
  })

  test("parses canonical JSONL with `ts` and `score`", () => {
    const file = tempFile(
      [
        JSON.stringify({ ts: 1, score: 80, hardFindings: 5 }),
        JSON.stringify({ ts: 2, score: 82, hardFindings: 4 }),
        "",
      ].join("\n"),
    )
    __readHistoryFileForTests(file)
    const history = useJankuraiHistory()()
    expect(history.length).toBe(2)
    expect(history[0]?.score).toBe(80)
    expect(history[1]?.hardFindings).toBe(4)
  })

  test("also accepts `generated_at` + decision substructure", () => {
    const file = tempFile(
      JSON.stringify({
        generated_at: 1_700_000_000,
        score: 78,
        decision: { hard_findings: 12, soft_findings: 47 },
        caps_applied: [{ id: "x" }],
      }) + "\n",
    )
    __readHistoryFileForTests(file)
    const history = useJankuraiHistory()()
    expect(history.length).toBe(1)
    expect(history[0]?.ts).toBe(1_700_000_000)
    expect(history[0]?.hardFindings).toBe(12)
    expect(history[0]?.softFindings).toBe(47)
    expect(history[0]?.capsApplied).toBe(1)
  })

  test("malformed lines are skipped without failing", () => {
    const file = tempFile(
      [JSON.stringify({ ts: 1, score: 50 }), "{not json}", JSON.stringify({ ts: 2, score: 55 })].join(
        "\n",
      ) + "\n",
    )
    __readHistoryFileForTests(file)
    const history = useJankuraiHistory()()
    expect(history.length).toBe(2)
    expect(history[0]?.ts).toBe(1)
    expect(history[1]?.ts).toBe(2)
  })
})
