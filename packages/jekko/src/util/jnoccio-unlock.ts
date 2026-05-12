// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import fs from "fs"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import { randomBytes, scryptSync, createCipheriv, createDecipheriv } from "crypto"
import { Schema } from "effect"
import { Global } from "@jekko-ai/core/global"
import { optionalOmitUndefined, withStatics } from "@/util/schema"
import { zod } from "@/util/effect-zod"
import { JNOCCIO_ENCRYPTED_GIT_CRYPT_KEY, type JnoccioEncryptedGitCryptKeyEnvelope } from "./jnoccio-encrypted-key"
import { ensureJnoccioFusionServer } from "./jnoccio-server"

export { JNOCCIO_ENCRYPTED_GIT_CRYPT_KEY } from "./jnoccio-encrypted-key"

export const JNOCCIO_PROVIDER_ID = "jnoccio"
export const JNOCCIO_MODEL_ID = "jnoccio-fusion"
export const JNOCCIO_DEFAULT_BASE_URL = "http://127.0.0.1:4317/v1"
export const JNOCCIO_DEFAULT_API_KEY = "jnoccio-local"
export const JNOCCIO_UNLOCK_SECRET_PATTERN = /^[A-Za-z0-9_-]{128}$/
export const JNOCCIO_DEFAULT_UNLOCK_SECRET_PATH = "~/jnoccio-fusion.unlock"
export const JNOCCIO_ENCRYPTION_AAD = "jnoccio-fusion-git-crypt-key-v1" as const
export const JNOCCIO_ENCRYPTION_PARAMS = {
  N: 262144,
  r: 8,
  p: 1,
  maxmem: 536870912,
} as const

export const JnoccioUnlockInput = Schema.Struct({
  unlockSecret: optionalOmitUndefined(Schema.String),
  keyPath: optionalOmitUndefined(Schema.String),
})
  .annotate({ identifier: "JnoccioUnlockInput" })
  .pipe(withStatics((s) => ({ zod: zod(s) })))
export type JnoccioUnlockInput = Schema.Schema.Type<typeof JnoccioUnlockInput>

export const JnoccioUnlockResult = Schema.Struct({
  status: Schema.Literals(["unlocked", "needs_secret", "error"]),
  message: Schema.String,
  envPath: optionalOmitUndefined(Schema.String),
  envCreated: Schema.Boolean,
  secretSaved: optionalOmitUndefined(Schema.Boolean),
})
  .annotate({ identifier: "JnoccioUnlockResult" })
  .pipe(withStatics((s) => ({ zod: zod(s) })))
export type JnoccioUnlockResult = Schema.Schema.Type<typeof JnoccioUnlockResult>

export type CommandResult = {
  exitCode: number
  stdout?: string
  stderr?: string
}

export type CommandRunner = (command: string, args: string[], options: { cwd: string }) => Promise<CommandResult>

export type UnlockOptions = {
  repoRoot?: string
  runner?: CommandRunner
  secretPath?: string
  envelope?: JnoccioEncryptedGitCryptKeyEnvelope
}

type EnvelopeParams = {
  N: number
  r: number
  p: number
  maxmem: number
}

export function normalizeJnoccioUnlockSecret(input: string) {
  const compact = input
    .replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, "")
    .replace(/\s+/g, "")
    .replace(/[^A-Za-z0-9_-]/g, "")

  if (compact.length === 134 && compact.startsWith("200") && compact.endsWith("201")) {
    return compact.slice(3, -3)
  }

  return compact
}

export function isValidUnlockSecret(input: string) {
  return JNOCCIO_UNLOCK_SECRET_PATTERN.test(normalizeJnoccioUnlockSecret(input))
}

export function findRepoRootFrom(start: string | undefined) {
  if (!start) return
  let current = path.resolve(expandHome(start))
  try {
    const stat = fs.statSync(current)
    if (stat.isFile()) current = path.dirname(current)
  } catch {}

  while (true) {
    if (fs.existsSync(path.join(current, "jnoccio-fusion")) && fs.existsSync(path.join(current, ".git"))) {
      return current
    }
    const parent = path.dirname(current)
    if (parent === current) return
    current = parent
  }
}

// Global registry of the jnoccio-fusion repo root. Once a user successfully
// unlocks (or jekko discovers) the repo from any directory, the absolute path
// is cached here so that future jekko invocations from any directory on the
// machine (e.g. ~/code/xdoug/) can locate jnoccio-fusion without depending on
// the current working directory containing the repo.
export function jnoccioGlobalConfigPath() {
  return path.join(Global.Path.state, "jnoccio.json")
}

type GlobalJnoccioConfig = {
  repo_root?: string
  registered_at?: string
}

const GlobalJnoccioConfigSchema = Schema.Struct({
  repo_root: optionalOmitUndefined(Schema.String),
  registered_at: optionalOmitUndefined(Schema.String),
})
const decodeGlobalJnoccioConfig = Schema.decodeUnknownSync(GlobalJnoccioConfigSchema)

export function readGlobalJnoccioRepoRoot(): string | undefined {
  try {
    const raw = fs.readFileSync(jnoccioGlobalConfigPath(), "utf8")
    const parsed: GlobalJnoccioConfig = decodeGlobalJnoccioConfig(JSON.parse(raw))
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (!parsed.repo_root) return undefined
    // Validate the cached path still has the repo layout we expect.
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (!fs.existsSync(path.join(parsed.repo_root, "jnoccio-fusion"))) return undefined
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (!fs.existsSync(path.join(parsed.repo_root, ".git"))) return undefined
    return parsed.repo_root
  } catch {
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    return undefined
  }
}

export function writeGlobalJnoccioRepoRoot(repoRoot: string) {
  try {
    const data: GlobalJnoccioConfig = {
      repo_root: repoRoot,
      registered_at: new Date().toISOString(),
    }
    fs.mkdirSync(path.dirname(jnoccioGlobalConfigPath()), { recursive: true })
    fs.writeFileSync(jnoccioGlobalConfigPath(), JSON.stringify(data, null, 2), { mode: 0o644 })
  } catch {
    // Non-fatal: global registration is a convenience; unlock still works locally.
  }
}

export function repoRootFromSource() {
  return (
    findRepoRootFrom(process.env.JNOCCIO_REPO_ROOT) ??
    readGlobalJnoccioRepoRoot() ??
    findRepoRootFrom(process.cwd()) ??
    findRepoRootFrom(import.meta.dir) ??
    path.resolve(import.meta.dir, "../../../..")
  )
}

export function expandHome(input: string) {
  const value = input.trim()
  if (value === "~") return os.homedir()
  if (value.startsWith("~/")) return path.join(os.homedir(), value.slice(2))
  return value
}

export function jnoccioFusionRoot(repoRoot = repoRootFromSource()) {
  return path.join(repoRoot, "jnoccio-fusion")
}

export function jnoccioEnvPath(repoRoot = repoRootFromSource()) {
  return path.join(jnoccioFusionRoot(repoRoot), ".env.jnoccio")
}

export function jnoccioEnvExamplePath(repoRoot = repoRootFromSource()) {
  return path.join(jnoccioFusionRoot(repoRoot), ".env.jnoccio.example")
}

export function jnoccioUnlockSecretPath() {
  return expandHome(process.env.JNOCCIO_UNLOCK_SECRET_PATH ?? JNOCCIO_DEFAULT_UNLOCK_SECRET_PATH)
}

function hasPlaintextSignals(repoRoot: string) {
  try {
    const cargo = fs.readFileSync(path.join(jnoccioFusionRoot(repoRoot), "Cargo.toml"), "utf8")
    if (!cargo.includes("[package]") || !cargo.includes('name = "jnoccio-fusion"')) return false

    const config = JSON.parse(fs.readFileSync(path.join(jnoccioFusionRoot(repoRoot), "config/server.json"), "utf8"))
    return config?.provider === JNOCCIO_PROVIDER_ID && config?.model === `${JNOCCIO_PROVIDER_ID}/${JNOCCIO_MODEL_ID}`
  } catch {
    return false
  }
}

export function isJnoccioFusionUnlocked(repoRoot = repoRootFromSource()) {
  return hasPlaintextSignals(repoRoot)
}

export function isJnoccioFusionConfigured(repoRoot = repoRootFromSource()) {
  return isJnoccioFusionUnlocked(repoRoot) && fs.existsSync(jnoccioEnvPath(repoRoot))
}

function deriveKey(secret: string, salt: Buffer, params: EnvelopeParams) {
  return scryptSync(Buffer.from(normalizeJnoccioUnlockSecret(secret), "utf8"), salt, 32, params)
}

export function encryptJnoccioGitCryptKey(
  rawKey: Buffer,
  secret: string,
  options?: {
    salt?: Buffer
    iv?: Buffer
    params?: EnvelopeParams
    aad?: string
  },
): JnoccioEncryptedGitCryptKeyEnvelope {
  const salt = options?.salt ?? randomBytes(16)
  const iv = options?.iv ?? randomBytes(12)
  const params = options?.params ?? JNOCCIO_ENCRYPTION_PARAMS
  const aad = options?.aad ?? JNOCCIO_ENCRYPTION_AAD
  const key = deriveKey(secret, salt, params)
  const cipher = createCipheriv("aes-256-gcm", key, iv)
  cipher.setAAD(Buffer.from(aad, "utf8"))
  const ciphertext = Buffer.concat([cipher.update(rawKey), cipher.final()])
  const tag = cipher.getAuthTag()

  return {
    version: 1,
    kdf: "scrypt",
    cipher: "aes-256-gcm",
    salt: salt.toString("base64url"),
    iv: iv.toString("base64url"),
    tag: tag.toString("base64url"),
    ciphertext: ciphertext.toString("base64url"),
    params,
    aad: aad as JnoccioEncryptedGitCryptKeyEnvelope["aad"],
  }
}

export function decryptJnoccioGitCryptKey(
  envelope: JnoccioEncryptedGitCryptKeyEnvelope,
  secret: string,
): Buffer {
  const salt = Buffer.from(envelope.salt, "base64url")
  const iv = Buffer.from(envelope.iv, "base64url")
  const tag = Buffer.from(envelope.tag, "base64url")
  const ciphertext = Buffer.from(envelope.ciphertext, "base64url")
  const key = deriveKey(secret, salt, envelope.params)
  const decipher = createDecipheriv(envelope.cipher, key, iv)
  decipher.setAAD(Buffer.from(envelope.aad, "utf8"))
  decipher.setAuthTag(tag)
  return Buffer.concat([decipher.update(ciphertext), decipher.final()])
}

export { unlockJnoccioFusion } from "./jnoccio-unlock-flow"
