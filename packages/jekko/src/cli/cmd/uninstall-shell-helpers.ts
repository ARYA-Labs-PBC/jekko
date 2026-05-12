import fs from "fs/promises"
import os from "os"
import path from "path"
import * as prompts from "@clack/prompts"
import { Filesystem } from "@/util/filesystem"

type ShellConfigReadState = { kind: "readable"; content: string } | { kind: "unreadable"; error: unknown }

const SHELL_CONFIG_READ_ATTEMPTS = 2
const SHELL_CONFIG_READ_RETRY_DELAY_MS = 25

export async function getShellConfigPathForUninstall(): Promise<string | undefined> {
  const shell = path.basename(process.env.SHELL || "bash")
  const home = os.homedir()
  const xdgConfig = process.env.XDG_CONFIG_HOME || path.join(home, ".config")

  const configFiles: Record<string, string[]> = {
    fish: [path.join(xdgConfig, "fish", "config.fish")],
    zsh: [
      path.join(home, ".zshrc"),
      path.join(home, ".zshenv"),
      path.join(xdgConfig, "zsh", ".zshrc"),
      path.join(xdgConfig, "zsh", ".zshenv"),
    ],
    bash: [
      path.join(home, ".bashrc"),
      path.join(home, ".bash_profile"),
      path.join(home, ".profile"),
      path.join(xdgConfig, "bash", ".bashrc"),
      path.join(xdgConfig, "bash", ".bash_profile"),
    ],
    ash: [path.join(home, ".ashrc"), path.join(home, ".profile")],
    sh: [path.join(home, ".profile")],
  }

  const candidates = configFiles[shell] || configFiles.bash

  for (const file of candidates) {
    const exists = await fs
      .access(file)
      .then(() => true)
      .catch(() => false)
    if (!exists) continue

    const content = await readShellConfigText(file)
    if (content.kind === "unreadable") {
      prompts.log.warn(`Skipping unreadable shell config ${shortenPath(file)}`)
      continue
    }

    if (content.content.includes("# jekko") || content.content.includes(".jekko/bin")) {
      return file
    }
  }
}

export async function cleanShellConfig(file: string) {
  const content = await Filesystem.readText(file)
  const lines = content.split("\n")

  const filtered: string[] = []
  let skip = false

  for (const line of lines) {
    const trimmed = line.trim()

    if (trimmed === "# jekko") {
      skip = true
      continue
    }

    if (skip) {
      skip = false
      if (trimmed.includes(".jekko/bin") || trimmed.includes("fish_add_path")) {
        continue
      }
    }

    if (
      (trimmed.startsWith("export PATH=") && trimmed.includes(".jekko/bin")) ||
      (trimmed.startsWith("fish_add_path") && trimmed.includes(".jekko"))
    ) {
      continue
    }

    filtered.push(line)
  }

  while (filtered.length > 0 && filtered[filtered.length - 1].trim() === "") {
    filtered.pop()
  }

  const output = filtered.join("\n") + "\n"
  await Filesystem.write(file, output)
}

export function shortenPath(p: string): string {
  const home = os.homedir()
  if (p.startsWith(home)) {
    return p.replace(home, "~")
  }
  return p
}

async function readShellConfigText(file: string): Promise<ShellConfigReadState> {
  let lastError: unknown

  for (let attempt = 1; attempt <= SHELL_CONFIG_READ_ATTEMPTS; attempt++) {
    try {
      return { kind: "readable", content: await Filesystem.readText(file) } as const
    } catch (error) {
      lastError = error
      if (attempt < SHELL_CONFIG_READ_ATTEMPTS) {
        await new Promise((resolve) => setTimeout(resolve, SHELL_CONFIG_READ_RETRY_DELAY_MS * attempt))
      }
    }
  }

  return { kind: "unreadable", error: lastError } as const
}
