#!/usr/bin/env node
// Reference concept persister for example 17. Reads a concept JSON from
// stdin and writes it to ~/.jankurai/concepts/<concept_id>.json plus updates
// the `_index.json` sibling so cross-repo recall is O(1).
//
// Expected stdin shape: { "concept_id": "...", "definition": "...",
//                         "derived_from": [...], "proof_refs": [...],
//                         "confidence": 0.5 }

import fs from "node:fs"
import os from "node:os"
import path from "node:path"

function readStdin() {
  return new Promise((resolve, reject) => {
    let buf = ""
    process.stdin.on("data", (chunk) => (buf += chunk))
    process.stdin.on("end", () => resolve(buf))
    process.stdin.on("error", reject)
  })
}

const root = process.env.JANKURAI_CONCEPT_ROOT ?? path.join(os.homedir(), ".jankurai", "concepts")
const indexPath = path.join(root, "_index.json")

const raw = await readStdin()
let parsed
try {
  parsed = JSON.parse(raw)
} catch (err) {
  process.stderr.write(`bad stdin JSON: ${err.message}\n`)
  process.exit(64)
}

if (typeof parsed?.concept_id !== "string" || parsed.concept_id.trim() === "") {
  process.stderr.write("missing concept_id\n")
  process.exit(64)
}

fs.mkdirSync(root, { recursive: true })
const sanitized = parsed.concept_id.replace(/[^A-Za-z0-9._-]+/g, "-")
const file = path.join(root, `${sanitized}.json`)
fs.writeFileSync(file, JSON.stringify(parsed, null, 2) + "\n", "utf-8")

let index
try {
  index = JSON.parse(fs.readFileSync(indexPath, "utf-8"))
} catch {
  index = { concepts: [] }
}
if (!Array.isArray(index.concepts)) index.concepts = []
const existing = index.concepts.findIndex((entry) => entry?.concept_id === parsed.concept_id)
const entry = {
  concept_id: parsed.concept_id,
  file: path.relative(root, file),
  derived_from: Array.isArray(parsed.derived_from) ? parsed.derived_from : [],
  confidence: typeof parsed.confidence === "number" ? parsed.confidence : 0.5,
  updated_at: Math.floor(Date.now() / 1000),
}
if (existing >= 0) index.concepts[existing] = entry
else index.concepts.push(entry)
fs.writeFileSync(indexPath, JSON.stringify(index, null, 2) + "\n", "utf-8")
process.stdout.write(`${file}\n`)
