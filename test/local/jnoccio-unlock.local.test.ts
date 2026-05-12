import { test, expect, describe } from "bun:test"
import { $ } from "bun"
import fs from "fs"
import path from "path"
import os from "os"

describe("Jnoccio Fusion Local Unlock Proof", () => {
  test("clones, unlocks, and verifies jnoccio-fusion", async () => {
    // 1. Skip in CI, without env var, or without git-crypt
    if (process.env.CI) {
      console.log("Skipping local unlock test in CI")
      return
    }

    const keyPath = process.env.JNOCCIO_GIT_CRYPT_KEY_PATH
    if (!keyPath) {
      console.log("Skipping local unlock test: JNOCCIO_GIT_CRYPT_KEY_PATH not set")
      return
    }

    if (!fs.existsSync(keyPath)) {
      throw new Error(`Key file not found at provided JNOCCIO_GIT_CRYPT_KEY_PATH: ${keyPath}`)
    }

    const hasGitCrypt = await $`which git-crypt`.quiet().nothrow()
    if (hasGitCrypt.exitCode !== 0) {
      console.log("Skipping local unlock test: git-crypt not installed")
      return
    }

    // 2. Create scratch directory
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "jnoccio-unlock-test-"))
    try {
      // 3. Clone fresh scratch copy
      const repoRoot = path.resolve(import.meta.dir, "../../")
      await $`git clone ${repoRoot} ${tempDir}`.quiet()

      // 4. Unlock with key file
      await $`git-crypt unlock ${keyPath}`.cwd(tempDir).quiet()

      // 5. Verify Jnoccio files parse (KEYS.md should now be readable)
      const keysContent = fs.readFileSync(path.join(tempDir, "jnoccio-fusion/KEYS.md"), "utf-8")
      expect(keysContent).toContain("OPENROUTER_API_KEY")

      // 6. Write the minimal .env.jnoccio needed for the encryption check
      const envPath = path.join(tempDir, "jnoccio-fusion/.env.jnoccio")
      fs.writeFileSync(envPath, "OPENROUTER_API_KEY=test_value")

      // 7. Run index encryption checker
      const checkRes = await $`tools/check-encrypted-paths.sh --index`.cwd(tempDir).nothrow().quiet()
      
      // If the checker fails, print output for debugging
      if (checkRes.exitCode !== 0) {
        console.error(checkRes.stderr.toString())
        console.error(checkRes.stdout.toString())
      }
      expect(checkRes.exitCode).toBe(0)
    } finally {
      // 8. Clean up
      fs.rmSync(tempDir, { recursive: true, force: true })
    }
  })
})
