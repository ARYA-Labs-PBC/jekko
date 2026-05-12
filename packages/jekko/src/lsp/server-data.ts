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


export const PHPIntelephense: Info = {
  id: "php intelephense",
  extensions: [".php"],
  root: NearestRoot(["composer.json", "composer.lock", ".php-version"]),
  async spawn(root) {
    let binary = which("intelephense")
    const args: string[] = []
    if (!binary) {
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      const resolved = await Npm.which("intelephense")
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
      initialization: {
        telemetry: {
          enabled: false,
        },
      },
    }
  },
}


export const Prisma: Info = {
  id: "prisma",
  extensions: [".prisma"],
  root: NearestRoot(["schema.prisma", "prisma/schema.prisma", "prisma"], ["package.json"]),
  async spawn(root) {
    const prisma = which("prisma")
    if (!prisma) {
      log.info("prisma not found, please install prisma")
      return
    }
    return {
      process: spawn(prisma, ["language-server"], {
        cwd: root,
      }),
    }
  },
}


export const Dart: Info = {
  id: "dart",
  extensions: [".dart"],
  root: NearestRoot(["pubspec.yaml", "analysis_options.yaml"]),
  async spawn(root) {
    const dart = which("dart")
    if (!dart) {
      log.info("dart not found, please install dart first")
      return
    }
    return {
      process: spawn(dart, ["language-server", "--lsp"], {
        cwd: root,
      }),
    }
  },
}


export const Ocaml: Info = {
  id: "ocaml-lsp",
  extensions: [".ml", ".mli"],
  root: NearestRoot(["dune-project", "dune-workspace", ".merlin", "opam"]),
  async spawn(root) {
    const bin = which("ocamllsp")
    if (!bin) {
      log.info("ocamllsp not found, please install ocaml-lsp-server")
      return
    }
    return {
      process: spawn(bin, {
        cwd: root,
      }),
    }
  },
}

export const BashLS: Info = {
  id: "bash",
  extensions: [".sh", ".bash", ".zsh", ".ksh"],
  root: async (_file, ctx) => ctx.directory,
  async spawn(root) {
    let binary = which("bash-language-server")
    const args: string[] = []
    if (!binary) {
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      const resolved = await Npm.which("bash-language-server")
      if (!resolved) return
      binary = resolved
    }
    args.push("start")
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


export const TerraformLS: Info = {
  id: "terraform",
  extensions: [".tf", ".tfvars"],
  root: NearestRoot([".terraform.lock.hcl", "terraform.tfstate", "*.tf"]),
  async spawn(root) {
    let bin = which("terraform-ls")

    if (!bin) {
      if (Flag.JEKKO_DISABLE_LSP_DOWNLOAD) return
      log.info("downloading terraform-ls from HashiCorp releases")

      const releaseResponse = await fetch("https://api.releases.hashicorp.com/v1/releases/terraform-ls/latest")
      if (!releaseResponse.ok) {
        log.error("Failed to fetch terraform-ls release info")
        return
      }

      const releaseResult = TerraformReleaseSchema.safeParse(await releaseResponse.json())
      if (!releaseResult.success) {
        log.error("Failed to parse terraform-ls release info")
        return
      }
      const release = releaseResult.data

      const platform = process.platform
      const arch = process.arch

      const tfArch = arch === "arm64" ? "arm64" : "amd64"
      const tfPlatform = platform === "win32" ? "windows" : platform

      const builds = release.builds ?? []
      const build = builds.find((b) => b.arch === tfArch && b.os === tfPlatform)
      if (!build?.url) {
        log.error(`Could not find build for ${tfPlatform}/${tfArch} terraform-ls release version ${release.version}`)
        return
      }

      const downloadResponse = await fetch(build.url)
      if (!downloadResponse.ok) {
        log.error("Failed to download terraform-ls")
        return
      }

      const tempPath = path.join(Global.Path.bin, "terraform-ls.zip")
      if (downloadResponse.body) await Filesystem.writeStream(tempPath, downloadResponse.body)

      const ok = await Archive.extractZip(tempPath, Global.Path.bin)
        .then(() => true)
        .catch((error) => {
          log.error("Failed to extract terraform-ls archive", { error })
          return false
        })
      if (!ok) return
      await fs.rm(tempPath, { force: true })

      bin = path.join(Global.Path.bin, "terraform-ls" + (platform === "win32" ? ".exe" : ""))

      if (!(await Filesystem.exists(bin))) {
        log.error("Failed to extract terraform-ls binary")
        return
      }

      if (platform !== "win32") {
        await fs.chmod(bin, 0o755).catch(() => {})
      }

      log.info(`installed terraform-ls`, { bin })
    }

    return {
      process: spawn(bin, ["serve"], {
        cwd: root,
      }),
      initialization: {
        experimentalFeatures: {
          prefillRequiredFields: true,
          validateOnSave: true,
        },
      },
    }
  },
}
