import fs from "fs"
import path from "path"
import { Effect } from "effect"
import { effectCmd, fail } from "../../effect-cmd"
import { UI } from "../../ui"
import { detectCanonical, hasJankuraiCiWorkflow } from "./detect"
import { auditPolicy } from "./validate-policy"
import { runJankuraiUpdate } from "./update"

const POLICY_TEMPLATE_PATH = path.join(__dirname, "templates", "audit-policy.defaults.toml")
const CI_TEMPLATE_PATH = path.join(__dirname, "templates", "jankurai.yml.txt")

type BootstrapArgs = {
  yes?: boolean
  strict?: boolean
  "dry-run"?: boolean
  "skip-update"?: boolean
  "skip-ci"?: boolean
  directory?: string
}

export type BootstrapReceipt = {
  ts: number
  cwd: string
  jankurai_installed: boolean
  jankurai_update: { exitCode: number | null; stderr: string }
  missing_required: string[]
  missing_optional: string[]
  ci_workflow_present: boolean
  ci_workflow_added: boolean
  policy_path: string
  policy_added: boolean
  policy_ok: boolean
  notes: string[]
  ok: boolean
}

export const JankuraiBootstrapCommand = effectCmd<BootstrapArgs, BootstrapReceipt>({
  command: ["bootstrap", "init"],
  describe: "set up or repair jankurai canonical files + audit policy in the current repo",
  instance: false,
  builder: (yargs) =>
    yargs
      // jankurai:allow HLT-027-HUMAN-REVIEW-EVIDENCE-GAP reason=bootstrap-writes-structured-receipt-to-agent-zyal-bootstrap-json expires=2027-01-01
      .option("yes", { type: "boolean", describe: "auto-confirm scaffolding prompts without interactive input" })
      .option("strict", { type: "boolean", describe: "fail instead of patching missing canonical files" })
      .option("dry-run", { type: "boolean", describe: "report intended actions without writing" })
      .option("skip-update", { type: "boolean", describe: "skip the `jankurai update --client-start --quiet` step" })
      .option("skip-ci", { type: "boolean", describe: "do not scaffold .github/workflows/jankurai.yml" })
      .option("directory", { type: "string", describe: "repo root (default: cwd)" }),
  handler: Effect.fn("Cli.jankurai.bootstrap")(function* (args) {
    const cwd = path.resolve(args.directory ?? process.cwd())
    const dryRun = Boolean(args["dry-run"])
    const strict = Boolean(args.strict)
    const yes = Boolean(args.yes)

    UI.println(`jankurai bootstrap: ${cwd}${dryRun ? " (dry-run)" : ""}`)

    // 1. Update jankurai itself, non-fatally.
    let updateOutcome = { exitCode: null as number | null, stderr: "" }
    let installed = false
    if (!args["skip-update"]) {
      const result = runJankuraiUpdate({ dryRun })
      installed = result.installed
      updateOutcome = { exitCode: result.exitCode, stderr: result.stderr }
      if (!installed) {
        UI.println("jankurai not on PATH — skipping update. Install with: cargo install --git https://github.com/neverhuman/jankurai --tag v1.4.2 --locked jankurai")
      } else if (result.exitCode !== 0) {
        UI.println(`jankurai update exited ${result.exitCode}. stderr: ${result.stderr.trim()}`)
      }
    }

    // 2. Detect canonical files.
    const detection = detectCanonical(cwd)
    if (detection.missingRequired.length > 0) {
      UI.println(`missing required canonical files: ${detection.missingRequired.join(", ")}`)
      if (strict) {
        return yield* fail(`strict mode: ${detection.missingRequired.length} required files missing`, 3)
      }
      if (!yes && !dryRun) {
        UI.println("Re-run with --yes to scaffold defaults, or run `jankurai init` directly.")
      }
    }

    // 3. Scaffold audit-policy.toml if missing (we only scaffold this file
    //    here — the rest of the canonical surface needs `jankurai init`).
    const policyPath = path.join(cwd, "agent", "audit-policy.toml")
    let policyAdded = false
    if (!fs.existsSync(policyPath)) {
      if (yes && !dryRun) {
        const template = fs.readFileSync(POLICY_TEMPLATE_PATH, "utf8")
        fs.mkdirSync(path.dirname(policyPath), { recursive: true })
        fs.writeFileSync(policyPath, template, "utf8")
        policyAdded = true
        UI.println(`wrote ${path.relative(cwd, policyPath)}`)
      } else if (dryRun) {
        UI.println(`(dry-run) would write ${path.relative(cwd, policyPath)}`)
      }
    }

    // 4. Validate audit-policy.toml content.
    const policy = auditPolicy(policyPath)
    if (!policy.ok && fs.existsSync(policyPath)) {
      const reasons = [
        policy.hasMinScore ? null : "missing min_score",
        policy.missingFailOn.length ? `fail_on missing ${policy.missingFailOn.join(",")}` : null,
        policy.missingAdvisoryOn.length ? `advisory_on missing ${policy.missingAdvisoryOn.join(",")}` : null,
      ]
        .filter(Boolean)
        .join("; ")
      UI.println(`audit-policy.toml needs attention: ${reasons}`)
      if (strict) {
        return yield* fail(`strict mode: policy invalid (${reasons})`, 3)
      }
    }

    // 5. Scaffold CI workflow if missing and not explicitly skipped.
    const ciPath = path.join(cwd, ".github", "workflows", "jankurai.yml")
    const ciPresent = hasJankuraiCiWorkflow(cwd)
    let ciAdded = false
    if (!ciPresent && !args["skip-ci"]) {
      if (yes && !dryRun) {
        const template = fs.readFileSync(CI_TEMPLATE_PATH, "utf8")
        fs.mkdirSync(path.dirname(ciPath), { recursive: true })
        fs.writeFileSync(ciPath, template, "utf8")
        ciAdded = true
        UI.println(`wrote ${path.relative(cwd, ciPath)}`)
      } else if (dryRun) {
        UI.println(`(dry-run) would write ${path.relative(cwd, ciPath)}`)
      }
    }

    // 6. Write receipt.
    const receipt: BootstrapReceipt = {
      ts: Math.floor(Date.now() / 1000),
      cwd,
      jankurai_installed: installed,
      jankurai_update: updateOutcome,
      missing_required: detection.missingRequired,
      missing_optional: detection.missingOptional,
      ci_workflow_present: ciPresent || ciAdded,
      ci_workflow_added: ciAdded,
      policy_path: policyPath,
      policy_added: policyAdded,
      policy_ok: policy.ok,
      notes: [],
      ok:
        detection.missingRequired.length === 0 &&
        (policy.ok || policyAdded) &&
        (ciPresent || ciAdded || Boolean(args["skip-ci"])),
    }
    const receiptDir = path.join(cwd, "agent", "zyal")
    const receiptPath = path.join(receiptDir, "bootstrap.json")
    if (!dryRun) {
      fs.mkdirSync(receiptDir, { recursive: true })
      fs.writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + "\n", "utf8")
    }
    UI.println(`bootstrap ${receipt.ok ? "OK" : "INCOMPLETE"} — receipt at ${path.relative(cwd, receiptPath)}`)
    return receipt
  }),
})
