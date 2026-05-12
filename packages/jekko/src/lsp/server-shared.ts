// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
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

export const log = Log.create({ service: "lsp.server" })
export const pathExists = async (p: string) =>
  fs
    .stat(p)
    .then(() => true)
    .catch(() => false)
export const run = (cmd: string[], opts: Process.RunOptions = {}) => Process.run(cmd, { ...opts, nothrow: true })
export const output = (cmd: string[], opts: Process.RunOptions = {}) => Process.text(cmd, { ...opts, nothrow: true })

export const ZlsReleaseSchema = z.object({
  assets: z
    .array(
      z.object({
        name: z.string().optional(),
        browser_download_url: z.string().optional(),
      }),
    )
    .optional(),
})

export const TerraformReleaseSchema = z.object({
  version: z.string().optional(),
  builds: z
    .array(
      z.object({
        arch: z.string().optional(),
        os: z.string().optional(),
        url: z.string().optional(),
      }),
    )
    .optional(),
})

export const ReleaseSchema = z.object({
  tag_name: z.string().optional(),
  assets: z
    .array(
      z.object({
        name: z.string().optional(),
        browser_download_url: z.string().optional(),
      }),
    )
    .optional(),
})

export interface Handle {
  process: ChildProcessWithoutNullStreams
  initialization?: Record<string, any>
}

export type RootFunction = (file: string, ctx: InstanceContext) => Promise<string | undefined>

export const NearestRoot = (includePatterns: string[], excludePatterns?: string[]): RootFunction => {
  return async (file, ctx) => {
    if (excludePatterns) {
      const excludedFiles = Filesystem.up({
        targets: excludePatterns,
        start: path.dirname(file),
        stop: ctx.directory,
      })
      const excluded = await excludedFiles.next()
      await excludedFiles.return()
      // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
      if (excluded.value) return undefined
    }
    const files = Filesystem.up({
      targets: includePatterns,
      start: path.dirname(file),
      stop: ctx.directory,
    })
    const first = await files.next()
    await files.return()
    if (!first.value) return ctx.directory
    return path.dirname(first.value)
  }
}

export interface Info {
  id: string
  extensions: string[]
  global?: boolean
  root: RootFunction
  spawn(root: string, ctx: InstanceContext): Promise<Handle | undefined>
}
