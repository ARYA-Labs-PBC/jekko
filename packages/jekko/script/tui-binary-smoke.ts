#!/usr/bin/env bun

import fs from "fs"

import { hostBinaryPath } from "./host-binary-path"

function run(binary: string, args: string[]) {
  const proc = Bun.spawnSync([binary, ...args], {
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      JEKKO_DISABLE_AUTOUPDATE: "1",
      JEKKO_DISABLE_LSP_DOWNLOAD: "1",
      JEKKO_DISABLE_MODELS_FETCH: "1",
      JEKKO_DISABLE_PRUNE: "1",
    },
  })
  const stdout = new TextDecoder().decode(proc.stdout)
  const stderr = new TextDecoder().decode(proc.stderr)
  if (proc.exitCode !== 0) {
    throw new Error(`${binary} ${args.join(" ")} failed with exit ${proc.exitCode}\nstdout:\n${stdout}\nstderr:\n${stderr}`)
  }
  return stdout + stderr
}

const binary = process.env.JEKKO_BIN || hostBinaryPath()
if (!fs.existsSync(binary)) {
  throw new Error(`Jekko binary is missing at ${binary}. Build it first with: bun --cwd packages/jekko ./script/build.ts --single --skip-install`)
}

const version = run(binary, ["--version"]).trim()
if (!version) throw new Error("Jekko binary --version returned empty output")

const help = run(binary, ["--help"])
if (!help.includes("start jekko tui")) {
  throw new Error("Jekko binary help does not expose the default TUI command")
}

const commandLines = help
  .split(/\r?\n/)
  .map((line) => line.trim())
  .filter(Boolean)
if (commandLines.some((line) => line === "web" || line.startsWith("web "))) {
  throw new Error("Jekko binary help still exposes a web command")
}
if (/browser UI|web UI|serve.*UI/i.test(help)) {
  throw new Error("Jekko binary help still advertises a browser UI surface")
}

console.log(`Jekko binary smoke passed: ${version}`)
console.log(binary)
