import fs from "fs"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import {
  decryptJnoccioGitCryptKey,
  expandHome,
  isJnoccioFusionUnlocked,
  isValidUnlockSecret,
  jnoccioEnvExamplePath,
  jnoccioEnvPath,
  jnoccioUnlockSecretPath,
  normalizeJnoccioUnlockSecret,
  readGlobalJnoccioRepoRoot,
  repoRootFromSource,
  writeGlobalJnoccioRepoRoot,
  JNOCCIO_ENCRYPTED_GIT_CRYPT_KEY,
  type CommandResult,
  type JnoccioUnlockInput,
  type JnoccioUnlockResult,
  type UnlockOptions,
} from "./jnoccio-unlock"
import { ensureJnoccioFusionServer } from "./jnoccio-server"

async function defaultRunner(command: string, args: string[], options: { cwd: string }): Promise<CommandResult> {
  try {
    const proc = Bun.spawn([command, ...args], {
      cwd: options.cwd,
      stdout: "pipe",
      stderr: "pipe",
    })
    const [stdout, stderr, exitCode] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
      proc.exited,
    ])
    return { exitCode, stdout, stderr }
  } catch (error) {
    if (error && typeof error === "object" && "code" in error && error.code === "ENOENT") {
      return { exitCode: 127, stderr: "git-crypt not found" }
    }
    throw error
  }
}

async function ensureEnvFile(repoRoot: string) {
  const envPath = jnoccioEnvPath(repoRoot)
  try {
    await fsp.access(envPath, fs.constants.F_OK)
    return { envPath, envCreated: false }
  } catch {}

  await fsp.copyFile(jnoccioEnvExamplePath(repoRoot), envPath, fs.constants.COPYFILE_EXCL)
  await fsp.chmod(envPath, 0o600)
  return { envPath, envCreated: true }
}

async function unlockedResult(repoRoot: string, secretSaved?: boolean): Promise<JnoccioUnlockResult> {
  try {
    const env = await ensureEnvFile(repoRoot)

    // Register this repo as the canonical jnoccio-fusion source globally so
    // jekko launched from any directory on the machine can find the binary,
    // env, and config without depending on $CWD.
    writeGlobalJnoccioRepoRoot(repoRoot)

    // Auto-start the jnoccio-fusion server in the background after unlock.
    // Fire-and-forget so it doesn't block the unlock response.
    ensureJnoccioFusionServer(repoRoot).catch(() => {})

    return {
      status: "unlocked",
      message: env.envCreated
        ? "Jnoccio Fusion is unlocked. Created .env.jnoccio with blank provider keys."
        : "Jnoccio Fusion is unlocked. Existing .env.jnoccio was left unchanged.",
      envPath: env.envPath,
      envCreated: env.envCreated,
      secretSaved,
    }
  } catch {
    return {
      status: "error",
      message: "Jnoccio Fusion unlocked, but .env.jnoccio could not be created from the example file.",
      envCreated: false,
      secretSaved,
    }
  }
}

async function needsSecretResult(_repoRoot: string, message?: string): Promise<JnoccioUnlockResult> {
  return {
    status: "needs_secret",
    message: message ?? "Enter your 128-character Jnoccio unlock secret to unlock Jnoccio Fusion.",
    envCreated: false,
  }
}

async function writeSecretFile(secretPath: string, secret: string) {
  await fsp.mkdir(path.dirname(secretPath), { recursive: true })
  await fsp.writeFile(secretPath, secret, { mode: 0o600 })
  await fsp.chmod(secretPath, 0o600)
}

async function readSecretFile(secretPath: string) {
  try {
    const content = await fsp.readFile(secretPath, "utf8")
    const secret = normalizeJnoccioUnlockSecret(content)
    return isValidUnlockSecret(secret) ? secret : undefined
  } catch {
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    return undefined
  }
}

async function unlockWithSecret(
  secret: string,
  options: Required<Pick<UnlockOptions, "repoRoot">> & UnlockOptions,
  inputSource: "cache" | "typed",
): Promise<JnoccioUnlockResult> {
  const repoRoot = options.repoRoot
  const runner = options.runner ?? defaultRunner
  const envelope = options.envelope ?? JNOCCIO_ENCRYPTED_GIT_CRYPT_KEY
  const secretPath = options.secretPath ?? jnoccioUnlockSecretPath()
  const normalized = normalizeJnoccioUnlockSecret(secret)

  if (!isValidUnlockSecret(normalized)) {
    return {
      status: "error",
      message: "Unlock secret must be exactly 128 ASCII characters from [A-Za-z0-9_-].",
      envCreated: false,
    }
  }

  let rawKey: Buffer
  try {
    rawKey = decryptJnoccioGitCryptKey(envelope, normalized)
  } catch {
    return {
      status: inputSource === "cache" ? "needs_secret" : "error",
      message:
        inputSource === "cache"
          ? "Stored unlock secret could not be used. Enter it again to refresh the cache."
          : "Unlock secret was not valid.",
      envCreated: false,
    }
  }
  const tempDir = await fsp.mkdtemp(path.join(os.tmpdir(), "jnoccio-unlock-"))
  const tempKeyPath = path.join(tempDir, "jnoccio-fusion.key")

  try {
    await fsp.writeFile(tempKeyPath, rawKey, { mode: 0o600 })
    await fsp.chmod(tempKeyPath, 0o600)

    let result: CommandResult
    try {
      result = await runner("git-crypt", ["unlock", tempKeyPath], { cwd: repoRoot })
    } catch {
      if (isJnoccioFusionUnlocked(repoRoot)) return unlockedResult(repoRoot, inputSource === "typed")
      return {
        status: "error",
        message: "git-crypt is not installed or could not be started.",
        envCreated: false,
      }
    }

    if (result.exitCode !== 0 && isJnoccioFusionUnlocked(repoRoot)) {
      return unlockedResult(repoRoot, inputSource === "typed")
    }

    if (result.exitCode === 127) {
      return {
        status: "error",
        // jankurai:allow HLT-001-DEAD-MARKER reason=user-facing-error-guidance-not-retry-logic expires=2027-01-01
        message: "git-crypt is not installed. Install git-crypt, then try again.",
        envCreated: false,
      }
    }

    if (result.exitCode !== 0) {
      return {
        status: "error",
        message: "Unlock secret was not valid.",
        envCreated: false,
      }
    }

    if (!isJnoccioFusionUnlocked(repoRoot)) {
      return {
        status: "error",
        message: "git-crypt reported success, but Jnoccio Fusion files are still locked.",
        envCreated: false,
      }
    }

    let secretSaved = inputSource === "typed"
    if (inputSource === "typed") {
      try {
        await writeSecretFile(secretPath, normalized)
      } catch {
        secretSaved = false
      }
    }

    return unlockedResult(repoRoot, secretSaved)
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true })
  }
}

export async function unlockJnoccioFusion(
  input: JnoccioUnlockInput = {},
  options: UnlockOptions = {},
): Promise<JnoccioUnlockResult> {
  const repoRoot = options.repoRoot ?? repoRootFromSource()

  if (isJnoccioFusionUnlocked(repoRoot)) {
    return unlockedResult(repoRoot, false)
  }

  const secretPath = options.secretPath ?? jnoccioUnlockSecretPath()

  if (input.keyPath) {
    const keyPath = expandHome(input.keyPath)
    if (!keyPath) {
      return {
        status: "error",
        message: "Choose a local git-crypt key file to unlock Jnoccio Fusion.",
        envCreated: false,
      }
    }

    try {
      const stat = await fsp.stat(keyPath)
      if (!stat.isFile()) {
        return {
          status: "error",
          message: "That path is not a readable key file.",
          envCreated: false,
        }
      }
      await fsp.access(keyPath, fs.constants.R_OK)
    } catch {
      return {
        status: "error",
        message: "That key file could not be read. Check the path and permissions.",
        envCreated: false,
      }
    }

    const runner = options.runner ?? defaultRunner
    let result: CommandResult
    try {
      result = await runner("git-crypt", ["unlock", keyPath], { cwd: repoRoot })
    } catch {
      if (isJnoccioFusionUnlocked(repoRoot)) return unlockedResult(repoRoot)
      return {
        status: "error",
        message: "git-crypt is not installed or could not be started.",
        envCreated: false,
      }
    }

    if (result.exitCode !== 0 && isJnoccioFusionUnlocked(repoRoot)) {
      return unlockedResult(repoRoot)
    }

    if (result.exitCode === 127) {
      return {
        status: "error",
        message: "git-crypt is not installed. Install git-crypt, then try the key file again.",
        envCreated: false,
      }
    }

    if (result.exitCode !== 0) {
      return {
        status: "error",
        message: "The key file did not unlock Jnoccio Fusion. Confirm you selected the project git-crypt key.",
        envCreated: false,
      }
    }

    if (!isJnoccioFusionUnlocked(repoRoot)) {
      return {
        status: "error",
        message: "git-crypt reported success, but Jnoccio Fusion files are still locked.",
        envCreated: false,
      }
    }

    return unlockedResult(repoRoot)
  }

  if (input.unlockSecret) {
    try {
      return await unlockWithSecret(input.unlockSecret, { ...options, repoRoot, secretPath }, "typed")
    } catch {
      return {
        status: "error",
        message: "Unlock secret was not valid.",
        envCreated: false,
      }
    }
  }

  const cachedSecret = await readSecretFile(secretPath)
  if (!cachedSecret) return needsSecretResult(repoRoot)

  return unlockWithSecret(cachedSecret, { ...options, repoRoot, secretPath }, "cache")
}
