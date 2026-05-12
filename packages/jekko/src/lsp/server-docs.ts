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


export const TexLab: Info = {
  id: "texlab",
  extensions: [".tex", ".bib"],
  root: NearestRoot([".latexmkrc", "latexmkrc", ".texlabroot", "texlabroot"]),
  async spawn(root) {
    let bin = which("texlab")

    if (!bin) {
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      log.info("downloading texlab from GitHub releases")

      const response = await fetch("https://api.github.com/repos/latex-lsp/texlab/releases/latest")
      if (!response.ok) {
        log.error("Failed to fetch texlab release info")
        return
      }

      const releaseResult = ReleaseSchema.safeParse(await response.json())
      if (!releaseResult.success) {
        log.error("Failed to parse texlab release info")
        return
      }
      const release = releaseResult.data
      const version = release.tag_name?.replace("v", "")
      if (!version) {
        log.error("texlab release did not include a version tag")
        return
      }

      const platform = process.platform
      const arch = process.arch

      const texArch = arch === "arm64" ? "aarch64" : "x86_64"
      const texPlatform = platform === "darwin" ? "macos" : platform === "win32" ? "windows" : "linux"
      const ext = platform === "win32" ? "zip" : "tar.gz"
      const assetName = `texlab-${texArch}-${texPlatform}.${ext}`

      const assets = release.assets ?? []
      const asset = assets.find((a) => a.name === assetName)
      if (!asset?.browser_download_url) {
        log.error(`Could not find asset ${assetName} in texlab release`)
        return
      }

      const downloadResponse = await fetch(asset.browser_download_url)
      if (!downloadResponse.ok) {
        log.error("Failed to download texlab")
        return
      }

      const tempPath = path.join(Global.Path.bin, assetName)
      if (downloadResponse.body) await Filesystem.writeStream(tempPath, downloadResponse.body)

      if (ext === "zip") {
        const ok = await Archive.extractZip(tempPath, Global.Path.bin)
          .then(() => true)
          .catch((error) => {
            log.error("Failed to extract texlab archive", { error })
            return false
          })
        if (!ok) return
      }
      if (ext === "tar.gz") {
        await run(["tar", "-xzf", tempPath], { cwd: Global.Path.bin })
      }

      await fs.rm(tempPath, { force: true })

      bin = path.join(Global.Path.bin, "texlab" + (platform === "win32" ? ".exe" : ""))

      if (!(await Filesystem.exists(bin))) {
        log.error("Failed to extract texlab binary")
        return
      }

      if (platform !== "win32") {
        await fs.chmod(bin, 0o755).catch(() => {})
      }

      log.info("installed texlab", { bin })
    }

    return {
      process: spawn(bin, {
        cwd: root,
      }),
    }
  },
}


export const DockerfileLS: Info = {
  id: "dockerfile",
  extensions: [".dockerfile", "Dockerfile"],
  root: async (_file, ctx) => ctx.directory,
  async spawn(root) {
    let binary = which("docker-langserver")
    const args: string[] = []
    if (!binary) {
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      const resolved = await Npm.which("dockerfile-language-server-nodejs")
      if (!resolved) return
      binary = resolved
    }
    args.push("--stdio")
    const proc = spawn(binary, args, {
      cwd: root,
      env: {
        ...process.env,
      },
    })
    return {
      process: proc,
    }
  },
}


export const Gleam: Info = {
  id: "gleam",
  extensions: [".gleam"],
  root: NearestRoot(["gleam.toml"]),
  async spawn(root) {
    const gleam = which("gleam")
    if (!gleam) {
      log.info("gleam not found, please install gleam first")
      return
    }
    return {
      process: spawn(gleam, ["lsp"], {
        cwd: root,
      }),
    }
  },
}


export const Clojure: Info = {
  id: "clojure-lsp",
  extensions: [".clj", ".cljs", ".cljc", ".edn"],
  root: NearestRoot(["deps.edn", "project.clj", "shadow-cljs.edn", "bb.edn", "build.boot"]),
  async spawn(root) {
    let bin = which("clojure-lsp")
    if (!bin && process.platform === "win32") {
      bin = which("clojure-lsp.exe")
    }
    if (!bin) {
      log.info("clojure-lsp not found, please install clojure-lsp first")
      return
    }
    return {
      process: spawn(bin, ["listen"], {
        cwd: root,
      }),
    }
  },
}


export const Nixd: Info = {
  id: "nixd",
  extensions: [".nix"],
  root: async (file, ctx) => {
    // First, look for flake.nix - the most reliable Nix project root indicator
    const flakeRoot = await NearestRoot(["flake.nix"])(file, ctx)
    if (flakeRoot && flakeRoot !== ctx.directory) return flakeRoot

    // If no flake.nix, fall back to git repository root
    if (ctx.worktree && ctx.worktree !== ctx.directory) return ctx.worktree

    // Finally, use the instance directory as alternative_path
    return ctx.directory
  },
  async spawn(root) {
    const nixd = which("nixd")
    if (!nixd) {
      log.info("nixd not found, please install nixd first")
      return
    }
    return {
      process: spawn(nixd, [], {
        cwd: root,
        env: {
          ...process.env,
        },
      }),
    }
  },
}


export const Tinymist: Info = {
  id: "tinymist",
  extensions: [".typ", ".typc"],
  root: NearestRoot(["typst.toml"]),
  async spawn(root) {
    let bin = which("tinymist")

    if (!bin) {
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      log.info("downloading tinymist from GitHub releases")

      const response = await fetch("https://api.github.com/repos/Myriad-Dreamin/tinymist/releases/latest")
      if (!response.ok) {
        log.error("Failed to fetch tinymist release info")
        return
      }

      const releaseResult = ReleaseSchema.safeParse(await response.json())
      if (!releaseResult.success) {
        log.error("Failed to parse tinymist release info")
        return
      }
      const release = releaseResult.data

      const platform = process.platform
      const arch = process.arch

      const tinymistArch = arch === "arm64" ? "aarch64" : "x86_64"
      let tinymistPlatform: string
      let ext: string

      if (platform === "darwin") {
        tinymistPlatform = "apple-darwin"
        ext = "tar.gz"
      } else if (platform === "win32") {
        tinymistPlatform = "pc-windows-msvc"
        ext = "zip"
      } else {
        tinymistPlatform = "unknown-linux-gnu"
        ext = "tar.gz"
      }

      const assetName = `tinymist-${tinymistArch}-${tinymistPlatform}.${ext}`

      const assets = release.assets ?? []
      const asset = assets.find((a) => a.name === assetName)
      if (!asset?.browser_download_url) {
        log.error(`Could not find asset ${assetName} in tinymist release`)
        return
      }

      const downloadResponse = await fetch(asset.browser_download_url)
      if (!downloadResponse.ok) {
        log.error("Failed to download tinymist")
        return
      }

      const tempPath = path.join(Global.Path.bin, assetName)
      if (downloadResponse.body) await Filesystem.writeStream(tempPath, downloadResponse.body)

      if (ext === "zip") {
        const ok = await Archive.extractZip(tempPath, Global.Path.bin)
          .then(() => true)
          .catch((error) => {
            log.error("Failed to extract tinymist archive", { error })
            return false
          })
        if (!ok) return
      } else {
        await run(["tar", "-xzf", tempPath, "--strip-components=1"], { cwd: Global.Path.bin })
      }

      await fs.rm(tempPath, { force: true })

      bin = path.join(Global.Path.bin, "tinymist" + (platform === "win32" ? ".exe" : ""))

      if (!(await Filesystem.exists(bin))) {
        log.error("Failed to extract tinymist binary")
        return
      }

      if (platform !== "win32") {
        await fs.chmod(bin, 0o755).catch(() => {})
      }

      log.info("installed tinymist", { bin })
    }

    return {
      process: spawn(bin, { cwd: root }),
    }
  },
}


export const HLS: Info = {
  id: "haskell-language-server",
  extensions: [".hs", ".lhs"],
  root: NearestRoot(["stack.yaml", "cabal.project", "hie.yaml", "*.cabal"]),
  async spawn(root) {
    const bin = which("haskell-language-server-wrapper")
    if (!bin) {
      log.info("haskell-language-server-wrapper not found, please install haskell-language-server")
      return
    }
    return {
      process: spawn(bin, ["--lsp"], {
        cwd: root,
      }),
    }
  },
}


export const JuliaLS: Info = {
  id: "julials",
  extensions: [".jl"],
  root: NearestRoot(["Project.toml", "Manifest.toml", "*.jl"]),
  async spawn(root) {
    const julia = which("julia")
    if (!julia) {
      log.info("julia not found, please install julia first (https://julialang.org/downloads/)")
      return
    }
    return {
      process: spawn(julia, ["--startup-file=no", "--history-file=no", "-e", "using LanguageServer; runserver()"], {
        cwd: root,
      }),
    }
  },
}
