import { readFile } from "node:fs/promises"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { renderZyalSpecMarkdown } from "../src/agent-script/schema-spec"

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const repoRoot = path.resolve(__dirname, "../../..")
const specPath = path.join(repoRoot, "docs/ZYAL/SPEC.md")

const mode = new Set(process.argv.slice(2))
const write = mode.has("--write")
const check = mode.has("--check") || !write

async function main() {
  const next = `${renderZyalSpecMarkdown()}\n`
  if (write) {
    await Bun.write(specPath, next)
    console.log(`wrote ${path.relative(repoRoot, specPath)}`)
    return
  }

  if (!check) {
    throw new Error("usage: bun --cwd packages/jekko ./script/generate-zyal-spec.ts --check|--write")
  }

  const current = await readFile(specPath, "utf8")
  if (current !== next) {
    process.stdout.write(next)
    throw new Error(`${path.relative(repoRoot, specPath)} is out of date`)
  }
  console.log(`checked ${path.relative(repoRoot, specPath)}`)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error))
  process.exitCode = 1
})
