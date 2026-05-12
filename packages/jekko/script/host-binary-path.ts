#!/usr/bin/env bun

import path from "path"

import pkg from "../package.json"

export function hostBinaryName(platform = process.platform, arch = process.arch) {
  const os = platform === "win32" ? "windows" : platform
  const rawArch = String(arch)
  const cpu = rawArch === "x64" || rawArch === "arm64" ? rawArch : rawArch === "aarch64" ? "arm64" : rawArch
  return [pkg.name, os, cpu].join("-")
}

export function hostBinaryPath() {
  return path.resolve(import.meta.dir, "..", "dist", hostBinaryName(), "bin", process.platform === "win32" ? "jekko.exe" : "jekko")
}

if (import.meta.main) {
  console.log(hostBinaryPath())
}
