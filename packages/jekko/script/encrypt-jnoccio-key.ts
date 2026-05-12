import crypto from "crypto"
import fs from "fs/promises"
import path from "path"
import {
  encryptJnoccioGitCryptKey,
  expandHome,
  isValidUnlockSecret,
  JNOCCIO_DEFAULT_UNLOCK_SECRET_PATH,
  JNOCCIO_ENCRYPTION_AAD,
  JNOCCIO_ENCRYPTION_PARAMS,
  repoRootFromSource,
} from "../src/util/jnoccio-unlock"
import type { JnoccioEncryptedGitCryptKeyEnvelope } from "../src/util/jnoccio-encrypted-key"

type Args = {
  keyFile: string
  secretFile: string
  output: string
  rotateSecret: boolean
}

const SOFTWARE_KEY_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
const SOFTWARE_KEY_LENGTH = 128

function parseArgs(argv: string[]): Args {
  const result: Args = {
    keyFile: expandHome(process.env.JNOCCIO_GIT_CRYPT_KEY_PATH ?? "~/jnoccio-fusion.key"),
    secretFile: expandHome(process.env.JNOCCIO_UNLOCK_SECRET_PATH ?? JNOCCIO_DEFAULT_UNLOCK_SECRET_PATH),
    output: path.join(repoRootFromSource(), "packages/jekko/src/util/jnoccio-encrypted-key.ts"),
    rotateSecret: false,
  }

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === "--key-file") result.keyFile = expandHome(argv[++i] ?? result.keyFile)
    if (arg === "--secret-file") result.secretFile = expandHome(argv[++i] ?? result.secretFile)
    if (arg === "--output") result.output = path.resolve(argv[++i] ?? result.output)
    if (arg === "--rotate-secret") result.rotateSecret = true
  }

  return result
}

function generateSoftwareUnlockSecret() {
  let secret = ""
  const max = Math.floor(256 / SOFTWARE_KEY_ALPHABET.length) * SOFTWARE_KEY_ALPHABET.length
  while (secret.length < SOFTWARE_KEY_LENGTH) {
    for (const byte of crypto.randomBytes(SOFTWARE_KEY_LENGTH)) {
      if (byte >= max) continue
      secret += SOFTWARE_KEY_ALPHABET[byte % SOFTWARE_KEY_ALPHABET.length]
      if (secret.length === SOFTWARE_KEY_LENGTH) break
    }
  }
  return secret
}

async function readOrCreateSecret(secretFile: string, rotateSecret: boolean) {
  if (!rotateSecret) {
    try {
      const existingUnlockSecret = (await fs.readFile(secretFile, "utf8")).trim()
      if (!isValidUnlockSecret(existingUnlockSecret)) throw new Error(`Invalid unlock secret in ${secretFile}`)
      return { unlockSecret: existingUnlockSecret, created: false, rotated: false }
    } catch (error) {
      if (error instanceof Error && error.message.startsWith("Invalid unlock secret")) throw error
    }
  }

  const unlockSecret = generateSoftwareUnlockSecret()
  if (!isValidUnlockSecret(unlockSecret)) {
    throw new Error("Generated unlock secret was not 128 ASCII characters")
  }
  try {
    await fs.mkdir(path.dirname(secretFile), { recursive: true })
    await fs.writeFile(secretFile, unlockSecret, { mode: 0o600 })
    await fs.chmod(secretFile, 0o600)
    return { unlockSecret, created: !rotateSecret, rotated: rotateSecret }
  } catch (error) {
    throw new Error(`Could not write unlock secret file ${secretFile}: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function renderModule(envelope: JnoccioEncryptedGitCryptKeyEnvelope) {
  return `export type JnoccioEncryptedGitCryptKeyEnvelope = {
  version: 1
  kdf: "scrypt"
  cipher: "aes-256-gcm"
  salt: string
  iv: string
  tag: string
  ciphertext: string
  params: {
    N: number
    r: number
    p: number
    maxmem: number
  }
  aad: "jnoccio-fusion-git-crypt-key-v1"
}

export const JNOCCIO_ENCRYPTED_GIT_CRYPT_KEY = ${JSON.stringify(envelope, null, 2)} as const satisfies JnoccioEncryptedGitCryptKeyEnvelope
`
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const rawKey = await fs.readFile(args.keyFile)
  const { unlockSecret, created, rotated } = await readOrCreateSecret(args.secretFile, args.rotateSecret)
  const envelope = encryptJnoccioGitCryptKey(rawKey, unlockSecret, {
    aad: JNOCCIO_ENCRYPTION_AAD,
    params: JNOCCIO_ENCRYPTION_PARAMS,
  })

  await fs.mkdir(path.dirname(args.output), { recursive: true })
  await fs.writeFile(args.output, renderModule(envelope))

  process.stdout.write(
    [
      `encrypted module written: ${args.output}`,
      `unlock secret file: ${args.secretFile} (${unlockSecret.length} chars${created ? ", created" : ""}${rotated ? ", rotated" : ""})`,
      `unlock secret alphabet: A-Z0-9`,
      `git-crypt key source: ${args.keyFile}`,
    ].join("\n") + "\n",
  )
}

await main().catch((error) => {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
  process.exit(1)
})
