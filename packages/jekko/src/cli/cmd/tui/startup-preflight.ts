import fs from "fs"
import path from "path"
import os from "os"
import { MODEL_KEY_TEMPLATE_PATH, readModelKeyStatuses } from "@/model-setup/model-keys"
import { UI } from "@/cli/ui"
import { which } from "@/util/which"
import { jnoccioEnvExamplePath, repoRootFromSource } from "@/util/jnoccio-unlock"

type PreflightCheck = {
  label: string
  ok: boolean
  detail?: string
  fix?: string
}

function color(ok: boolean) {
  return ok ? UI.Style.TEXT_SUCCESS_BOLD : UI.Style.TEXT_WARNING_BOLD
}

function installCommand() {
  const brew = which("brew")
  if (process.platform === "darwin" && brew) return `${brew} install git-crypt`
  const aptGet = which("apt-get")
  if (aptGet) return `${aptGet} update && sudo ${aptGet} install git-crypt`
  const dnf = which("dnf")
  if (dnf) return `sudo ${dnf} install git-crypt`
  const pacman = which("pacman")
  if (pacman) return `sudo ${pacman} -S git-crypt`
  return "install git-crypt with your distro package manager"
}

function homeDeveloperKeyPath(homeDir = os.homedir()) {
  return path.join(homeDir, ".env.jnoccio")
}

function hasDeveloperKey(homeDir = os.homedir()) {
  if (Boolean(process.env.JNOCCIO_DEVELOPER_KEY?.trim())) return true
  const developerKeyPath = homeDeveloperKeyPath(homeDir)
  if (!fs.existsSync(developerKeyPath)) return false
  try {
    const raw = fs.readFileSync(developerKeyPath, "utf8")
    return /^JNOCCIO_DEVELOPER_KEY\s*=\s*(.+)$/m.test(raw)
  } catch {
    return false
  }
}

function sampleLines() {
  return [
    "OPENROUTER_API_KEY=fake-openrouter-key   # https://openrouter.ai/keys",
    "GITHUB_TOKEN=fake-github-token           # https://github.com/settings/tokens",
    "GEMINI_API_KEY=fake-gemini-key           # https://aistudio.google.com/apikey",
    "MISTRAL_API_KEY=fake-mistral-key         # https://console.mistral.ai/api-keys",
  ]
}

function printCheck(check: PreflightCheck) {
  const prefix = check.ok ? `${UI.Style.TEXT_SUCCESS_BOLD}[ok]${UI.Style.TEXT_NORMAL}` : `${UI.Style.TEXT_WARNING_BOLD}[check]${UI.Style.TEXT_NORMAL}`
  UI.println(color(check.ok) + `${prefix} ${check.label}` + UI.Style.TEXT_NORMAL)
  if (check.detail) UI.println(UI.Style.TEXT_DIM + `      ${check.detail}` + UI.Style.TEXT_NORMAL)
  if (check.fix) UI.println(UI.Style.TEXT_INFO_BOLD + `      ${check.fix}` + UI.Style.TEXT_NORMAL)
}

export async function runStartupPreflight(repoRoot = repoRootFromSource(), homeDir = os.homedir()) {
  const envPath = homeDeveloperKeyPath(homeDir)
  const envExamplePath = jnoccioEnvExamplePath(repoRoot)
  const modelKeys = await readModelKeyStatuses()
  const configuredModelKeys = modelKeys.statuses.filter((status) => status.configured)
  const modelKeysConfigured = configuredModelKeys.length > 0
  const developerKeyPresent = hasDeveloperKey(homeDir)
  const gitCrypt = developerKeyPresent ? which("git-crypt") : null
  const checks: PreflightCheck[] = [
    {
      label: "Jnoccio key file",
      ok: fs.existsSync(envPath),
      detail: fs.existsSync(envPath) ? envPath : `missing: ${envPath}`,
      fix: fs.existsSync(envPath)
        ? undefined
        : `copy the template: cp "${envExamplePath}" "${envPath}"`,
    },
    {
      label: "Model keys",
      ok: modelKeysConfigured,
      detail: modelKeysConfigured
        ? `${configuredModelKeys.length} configured at ${MODEL_KEY_TEMPLATE_PATH}`
        : `no configured providers in ${MODEL_KEY_TEMPLATE_PATH}`,
      fix: modelKeysConfigured
        ? undefined
        : `put provider keys in ${MODEL_KEY_TEMPLATE_PATH}`,
    },
  ]
  if (developerKeyPresent) {
    checks.push({
      label: "git-crypt",
      ok: Boolean(gitCrypt),
      detail: gitCrypt ? gitCrypt : "missing from PATH",
      fix: gitCrypt ? undefined : installCommand(),
    })
  }

  UI.println(UI.Style.TEXT_INFO_BOLD + "Jekko startup preflight" + UI.Style.TEXT_NORMAL)
  UI.println(UI.Style.TEXT_DIM + `repo: ${repoRoot}` + UI.Style.TEXT_NORMAL)
  UI.println(UI.Style.TEXT_DIM + `env:  ${envPath}` + UI.Style.TEXT_NORMAL)
  UI.println(UI.Style.TEXT_DIM + `tmpl: ${envExamplePath}` + UI.Style.TEXT_NORMAL)
  UI.println(UI.Style.TEXT_DIM + `keys: ${MODEL_KEY_TEMPLATE_PATH}` + UI.Style.TEXT_NORMAL)
  UI.println("")

  for (const check of checks) {
    printCheck(check)
  }

  const missingKeyFile = !checks[0]?.ok
  const missingModelKeys = !modelKeysConfigured
  const gitCryptCheck = checks.find((check) => check.label === "git-crypt")
  const missingGitCrypt = developerKeyPresent && gitCryptCheck ? !gitCryptCheck.ok : false

  if (missingKeyFile || missingModelKeys) {
    UI.println("")
    if (missingKeyFile) {
      UI.println(UI.Style.TEXT_WARNING_BOLD + "Jnoccio key file is missing." + UI.Style.TEXT_NORMAL)
    }
    if (missingModelKeys) {
      UI.println(UI.Style.TEXT_WARNING_BOLD + "No model keys found." + UI.Style.TEXT_NORMAL)
    }
    UI.println(UI.Style.TEXT_DIM + "Example entries:" + UI.Style.TEXT_NORMAL)
    for (const line of sampleLines()) {
      UI.println(UI.Style.TEXT_DIM + `  ${line}` + UI.Style.TEXT_NORMAL)
    }
    UI.println(UI.Style.TEXT_DIM + `Get real keys from the provider URLs in ${path.relative(repoRoot, envExamplePath)}` + UI.Style.TEXT_NORMAL)
    UI.println(UI.Style.TEXT_DIM + `Model keys live in ${MODEL_KEY_TEMPLATE_PATH}` + UI.Style.TEXT_NORMAL)
    if (missingModelKeys) {
      UI.println(UI.Style.TEXT_DIM + "Run `jekko keys init` to create the canonical model-key file." + UI.Style.TEXT_NORMAL)
    }
  }

  if (developerKeyPresent && missingGitCrypt) {
    UI.println("")
    UI.println(UI.Style.TEXT_WARNING_BOLD + "git-crypt is required to unlock the Jnoccio key file." + UI.Style.TEXT_NORMAL)
    UI.println(UI.Style.TEXT_DIM + `Install command: ${installCommand()}` + UI.Style.TEXT_NORMAL)
  }

  UI.println("")
  return !(missingKeyFile || missingModelKeys || missingGitCrypt)
}
