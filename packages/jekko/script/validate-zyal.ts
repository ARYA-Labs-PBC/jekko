import { readFile } from "node:fs/promises"
import { Effect } from "effect"
import { detectZyal } from "../src/agent-script/activation"
import { parseZyal } from "../src/agent-script/parser"

async function main() {
  const files = process.argv.slice(2).filter((arg) => arg !== "--")
  if (files.length === 0) {
    throw new Error("usage: bun run script/validate-zyal.ts -- <file> [file...]")
  }

  for (const file of files) {
    const text = await readFile(file, "utf8")
    const parsed = await Effect.runPromise(parseZyal(text))
    const detection = detectZyal(text)
    if (detection.kind !== "preview") {
      throw new Error(`${file}: detectZyal returned ${detection.kind}`)
    }
    if (parsed.preview.runtime_sentinel_version !== "v1") {
      throw new Error(`${file}: unexpected runtime_sentinel_version ${parsed.preview.runtime_sentinel_version}`)
    }
  }

  console.log(`validated ${files.length} ZYAL files`)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error))
  process.exitCode = 1
})
