import type { ChildProcessWithoutNullStreams } from "child_process"
import path from "path"
import os from "os"
import { Global } from "@jekko-ai/core/global"
import * as Log from "@jekko-ai/core/util/log"
import { text } from "node:stream/consumers"
import fs from "fs/promises"
import { Filesystem } from "@/util/filesystem"
import type { InstanceContext } from "../project/instance"
import { Flag } from "@jekko-ai/core/flag/flag"
import { Archive } from "@/util/archive"
import { Process } from "@/util/process"
import { which } from "../util/which"
import { Module } from "@jekko-ai/core/util/module"
import { spawn } from "./launch"
import { Npm } from "@jekko-ai/core/npm"
import z from "zod"
import { parseJavaMajorVersion } from "./java-version"
import type { Handle, Info } from "./server-shared"
import { NearestRoot, pathExists, run, output, ZlsReleaseSchema, TerraformReleaseSchema, ReleaseSchema, log } from "./server-shared"


export const CSharp: Info = {
  id: "csharp",
  root: NearestRoot([".slnx", ".sln", ".csproj", "global.json"]),
  extensions: [".cs", ".csx"],
  async spawn(root) {
    const bin = await getRoslynLanguageServer()
    if (!bin) return

    return {
      process: spawn(bin, ["--stdio", "--autoLoadProjects"], {
        cwd: root,
      }),
    }
  },
}


export const Razor: Info = {
  id: "razor",
  root: NearestRoot([".slnx", ".sln", ".csproj", "global.json"]),
  extensions: [".razor", ".cshtml"],
  async spawn(root) {
    const bin = await getRoslynLanguageServer()
    if (!bin) return

    const razor = await findVscodeRazorExtension()
    if (!razor) {
      log.info("VS Code C# extension with Razor support not found, skipping Razor LSP")
      return
    }

    log.info("using VS Code Razor extension for roslyn-language-server", { extension: razor.extension })
    return {
      process: spawn(
        bin,
        [
          "--stdio",
          "--autoLoadProjects",
          `--razorSourceGenerator=${razor.compiler}`,
          `--razorDesignTimePath=${razor.targets}`,
          "--extension",
          razor.extension,
        ],
        {
          cwd: root,
        },
      ),
    }
  },
}

let roslynLanguageServerInstall: Promise<string | undefined> | undefined

async function getRoslynLanguageServer() {
  const existing = which("roslyn-language-server")
  if (existing) return existing

  const global = await roslynLanguageServerGlobalPath()
  if (global) return global

  roslynLanguageServerInstall ||= installRoslynLanguageServer().finally(() => {
    roslynLanguageServerInstall = undefined
  })
  return roslynLanguageServerInstall
}

async function installRoslynLanguageServer() {
  if (!which("dotnet")) {
    log.error(".NET SDK is required to install roslyn-language-server")
    return
  }

  if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
  log.info("installing roslyn-language-server via dotnet tool")
  const proc = Process.spawn(["dotnet", "tool", "install", "--global", "roslyn-language-server", "--prerelease"], {
    stdout: "pipe",
    stderr: "pipe",
    stdin: "pipe",
  })
  const exit = await proc.exited
  if (exit !== 0) {
    log.error("Failed to install roslyn-language-server")
    return
  }

  const resolved = which("roslyn-language-server")
  if (resolved) {
    log.info(`installed roslyn-language-server`, { bin: resolved })
    return resolved
  }

  const global = await roslynLanguageServerGlobalPath()
  if (global) {
    log.info(`installed roslyn-language-server`, { bin: global })
    return global
  }

  log.error("Installed roslyn-language-server but could not resolve executable")
}

async function roslynLanguageServerGlobalPath() {
  const bin = path.join(
    process.env.DOTNET_CLI_HOME ?? os.homedir(),
    ".dotnet",
    "tools",
    "roslyn-language-server" + (process.platform === "win32" ? ".cmd" : ""),
  )
  return (await pathExists(bin)) ? bin : undefined
}

async function findVscodeRazorExtension() {
  const roots = [
    process.env.VSCODE_EXTENSIONS,
    path.join(os.homedir(), ".vscode", "extensions"),
    path.join(os.homedir(), ".vscode-insiders", "extensions"),
    path.join(os.homedir(), ".vscode-server", "extensions"),
    path.join(os.homedir(), ".vscode-server-insiders", "extensions"),
  ].filter((item) => item !== undefined)

  for (const root of [...new Set(roots)]) {
    const entries = await fs.readdir(root, { withFileTypes: true }).catch(() => [])
    const candidates = await Promise.all(
      entries
        .filter((entry) => entry.isDirectory() && entry.name.startsWith("ms-dotnettools.csharp-"))
        .map(async (entry) => ({
          path: path.join(root, entry.name, ".razorExtension"),
          modified: (await fs.stat(path.join(root, entry.name)).catch(() => undefined))?.mtimeMs ?? 0,
        })),
    )
    for (const entry of candidates.sort((a, b) => b.modified - a.modified).map((candidate) => candidate.path)) {
      const result = {
        compiler: path.join(entry, "Microsoft.CodeAnalysis.Razor.Compiler.dll"),
        targets: path.join(entry, "Targets", "Microsoft.NET.Sdk.Razor.DesignTime.targets"),
        extension: path.join(entry, "Microsoft.VisualStudioCode.RazorExtension.dll"),
      }
      if (
        (await pathExists(result.compiler)) &&
        (await pathExists(result.targets)) &&
        (await pathExists(result.extension))
      ) {
        return result
      }
    }
  }
}


export const FSharp: Info = {
  id: "fsharp",
  root: NearestRoot([".slnx", ".sln", ".fsproj", "global.json"]),
  extensions: [".fs", ".fsi", ".fsx", ".fsscript"],
  async spawn(root) {
    let bin = which("fsautocomplete")
    if (!bin) {
      if (!which("dotnet")) {
        log.error(".NET SDK is required to install fsautocomplete")
        return
      }

      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      log.info("installing fsautocomplete via dotnet tool")
      const proc = Process.spawn(["dotnet", "tool", "install", "fsautocomplete", "--tool-path", Global.Path.bin], {
        stdout: "pipe",
        stderr: "pipe",
        stdin: "pipe",
      })
      const exit = await proc.exited
      if (exit !== 0) {
        log.error("Failed to install fsautocomplete")
        return
      }

      bin = path.join(Global.Path.bin, "fsautocomplete" + (process.platform === "win32" ? ".exe" : ""))
      log.info(`installed fsautocomplete`, { bin })
    }

    return {
      process: spawn(bin, {
        cwd: root,
      }),
    }
  },
}


export const SourceKit: Info = {
  id: "sourcekit-lsp",
  extensions: [".swift", ".objc", "objcpp"],
  root: NearestRoot(["Package.swift", "*.xcodeproj", "*.xcworkspace"]),
  async spawn(root) {
    // Check if sourcekit-lsp is available in the PATH
    // This is installed with the Swift toolchain
    const sourcekit = which("sourcekit-lsp")
    if (sourcekit) {
      return {
        process: spawn(sourcekit, {
          cwd: root,
        }),
      }
    }

    // If sourcekit-lsp not found, check if xcrun is available
    // This is specific to macOS where sourcekit-lsp is typically installed with Xcode
    if (!which("xcrun")) return

    const lspLoc = await output(["xcrun", "--find", "sourcekit-lsp"])

    if (lspLoc.code !== 0) return

    const bin = lspLoc.text.trim()

    return {
      process: spawn(bin, {
        cwd: root,
      }),
    }
  },
}


export const RustAnalyzer: Info = {
  id: "rust",
  root: async (file, ctx) => {
    const crateRoot = await NearestRoot(["Cargo.toml", "Cargo.lock"])(file, ctx)
    if (crateRoot === undefined) {
      // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
      return undefined
    }
    let currentDir = crateRoot

    while (currentDir !== path.dirname(currentDir)) {
      // Stop at filesystem root
      const cargoTomlPath = path.join(currentDir, "Cargo.toml")
      try {
        const cargoTomlContent = await Filesystem.readText(cargoTomlPath)
        if (cargoTomlContent.includes("[workspace]")) {
          return currentDir
        }
      } catch {
        // File doesn't exist or can't be read, continue searching up
      }

      const parentDir = path.dirname(currentDir)
      if (parentDir === currentDir) break // Reached filesystem root
      currentDir = parentDir

      // Stop if we've gone above the app root
      if (!currentDir.startsWith(ctx.worktree)) break
    }

    return crateRoot
  },
  extensions: [".rs"],
  async spawn(root) {
    const bin = which("rust-analyzer")
    if (!bin) {
      log.info("rust-analyzer not found in path, please install it")
      return
    }
    return {
      process: spawn(bin, {
        cwd: root,
      }),
    }
  },
}


export const Clangd: Info = {
  id: "clangd",
  root: NearestRoot(["compile_commands.json", "compile_flags.txt", ".clangd"]),
  extensions: [".c", ".cpp", ".cc", ".cxx", ".c++", ".h", ".hpp", ".hh", ".hxx", ".h++"],
  async spawn(root) {
    const args = ["--background-index", "--clang-tidy"]
    const fromPath = which("clangd")
    if (fromPath) {
      return {
        process: spawn(fromPath, args, {
          cwd: root,
        }),
      }
    }

    const ext = process.platform === "win32" ? ".exe" : ""
    const direct = path.join(Global.Path.bin, "clangd" + ext)
    if (await Filesystem.exists(direct)) {
      return {
        process: spawn(direct, args, {
          cwd: root,
        }),
      }
    }

    const entries = await fs.readdir(Global.Path.bin, { withFileTypes: true }).catch(() => [])
    for (const entry of entries) {
      if (!entry.isDirectory()) continue
      if (!entry.name.startsWith("clangd_")) continue
      const candidate = path.join(Global.Path.bin, entry.name, "bin", "clangd" + ext)
      if (await Filesystem.exists(candidate)) {
        return {
          process: spawn(candidate, args, {
            cwd: root,
          }),
        }
      }
    }

    if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
    log.info("downloading clangd from GitHub releases")

    const releaseResponse = await fetch("https://api.github.com/repos/clangd/clangd/releases/latest")
    if (!releaseResponse.ok) {
      log.error("Failed to fetch clangd release info")
      return
    }

    const release: {
      tag_name?: string
      assets?: { name?: string; browser_download_url?: string }[]
    } = await releaseResponse.json()

    const tag = release.tag_name
    if (!tag) {
      log.error("clangd release did not include a tag name")
      return
    }
    const platform = process.platform
    const tokens: Record<string, string> = {
      darwin: "mac",
      linux: "linux",
      win32: "windows",
    }
    const token = tokens[platform]
    if (!token) {
      log.error(`Platform ${platform} is not supported by clangd auto-download`)
      return
    }

    const assets = release.assets ?? []
    const valid = (item: { name?: string; browser_download_url?: string }) => {
      if (!item.name) return false
      if (!item.browser_download_url) return false
      if (!item.name.includes(token)) return false
      return item.name.includes(tag)
    }

    const asset =
      assets.find((item) => valid(item) && item.name?.endsWith(".zip")) ??
      assets.find((item) => valid(item) && item.name?.endsWith(".tar.xz")) ??
      assets.find((item) => valid(item))
    if (!asset?.name || !asset.browser_download_url) {
      log.error("clangd could not match release asset", { tag, platform })
      return
    }

    const name = asset.name
    const downloadResponse = await fetch(asset.browser_download_url)
    if (!downloadResponse.ok) {
      log.error("Failed to download clangd")
      return
    }

    const archive = path.join(Global.Path.bin, name)
    const buf = await downloadResponse.arrayBuffer()
    if (buf.byteLength === 0) {
      log.error("Failed to write clangd archive")
      return
    }
    await Filesystem.write(archive, Buffer.from(buf))

    const zip = name.endsWith(".zip")
    const tar = name.endsWith(".tar.xz")
    if (!zip && !tar) {
      log.error("clangd encountered unsupported asset", { asset: name })
      return
    }

    if (zip) {
      const ok = await Archive.extractZip(archive, Global.Path.bin)
        .then(() => true)
        .catch((error) => {
          log.error("Failed to extract clangd archive", { error })
          return false
        })
      if (!ok) return
    }
    if (tar) {
      await run(["tar", "-xf", archive], { cwd: Global.Path.bin })
    }
    await fs.rm(archive, { force: true })

    const bin = path.join(Global.Path.bin, "clangd_" + tag, "bin", "clangd" + ext)
    if (!(await Filesystem.exists(bin))) {
      log.error("Failed to extract clangd binary")
      return
    }

    if (platform !== "win32") {
      await fs.chmod(bin, 0o755).catch(() => {})
    }

    await fs.unlink(path.join(Global.Path.bin, "clangd")).catch(() => {})
    await fs.symlink(bin, path.join(Global.Path.bin, "clangd")).catch(() => {})

    log.info(`installed clangd`, { bin })

    return {
      process: spawn(bin, args, {
        cwd: root,
      }),
    }
  },
}
