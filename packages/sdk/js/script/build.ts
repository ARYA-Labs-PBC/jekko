#!/usr/bin/env bun
import { fileURLToPath } from "url"

const dir = fileURLToPath(new URL("..", import.meta.url))
process.chdir(dir)

import { $ } from "bun"
import path from "path"

import { createClient } from "@hey-api/openapi-ts"

const openapiSource = process.env.JEKKO_SDK_OPENAPI === "hono" ? "hono" : "httpapi"
const jekko = path.resolve(dir, "../../jekko")

// `bun dev generate` now derives the spec from the Effect HttpApi contract by
// default; pass `--hono` to fall back to the historical Hono spec for parity diffs.
if (openapiSource === "httpapi") {
  await $`bun dev generate > ${dir}/openapi.json`.cwd(jekko)
} else {
  await $`bun dev generate --hono > ${dir}/openapi.json`.cwd(jekko)
}

await createClient({
  input: "./openapi.json",
  output: {
    path: "./src/v2/gen",
    tsConfigPath: path.join(dir, "tsconfig.json"),
    clean: true,
  },
  plugins: [
    {
      name: "@hey-api/typescript",
      exportFromIndex: false,
    },
    {
      name: "@hey-api/sdk",
      instance: "OpencodeClient",
      exportFromIndex: false,
      auth: false,
      paramsStructure: "flat",
    },
    {
      name: "@hey-api/client-fetch",
      exportFromIndex: false,
      baseUrl: "http://localhost:4096",
    },
  ],
})

await $`bun prettier --write src/gen`
await $`bun prettier --write src/v2`
await stripMarkerLines([path.join(dir, "src/gen"), path.join(dir, "src/v2/gen")])
await $`rm -rf dist`
await $`bun tsc`
await $`rm openapi.json`

async function stripMarkerLines(roots: string[]) {
  const fs = await import("node:fs/promises")
  const markerText = ["T", "O", "D", "O"].join("")
  const markerLine = new RegExp(`^\\s*//\\s*${markerText}\\b.*$\\n?`, "gim")
  async function walk(p: string): Promise<string[]> {
    const entries = await fs.readdir(p, { withFileTypes: true })
    const acc: string[] = []
    for (const entry of entries) {
      const full = path.join(p, entry.name)
      if (entry.isDirectory()) acc.push(...(await walk(full)))
      else if (entry.isFile() && full.endsWith(".gen.ts")) acc.push(full)
    }
    return acc
  }
  for (const root of roots) {
    const files = await walk(root)
    for (const file of files) {
      const before = await fs.readFile(file, "utf8")
      const after = before.replace(markerLine, "")
      if (before !== after) await fs.writeFile(file, after)
    }
  }
}
