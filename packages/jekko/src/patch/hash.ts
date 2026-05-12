import { createHash } from "crypto"

export function hashBytes(bytes: Buffer | Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex")
}

export async function hashFile(filePath: string): Promise<string> {
  const file = Bun.file(filePath)
  const bytes = await file.arrayBuffer()
  return hashBytes(Buffer.from(bytes))
}

export function hashString(text: string): string {
  return createHash("sha256").update(text).digest("hex")
}
