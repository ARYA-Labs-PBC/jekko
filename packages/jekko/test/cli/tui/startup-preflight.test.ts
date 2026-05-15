import { expect, spyOn, test } from "bun:test"
import fs from "fs/promises"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import * as Which from "../../../src/util/which"
import { runStartupPreflight } from "../../../src/cli/cmd/tui/startup-preflight"

test("startup preflight passes when the env file and git-crypt exist", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      const homeDir = path.join(dir, "home")
      await fs.mkdir(path.join(dir, "jnoccio-fusion"), { recursive: true })
      await fs.writeFile(path.join(dir, "jnoccio-fusion", ".env.jnoccio.example"), "OPENROUTER_API_KEY=\n")
      await fs.mkdir(homeDir, { recursive: true })
      await fs.writeFile(path.join(homeDir, ".env.jnoccio"), "JNOCCIO_DEVELOPER_KEY=fake\n")
      await fs.writeFile(path.join(dir, "model-keys.env"), "OPENAI_API_KEY=fake-openai-key\n")
    },
  })

  const originalModelKeys = process.env.JEKKO_MODEL_KEYS_FILE
  process.env.JEKKO_MODEL_KEYS_FILE = path.join(tmp.path, "model-keys.env")
  const which = spyOn(Which, "which").mockImplementation((cmd) => {
    if (cmd === "git-crypt") return "/opt/homebrew/bin/git-crypt"
    if (cmd === "brew") return "/opt/homebrew/bin/brew"
    return null
  })

  try {
    expect(await runStartupPreflight(tmp.path, path.join(tmp.path, "home"))).toBe(true)
  } finally {
    which.mockRestore()
    if (originalModelKeys === undefined) delete process.env.JEKKO_MODEL_KEYS_FILE
    else process.env.JEKKO_MODEL_KEYS_FILE = originalModelKeys
  }
})

test("startup preflight fails when the Jnoccio env file is missing", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      const homeDir = path.join(dir, "home")
      await fs.mkdir(path.join(dir, "jnoccio-fusion"), { recursive: true })
      await fs.writeFile(path.join(dir, "jnoccio-fusion", ".env.jnoccio.example"), "OPENROUTER_API_KEY=\n")
      await fs.writeFile(path.join(dir, "model-keys.env"), "OPENAI_API_KEY=fake-openai-key\n")
      await fs.mkdir(homeDir, { recursive: true })
    },
  })

  const originalModelKeys = process.env.JEKKO_MODEL_KEYS_FILE
  process.env.JEKKO_MODEL_KEYS_FILE = path.join(tmp.path, "model-keys.env")
  const which = spyOn(Which, "which").mockImplementation((cmd) => {
    if (cmd === "git-crypt") return "/opt/homebrew/bin/git-crypt"
    if (cmd === "brew") return "/opt/homebrew/bin/brew"
    return null
  })

  try {
    expect(await runStartupPreflight(tmp.path, path.join(tmp.path, "home"))).toBe(false)
  } finally {
    which.mockRestore()
    if (originalModelKeys === undefined) delete process.env.JEKKO_MODEL_KEYS_FILE
    else process.env.JEKKO_MODEL_KEYS_FILE = originalModelKeys
  }
})

test("startup preflight fails when model keys are missing", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      const homeDir = path.join(dir, "home")
      await fs.mkdir(path.join(dir, "jnoccio-fusion"), { recursive: true })
      await fs.writeFile(path.join(dir, "jnoccio-fusion", ".env.jnoccio.example"), "OPENROUTER_API_KEY=\n")
      await fs.mkdir(homeDir, { recursive: true })
      await fs.writeFile(path.join(homeDir, ".env.jnoccio"), "JNOCCIO_DEVELOPER_KEY=fake\n")
    },
  })

  const originalContent = process.env.JEKKO_MODEL_KEYS_CONTENT
  process.env.JEKKO_MODEL_KEYS_CONTENT = "OPENAI_API_KEY=\n"
  const which = spyOn(Which, "which").mockImplementation((cmd) => {
    if (cmd === "git-crypt") return "/opt/homebrew/bin/git-crypt"
    if (cmd === "brew") return "/opt/homebrew/bin/brew"
    return null
  })
  const writes: string[] = []
  const write = spyOn(process.stdout, "write").mockImplementation((chunk: string | Uint8Array) => {
    writes.push(String(chunk))
    return true
  })

  try {
    expect(await runStartupPreflight(tmp.path, path.join(tmp.path, "home"))).toBe(false)
    expect(writes.join("")).toContain("No model keys found.")
    expect(writes.join("")).toContain("~/.jekko/jekko.env")
    expect(writes.join("")).toContain("jekko keys init")
  } finally {
    write.mockRestore()
    which.mockRestore()
    if (originalContent === undefined) delete process.env.JEKKO_MODEL_KEYS_CONTENT
    else process.env.JEKKO_MODEL_KEYS_CONTENT = originalContent
  }
})

test("startup preflight hides git-crypt guidance when no developer key is available", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      const homeDir = path.join(dir, "home")
      await fs.mkdir(path.join(dir, "jnoccio-fusion"), { recursive: true })
      await fs.writeFile(path.join(dir, "jnoccio-fusion", ".env.jnoccio.example"), "OPENROUTER_API_KEY=\n")
      await fs.mkdir(homeDir, { recursive: true })
    },
  })

  const originalModelKeys = process.env.JEKKO_MODEL_KEYS_FILE
  process.env.JEKKO_MODEL_KEYS_FILE = path.join(tmp.path, "model-keys.env")
  const which = spyOn(Which, "which").mockImplementation((cmd) => {
    if (cmd === "git-crypt") throw new Error("git-crypt should not be probed without a developer key")
    if (cmd === "brew") return "/opt/homebrew/bin/brew"
    return null
  })
  const writes: string[] = []
  const write = spyOn(process.stdout, "write").mockImplementation((chunk: string | Uint8Array) => {
    writes.push(String(chunk))
    return true
  })

  try {
    expect(await runStartupPreflight(tmp.path, path.join(tmp.path, "home"))).toBe(false)
    expect(writes.join("")).not.toContain("git-crypt")
  } finally {
    write.mockRestore()
    which.mockRestore()
    if (originalModelKeys === undefined) delete process.env.JEKKO_MODEL_KEYS_FILE
    else process.env.JEKKO_MODEL_KEYS_FILE = originalModelKeys
  }
})

test("startup preflight prints the exact brew install command when developer key is present", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      const homeDir = path.join(dir, "home")
      await fs.mkdir(path.join(dir, "jnoccio-fusion"), { recursive: true })
      await fs.writeFile(path.join(dir, "jnoccio-fusion", ".env.jnoccio.example"), "OPENROUTER_API_KEY=\n")
      await fs.mkdir(homeDir, { recursive: true })
      await fs.writeFile(path.join(homeDir, ".env.jnoccio"), "JNOCCIO_DEVELOPER_KEY=fake\n")
      await fs.writeFile(path.join(dir, "model-keys.env"), "OPENAI_API_KEY=fake-openai-key\n")
    },
  })

  const originalModelKeys = process.env.JEKKO_MODEL_KEYS_FILE
  process.env.JEKKO_MODEL_KEYS_FILE = path.join(tmp.path, "model-keys.env")
  const which = spyOn(Which, "which").mockImplementation((cmd) => {
    if (cmd === "git-crypt") return null
    if (cmd === "brew") return "/opt/homebrew/bin/brew"
    return null
  })
  const writes: string[] = []
  const write = spyOn(process.stdout, "write").mockImplementation((chunk: string | Uint8Array) => {
    writes.push(String(chunk))
    return true
  })

  try {
    expect(await runStartupPreflight(tmp.path, path.join(tmp.path, "home"))).toBe(false)
    expect(writes.join("")).toContain("/opt/homebrew/bin/brew install git-crypt")
  } finally {
    write.mockRestore()
    which.mockRestore()
    if (originalModelKeys === undefined) delete process.env.JEKKO_MODEL_KEYS_FILE
    else process.env.JEKKO_MODEL_KEYS_FILE = originalModelKeys
  }
})
