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


export const Gopls: Info = {
  id: "gopls",
  root: async (file, ctx) => {
    const work = await NearestRoot(["go.work"])(file, ctx)
    if (work) return work
    return NearestRoot(["go.mod", "go.sum"])(file, ctx)
  },
  extensions: [".go"],
  async spawn(root) {
    let bin = which("gopls")
    if (!bin) {
      if (!which("go")) return
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return

      log.info("installing gopls")
      const proc = Process.spawn(["go", "install", "golang.org/x/tools/gopls@latest"], {
        env: { ...process.env, GOBIN: Global.Path.bin },
        stdout: "pipe",
        stderr: "pipe",
        stdin: "pipe",
      })
      const exit = await proc.exited
      if (exit !== 0) {
        log.error("Failed to install gopls")
        return
      }
      bin = path.join(Global.Path.bin, "gopls" + (process.platform === "win32" ? ".exe" : ""))
      log.info(`installed gopls`, {
        bin,
      })
    }
    return {
      process: spawn(bin!, {
        cwd: root,
      }),
    }
  },
}


export const Rubocop: Info = {
  id: "ruby-lsp",
  root: NearestRoot(["Gemfile"]),
  extensions: [".rb", ".rake", ".gemspec", ".ru"],
  async spawn(root) {
    let bin = which("rubocop")
    if (!bin) {
      const ruby = which("ruby")
      const gem = which("gem")
      if (!ruby || !gem) {
        log.info("Ruby not found, please install Ruby first")
        return
      }
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      log.info("installing rubocop")
      const proc = Process.spawn(["gem", "install", "rubocop", "--bindir", Global.Path.bin], {
        stdout: "pipe",
        stderr: "pipe",
        stdin: "pipe",
      })
      const exit = await proc.exited
      if (exit !== 0) {
        log.error("Failed to install rubocop")
        return
      }
      bin = path.join(Global.Path.bin, "rubocop" + (process.platform === "win32" ? ".exe" : ""))
      log.info(`installed rubocop`, {
        bin,
      })
    }
    return {
      process: spawn(bin!, ["--lsp"], {
        cwd: root,
      }),
    }
  },
}


export const Ty: Info = {
  id: "ty",
  extensions: [".py", ".pyi"],
  root: NearestRoot([
    "pyproject.toml",
    "ty.toml",
    "setup.py",
    "setup.cfg",
    "requirements.txt",
    "Pipfile",
    "pyrightconfig.json",
  ]),
  async spawn(root) {
    if (!Flag.JEKKO_EXPERIMENTAL_LSP_TY) {
      // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
      return undefined
    }

    let binary = which("ty")

    const initialization: Record<string, string> = {}

    const potentialVenvPaths = [process.env["VIRTUAL_ENV"], path.join(root, ".venv"), path.join(root, "venv")].filter(
      (p): p is string => p !== undefined,
    )
    for (const venvPath of potentialVenvPaths) {
      const isWindows = process.platform === "win32"
      const potentialPythonPath = isWindows
        ? path.join(venvPath, "Scripts", "python.exe")
        : path.join(venvPath, "bin", "python")
      if (await Filesystem.exists(potentialPythonPath)) {
        initialization["pythonPath"] = potentialPythonPath
        break
      }
    }

    if (!binary) {
      for (const venvPath of potentialVenvPaths) {
        const isWindows = process.platform === "win32"
        const potentialTyPath = isWindows ? path.join(venvPath, "Scripts", "ty.exe") : path.join(venvPath, "bin", "ty")
        if (await Filesystem.exists(potentialTyPath)) {
          binary = potentialTyPath
          break
        }
      }
    }

    if (!binary) {
      log.error("ty not found, please install ty first")
      return
    }

    const proc = spawn(binary, ["server"], {
      cwd: root,
    })

    return {
      process: proc,
      initialization,
    }
  },
}


export const Pyright: Info = {
  id: "pyright",
  extensions: [".py", ".pyi"],
  root: NearestRoot(["pyproject.toml", "setup.py", "setup.cfg", "requirements.txt", "Pipfile", "pyrightconfig.json"]),
  async spawn(root) {
    let binary = which("pyright-langserver")
    const args = []
    if (!binary) {
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      const resolved = await Npm.which("pyright", "pyright-langserver")
      if (!resolved) return
      binary = resolved
    }
    args.push("--stdio")

    const initialization: Record<string, string> = {}

    const potentialVenvPaths = [process.env["VIRTUAL_ENV"], path.join(root, ".venv"), path.join(root, "venv")].filter(
      (p): p is string => p !== undefined,
    )
    for (const venvPath of potentialVenvPaths) {
      const isWindows = process.platform === "win32"
      const potentialPythonPath = isWindows
        ? path.join(venvPath, "Scripts", "python.exe")
        : path.join(venvPath, "bin", "python")
      if (await Filesystem.exists(potentialPythonPath)) {
        initialization["pythonPath"] = potentialPythonPath
        break
      }
    }

    const proc = spawn(binary, args, {
      cwd: root,
      env: {
        ...process.env,
      },
    })
    return {
      process: proc,
      initialization,
    }
  },
}


export const ElixirLS: Info = {
  id: "elixir-ls",
  extensions: [".ex", ".exs"],
  root: NearestRoot(["mix.exs", "mix.lock"]),
  async spawn(root) {
    let binary = which("elixir-ls")
    if (!binary) {
      const elixirLsPath = path.join(Global.Path.bin, "elixir-ls")
      binary = path.join(
        Global.Path.bin,
        "elixir-ls-master",
        "release",
        process.platform === "win32" ? "language_server.bat" : "language_server.sh",
      )

      if (!(await Filesystem.exists(binary))) {
        const elixir = which("elixir")
        if (!elixir) {
          log.error("elixir is required to run elixir-ls")
          return
        }

        if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
        log.info("downloading elixir-ls from GitHub releases")

        const response = await fetch("https://github.com/elixir-lsp/elixir-ls/archive/refs/heads/master.zip")
        if (!response.ok) return
        const zipPath = path.join(Global.Path.bin, "elixir-ls.zip")
        if (response.body) await Filesystem.writeStream(zipPath, response.body)

        const ok = await Archive.extractZip(zipPath, Global.Path.bin)
          .then(() => true)
          .catch((error) => {
            log.error("Failed to extract elixir-ls archive", { error })
            return false
          })
        if (!ok) return

        await fs.rm(zipPath, {
          force: true,
          recursive: true,
        })

        const cwd = path.join(Global.Path.bin, "elixir-ls-master")
        const env = { MIX_ENV: "prod", ...process.env }
        await Process.run(["mix", "deps.get"], { cwd, env })
        await Process.run(["mix", "compile"], { cwd, env })
        await Process.run(["mix", "elixir_ls.release2", "-o", "release"], { cwd, env })

        log.info(`installed elixir-ls`, {
          path: elixirLsPath,
        })
      }
    }

    return {
      process: spawn(binary, {
        cwd: root,
      }),
    }
  },
}


export const Zls: Info = {
  id: "zls",
  extensions: [".zig", ".zon"],
  root: NearestRoot(["build.zig"]),
  async spawn(root) {
    let bin = which("zls")

    if (!bin) {
      const zig = which("zig")
      if (!zig) {
        log.error("Zig is required to use zls. Please install Zig first.")
        return
      }

      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      log.info("downloading zls from GitHub releases")

      const releaseResponse = await fetch("https://api.github.com/repos/zigtools/zls/releases/latest")
      if (!releaseResponse.ok) {
        log.error("Failed to fetch zls release info")
        return
      }

      const releaseResult = ZlsReleaseSchema.safeParse(await releaseResponse.json())
      if (!releaseResult.success) {
        log.error("Failed to parse zls release info")
        return
      }
      const release = releaseResult.data

      const platform = process.platform
      const arch = process.arch
      let assetName = ""

      let zlsArch: string = arch
      if (arch === "arm64") zlsArch = "aarch64"
      else if (arch === "x64") zlsArch = "x86_64"
      else if (arch === "ia32") zlsArch = "x86"

      let zlsPlatform: string = platform
      if (platform === "darwin") zlsPlatform = "macos"
      else if (platform === "win32") zlsPlatform = "windows"

      const ext = platform === "win32" ? "zip" : "tar.xz"

      assetName = `zls-${zlsArch}-${zlsPlatform}.${ext}`

      const supportedCombos = [
        "zls-x86_64-linux.tar.xz",
        "zls-x86_64-macos.tar.xz",
        "zls-x86_64-windows.zip",
        "zls-aarch64-linux.tar.xz",
        "zls-aarch64-macos.tar.xz",
        "zls-aarch64-windows.zip",
        "zls-x86-linux.tar.xz",
        "zls-x86-windows.zip",
      ]

      if (!supportedCombos.includes(assetName)) {
        log.error(`Platform ${platform} and architecture ${arch} is not supported by zls`)
        return
      }

      const asset = release.assets?.find((a) => a.name === assetName)
      if (!asset?.browser_download_url) {
        log.error(`Could not find asset ${assetName} in latest zls release`)
        return
      }

      const downloadUrl = asset.browser_download_url
      const downloadResponse = await fetch(downloadUrl)
      if (!downloadResponse.ok) {
        log.error("Failed to download zls")
        return
      }

      const tempPath = path.join(Global.Path.bin, assetName)
      if (downloadResponse.body) await Filesystem.writeStream(tempPath, downloadResponse.body)

      if (ext === "zip") {
        const ok = await Archive.extractZip(tempPath, Global.Path.bin)
          .then(() => true)
          .catch((error) => {
            log.error("Failed to extract zls archive", { error })
            return false
          })
        if (!ok) return
      } else {
        await run(["tar", "-xf", tempPath], { cwd: Global.Path.bin })
      }

      await fs.rm(tempPath, { force: true })

      bin = path.join(Global.Path.bin, "zls" + (platform === "win32" ? ".exe" : ""))

      if (!(await Filesystem.exists(bin))) {
        log.error("Failed to extract zls binary")
        return
      }

      if (platform !== "win32") {
        await fs.chmod(bin, 0o755).catch(() => {})
      }

      log.info(`installed zls`, { bin })
    }

    return {
      process: spawn(bin, {
        cwd: root,
      }),
    }
  },
}
