import path from "path"
import { WithInstance } from "@/project/with-instance"
import { ConfigPlugin } from "@/config/plugin"
import { TuiConfig } from "@/cli/cmd/tui/config/tui"
import { fail, type PluginLoad, type RuntimeState } from "./runtime-core"
import { activatePluginEntry } from "./runtime-api"
import { addExternalPluginEntries, resolveExternalPlugins } from "./runtime-load-external"
export { addExternalPluginEntries, resolveExternalPlugins } from "./runtime-load-external"
import { installPlugin as installModulePlugin, patchPluginConfig, readPluginManifest } from "@/plugin/install"
import { errorMessage } from "@/util/error"
import { Process } from "@/util/process"
import type { TuiPluginInstallResult } from "@jekko-ai/plugin/tui"

export function defaultPluginOrigin(state: RuntimeState, spec: string): ConfigPlugin.Origin {
  return {
    spec,
    scope: "local",
    source: state.api.state.path.config || path.join(state.directory, ".jekko", "tui.json"),
  }
}

export function installCause(err: unknown) {
  if (!err || typeof err !== "object") return
  if (!("cause" in err)) return
  return (err as { cause?: unknown }).cause
}

export function installDetail(err: unknown) {
  const hit = installCause(err) ?? err
  if (!(hit instanceof Process.RunFailedError)) {
    return {
      message: errorMessage(hit),
      missing: false,
    }
  }

  const lines = hit.stderr
    .toString()
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
  const errs = lines.filter((line) => line.startsWith("error:")).map((line) => line.replace(/^error:\s*/, ""))
  return {
    message: errs[0] ?? lines.at(-1) ?? errorMessage(hit),
    missing: lines.some((line) => line.includes("No version matching")),
  }
}

export async function addPluginBySpec(state: RuntimeState | undefined, raw: string) {
  if (!state) return false
  const spec = raw.trim()
  if (!spec) return false

  const cfg = state.pending.get(spec) ?? defaultPluginOrigin(state, spec)
  const next = ConfigPlugin.pluginSpecifier(cfg.spec)
  if (state.plugins.some((plugin) => plugin.load.spec === next)) {
    state.pending.delete(spec)
    return true
  }
  const ready = await WithInstance.provide({
    directory: state.directory,
    fn: () => resolveExternalPlugins([cfg], () => TuiConfig.waitForDependencies()),
  }).catch((error) => {
    fail("failed to add tui plugin", { path: next, error })
    return [] as PluginLoad[]
  })
  if (!ready.length) {
    return false
  }

  const first = ready[0]
  if (!first) {
    fail("failed to add tui plugin", { path: next })
    return false
  }
  if (state.plugins_by_id.has(first.id)) {
    state.pending.delete(spec)
    return true
  }

  const out = await addExternalPluginEntries(state, [first])
  let ok = out.ok && out.plugins.length > 0
  for (const plugin of out.plugins) {
    const active = await activatePluginEntry(state, plugin, false)
    if (!active) ok = false
  }

  if (ok) state.pending.delete(spec)
  if (!ok) {
    fail("failed to add tui plugin", { path: next })
  }
  return ok
}

export async function installPluginBySpec(
  state: RuntimeState | undefined,
  raw: string,
  global = false,
): Promise<TuiPluginInstallResult> {
  if (!state) {
    return {
      ok: false,
      message: "Plugin runtime is not ready.",
    }
  }

  const spec = raw.trim()
  if (!spec) {
    return {
      ok: false,
      message: "Plugin package name is required",
    }
  }

  const dir = state.api.state.path
  if (!dir.directory) {
    return {
      ok: false,
      message: "Paths are still syncing. Try again in a moment.",
    }
  }

  const install = await installModulePlugin(spec)
  if (!install.ok) {
    const out = installDetail(install.error)
    return {
      ok: false,
      message: out.message,
      missing: out.missing,
    }
  }

  const manifest = await readPluginManifest(install.target)
  if (!manifest.ok) {
    if (manifest.code === "manifest_no_targets") {
      return {
        ok: false,
        message: `"${spec}" does not expose plugin entrypoints or oc-themes in package.json`,
      }
    }

    return {
      ok: false,
      message: `Installed "${spec}" but failed to read ${manifest.file}`,
    }
  }

  const patch = await patchPluginConfig({
    spec,
    targets: manifest.targets,
    global,
    vcs: dir.worktree && dir.worktree !== "/" ? "git" : undefined,
    worktree: dir.worktree,
    directory: dir.directory,
  })
  if (!patch.ok) {
    if (patch.code === "invalid_json") {
      return {
        ok: false,
        message: `Invalid JSON in ${patch.file} (${patch.parse} at line ${patch.line}, column ${patch.col})`,
      }
    }

    return {
      ok: false,
      message: errorMessage(patch.error),
    }
  }

  const tui = manifest.targets.find((item) => item.kind === "tui")
  if (tui) {
    const file = patch.items.find((item) => item.kind === "tui")?.file
    const next = tui.opts ? ([spec, tui.opts] as ConfigPlugin.Spec) : spec
    state.pending.set(spec, {
      spec: next,
      scope: global ? "global" : "local",
      source: (file ?? dir.config) || path.join(patch.dir, "tui.json"),
    })
  }

  return {
    ok: true,
    dir: patch.dir,
    tui: Boolean(tui),
  }
}
