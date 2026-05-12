import fs from "node:fs"

const code = Number(process.env.JEKKO_NON_ZERO_EXIT_CODE ?? "1")
const marker = process.env.JEKKO_NON_ZERO_EXIT_MARKER

if (!marker) {
  throw new Error("JEKKO_NON_ZERO_EXIT_MARKER is required")
}

fs.writeFileSync(marker, "ran")
process.exitCode = code
