import { afterEach, describe, expect, test } from "bun:test"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import {
  decryptJnoccioGitCryptKey,
  encryptJnoccioGitCryptKey,
  normalizeJnoccioUnlockSecret,
  unlockJnoccioFusion,
} from "../../src/util/jnoccio-unlock"
import { JNOCCIO_ENCRYPTED_GIT_CRYPT_KEY } from "../../src/util/jnoccio-encrypted-key"

const tempDirs: string[] = []

async function tempRepo(options: { plaintext?: boolean; env?: string } = {}) {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "jnoccio-unlock-test-"))
  tempDirs.push(root)
  const fusion = path.join(root, "jnoccio-fusion")
  await fsp.mkdir(path.join(fusion, "config"), { recursive: true })
  await fsp.writeFile(path.join(root, "key.git-crypt-key"), "fake-key")
  await fsp.writeFile(path.join(fusion, ".env.jnoccio.example"), "OPENROUTER_API_KEY=\n")
  if (options.env !== undefined) await fsp.writeFile(path.join(fusion, ".env.jnoccio"), options.env)

  if (options.plaintext ?? true) {
    await fsp.writeFile(path.join(fusion, "Cargo.toml"), '[package]\nname = "jnoccio-fusion"\n')
    await fsp.writeFile(
      path.join(fusion, "config/server.json"),
      JSON.stringify({ provider: "jnoccio", model: "jnoccio/jnoccio-fusion" }),
    )
  } else {
    await fsp.writeFile(path.join(fusion, "Cargo.toml"), "\u0000GITCRYPT\u0000locked")
    await fsp.writeFile(path.join(fusion, "config/server.json"), "\u0000GITCRYPT\u0000locked")
  }

  return {
    root,
    keyPath: path.join(root, "key.git-crypt-key"),
    envPath: path.join(fusion, ".env.jnoccio"),
    fusion,
  }
}

function unlockSecret() {
  return "A".repeat(128)
}

function encryptedEnvelope(secret = unlockSecret(), rawKey = Buffer.from("fake-git-crypt-key")) {
  return encryptJnoccioGitCryptKey(rawKey, secret, {
    salt: Buffer.from("00112233445566778899aabbccddeeff", "hex"),
    iv: Buffer.from("00112233445566778899aabb", "hex"),
  })
}

async function writeUnlockedSignals(fusion: string) {
  await fsp.writeFile(path.join(fusion, "Cargo.toml"), '[package]\nname = "jnoccio-fusion"\n')
  await fsp.writeFile(path.join(fusion, "config/server.json"), JSON.stringify({ provider: "jnoccio", model: "jnoccio/jnoccio-fusion" }))
}

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
})

describe("unlockJnoccioFusion", () => {
  test("rejects missing key paths before running git-crypt", async () => {
    const repo = await tempRepo({ plaintext: false })
    let calls = 0
    const result = await unlockJnoccioFusion(
      { keyPath: path.join(repo.root, "missing.key") },
      {
        repoRoot: repo.root,
        runner: async () => {
          calls += 1
          return { exitCode: 0 }
        },
      },
    )

    expect(result.status).toBe("error")
    expect(result.message).toContain("could not be read")
    expect(calls).toBe(0)
  })

  test("returns a friendly message when git-crypt is unavailable", async () => {
    const repo = await tempRepo({ plaintext: false })
    const result = await unlockJnoccioFusion(
      { keyPath: repo.keyPath },
      {
        repoRoot: repo.root,
        runner: async () => ({ exitCode: 127, stderr: "not found" }),
      },
    )

    expect(result).toMatchObject({
      status: "error",
      envCreated: false,
    })
    expect(result.message).toContain("git-crypt is not installed")
  })

  test("returns a retryable error for invalid keys", async () => {
    const repo = await tempRepo({ plaintext: false })
    const result = await unlockJnoccioFusion(
      { keyPath: repo.keyPath },
      {
        repoRoot: repo.root,
        runner: async () => ({ exitCode: 1, stderr: "bad key" }),
      },
    )

    expect(result.status).toBe("error")
    expect(result.message).toContain("did not unlock")
  })

  test("treats an already unlocked repo as success when git-crypt refuses to unlock again", async () => {
    const repo = await tempRepo()
    const result = await unlockJnoccioFusion(
      { keyPath: repo.keyPath },
      {
        repoRoot: repo.root,
        runner: async () => ({ exitCode: 1, stderr: "already unlocked" }),
      },
    )

    expect(result).toMatchObject({
      status: "unlocked",
      envPath: repo.envPath,
      envCreated: true,
    })
  })

  test("short-circuits an already unlocked plaintext repo and creates .env.jnoccio", async () => {
    const repo = await tempRepo()
    let calls = 0
    const result = await unlockJnoccioFusion(
      { keyPath: repo.keyPath },
      {
        repoRoot: repo.root,
        runner: async () => {
          calls += 1
          return { exitCode: 0 }
        },
      },
    )

    expect(calls).toBe(0)
    expect(result).toMatchObject({
      status: "unlocked",
      envPath: repo.envPath,
      envCreated: true,
    })
    await expect(fsp.readFile(repo.envPath, "utf8")).resolves.toBe("OPENROUTER_API_KEY=\n")
  })

  test("does not overwrite an existing .env.jnoccio", async () => {
    const repo = await tempRepo({ env: "OPENROUTER_API_KEY=local\n" })
    const result = await unlockJnoccioFusion(
      { keyPath: repo.keyPath },
      {
        repoRoot: repo.root,
        runner: async () => ({ exitCode: 0 }),
      },
    )

    expect(result).toMatchObject({
      status: "unlocked",
      envCreated: false,
    })
    await expect(fsp.readFile(repo.envPath, "utf8")).resolves.toBe("OPENROUTER_API_KEY=local\n")
  })

  test("fails when git-crypt reports success but files remain locked", async () => {
    const repo = await tempRepo({ plaintext: false })
    const result = await unlockJnoccioFusion(
      { keyPath: repo.keyPath },
      {
        repoRoot: repo.root,
        runner: async () => ({ exitCode: 0 }),
      },
    )

    expect(result.status).toBe("error")
    expect(result.message).toContain("still locked")
  })

  test("returns needs_secret when the cache file is missing", async () => {
    const repo = await tempRepo({ plaintext: false })
    let calls = 0
    const secretPath = path.join(repo.root, "missing.unlock")
    const result = await unlockJnoccioFusion(
      {},
      {
        repoRoot: repo.root,
        secretPath,
        runner: async () => {
          calls += 1
          return { exitCode: 0 }
        },
      },
    )

    expect(result.status).toBe("needs_secret")
    expect(result.message).toContain("Enter your 128-character")
    expect(calls).toBe(0)
  })

  test("unlocks with a typed secret, writes the cache file, and deletes the ephemeral raw key", async () => {
    const repo = await tempRepo({ plaintext: false })
    const secret = unlockSecret()
    const rawKey = Buffer.from("fake-git-crypt-key")
    const tempKeyPaths: string[] = []
    const secretPath = path.join(repo.root, "jnoccio-fusion.unlock")

    const result = await unlockJnoccioFusion(
      { unlockSecret: secret },
      {
        repoRoot: repo.root,
        secretPath,
        envelope: encryptedEnvelope(secret, rawKey),
        runner: async (_command, args, { cwd }) => {
          expect(cwd).toBe(repo.root)
          const tempKeyPath = args[1]
          tempKeyPaths.push(tempKeyPath)
          await expect(fsp.readFile(tempKeyPath)).resolves.toEqual(rawKey)
          await writeUnlockedSignals(repo.fusion)
          return { exitCode: 0 }
        },
      },
    )

    expect(result).toMatchObject({
      status: "unlocked",
      envCreated: true,
      secretSaved: true,
    })
    await expect(fsp.readFile(secretPath, "utf8")).resolves.toBe(secret)
    await expect(fsp.stat(secretPath).then((stat) => stat.mode & 0o777)).resolves.toBe(0o600)
    await expect(fsp.readFile(path.join(repo.root, "jnoccio-fusion", ".env.jnoccio"), "utf8")).resolves.toBe(
      "OPENROUTER_API_KEY=\n",
    )
    expect(tempKeyPaths.length).toBe(1)
    await expect(fsp.stat(tempKeyPaths[0])).rejects.toThrow()
  })

  test("normalizes terminal paste artifacts around a typed secret", async () => {
    const repo = await tempRepo({ plaintext: false })
    const secret = unlockSecret()
    const rawKey = Buffer.from("fake-git-crypt-key")
    const secretPath = path.join(repo.root, "jnoccio-fusion.unlock")
    let calls = 0

    const result = await unlockJnoccioFusion(
      { unlockSecret: `200\x1b[200~${secret}\x1b[201~201` },
      {
        repoRoot: repo.root,
        secretPath,
        envelope: encryptedEnvelope(secret, rawKey),
        runner: async (_command, args) => {
          calls += 1
          await expect(fsp.readFile(args[1])).resolves.toEqual(rawKey)
          await writeUnlockedSignals(repo.fusion)
          return { exitCode: 0 }
        },
      },
    )

    expect(result).toMatchObject({
      status: "unlocked",
      envCreated: true,
      secretSaved: true,
    })
    expect(calls).toBe(1)
    await expect(fsp.readFile(secretPath, "utf8")).resolves.toBe(secret)
  })

  test("uses the cached secret without prompting and leaves the cache file intact", async () => {
    const repo = await tempRepo({ plaintext: false })
    const secret = unlockSecret()
    const secretPath = path.join(repo.root, "jnoccio-fusion.unlock")
    await fsp.writeFile(secretPath, secret, { mode: 0o600 })
    await fsp.chmod(secretPath, 0o600)
    const rawKey = Buffer.from("fake-git-crypt-key")
    let calls = 0

    const result = await unlockJnoccioFusion(
      {},
      {
        repoRoot: repo.root,
        secretPath,
        envelope: encryptedEnvelope(secret, rawKey),
        runner: async (_command, args) => {
          calls += 1
          await expect(fsp.readFile(args[1])).resolves.toEqual(rawKey)
          await writeUnlockedSignals(repo.fusion)
          return { exitCode: 0 }
        },
      },
    )

    expect(result).toMatchObject({
      status: "unlocked",
      envCreated: true,
    })
    expect(result.secretSaved).toBe(false)
    expect(calls).toBe(1)
    await expect(fsp.readFile(secretPath, "utf8")).resolves.toBe(secret)
    await expect(fsp.stat(secretPath).then((stat) => stat.mode & 0o777)).resolves.toBe(0o600)
  })

  test("rejects malformed secrets before decrypting", async () => {
    const repo = await tempRepo({ plaintext: false })
    let calls = 0
    const result = await unlockJnoccioFusion(
      { unlockSecret: "too-short" },
      {
        repoRoot: repo.root,
        envelope: encryptedEnvelope(),
        runner: async () => {
          calls += 1
          return { exitCode: 0 }
        },
      },
    )

    expect(result.status).toBe("error")
    expect(result.message).toContain("128 ASCII characters")
    expect(calls).toBe(0)
  })

  test("rejects a wrong 128-character secret without writing the cache", async () => {
    const repo = await tempRepo({ plaintext: false })
    const secretPath = path.join(repo.root, "jnoccio-fusion.unlock")
    let calls = 0
    const result = await unlockJnoccioFusion(
      { unlockSecret: "B".repeat(128) },
      {
        repoRoot: repo.root,
        secretPath,
        envelope: encryptedEnvelope(),
        runner: async () => {
          calls += 1
          return { exitCode: 0 }
        },
      },
    )

    expect(result.status).toBe("error")
    expect(result.message).toContain("Unlock secret was not valid")
    expect(calls).toBe(0)
    await expect(fsp.access(secretPath)).rejects.toThrow()
  })

  const localRawKey = process.env.JNOCCIO_RAW_KEY_PATH
  const localSecretPath = process.env.JNOCCIO_UNLOCK_SECRET_PATH
  test.skipIf(!localRawKey || !localSecretPath)(
    "embedded envelope round-trips with the local unlock secret",
    async () => {
      const secret = normalizeJnoccioUnlockSecret(await fsp.readFile(localSecretPath!, "utf8"))
      const expected = await fsp.readFile(localRawKey!)
      const decrypted = decryptJnoccioGitCryptKey(JNOCCIO_ENCRYPTED_GIT_CRYPT_KEY, secret)
      expect(decrypted.equals(expected)).toBe(true)
    },
  )

  test("deletes the ephemeral raw key file after a failed unlock", async () => {
    const repo = await tempRepo({ plaintext: false })
    const secret = unlockSecret()
    const rawKey = Buffer.from("fake-git-crypt-key")
    const tempKeyPaths: string[] = []

    const result = await unlockJnoccioFusion(
      { unlockSecret: secret },
      {
        repoRoot: repo.root,
        envelope: encryptedEnvelope(secret, rawKey),
        runner: async (_command, args) => {
          tempKeyPaths.push(args[1])
          await expect(fsp.readFile(args[1])).resolves.toEqual(rawKey)
          return { exitCode: 1, stderr: "bad key" }
        },
      },
    )

    expect(result.status).toBe("error")
    expect(result.message).toContain("Unlock secret was not valid")
    expect(tempKeyPaths.length).toBe(1)
    await expect(fsp.stat(tempKeyPaths[0])).rejects.toThrow()
    await expect(fsp.access(path.join(repo.root, "jnoccio-fusion.unlock"))).rejects.toThrow()
  })
})
