import fs from "fs"
import os from "os"
import path from "path"
import { fileURLToPath } from "url"

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export const JNOCCIO_FUSION_BUNDLE_NAME = "jnoccio-fusion"
export const JNOCCIO_FUSION_BUNDLE_FILES = {
  server: "server.jsonc",
  models: "models.json",
}

export function resolveJekkoConfigRoot(env = process.env) {
  const override = env.JEKKO_CONFIG_DIR?.trim()
  if (override) return override

  const home = env.HOME?.trim() || env.USERPROFILE?.trim() || os.homedir()
  if (env.XDG_CONFIG_HOME?.trim()) {
    return path.join(env.XDG_CONFIG_HOME.trim(), "jekko")
  }

  if (process.platform === "win32") {
    const appData = env.APPDATA?.trim() || env.LOCALAPPDATA?.trim() || path.join(home, "AppData", "Roaming")
    return path.join(appData, "jekko")
  }

  return path.join(home, ".config", "jekko")
}

export function jnoccioFusionBundlePaths(configRoot = resolveJekkoConfigRoot()) {
  const bundleDir = path.join(configRoot, JNOCCIO_FUSION_BUNDLE_NAME)
  return {
    configRoot,
    bundleDir,
    serverPath: path.join(bundleDir, JNOCCIO_FUSION_BUNDLE_FILES.server),
    modelsPath: path.join(bundleDir, JNOCCIO_FUSION_BUNDLE_FILES.models),
  }
}

function copyIfMissing(source, target) {
  if (!fs.existsSync(source)) {
    throw new Error(`Missing seed file: ${source}`)
  }
  if (fs.existsSync(target)) return false
  fs.mkdirSync(path.dirname(target), { recursive: true })
  fs.copyFileSync(source, target)
  return true
}

export function seedJnoccioFusionBundle(options = {}) {
  const configRoot = options.configRoot ?? resolveJekkoConfigRoot()
  const seedRoot = options.seedRoot ?? path.join(__dirname, "seed", JNOCCIO_FUSION_BUNDLE_NAME)
  const paths = jnoccioFusionBundlePaths(configRoot)

  const serverCreated = copyIfMissing(path.join(seedRoot, JNOCCIO_FUSION_BUNDLE_FILES.server), paths.serverPath)
  const modelsCreated = copyIfMissing(path.join(seedRoot, JNOCCIO_FUSION_BUNDLE_FILES.models), paths.modelsPath)

  return {
    ...paths,
    seedRoot,
    serverCreated,
    modelsCreated,
    bundleCreated: serverCreated || modelsCreated,
  }
}
