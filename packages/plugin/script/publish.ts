#!/usr/bin/env bun
import { Script } from "@jekko-ai/script"
import { $ } from "bun"
import { fileURLToPath } from "url"

const dir = fileURLToPath(new URL("..", import.meta.url))
process.chdir(dir)

async function published(name: string, version: string) {
  return (await $`npm view ${name}@${version} version`.nothrow()).exitCode === 0
}

await $`bun tsc`
const originalText = await Bun.file("package.json").text()
const pkg: unknown = JSON.parse(originalText)
if (
  typeof pkg !== "object" ||
  pkg === null ||
  !("name" in pkg) ||
  !("version" in pkg) ||
  !("exports" in pkg)
) {
  throw new Error("invalid package.json shape")
}
const typedPkg = pkg as { name: string; version: string; exports: Record<string, unknown> }
if (await published(typedPkg.name, typedPkg.version)) {
  console.log(`already published ${typedPkg.name}@${typedPkg.version}`)
} else {
  for (const [key, value] of Object.entries(typedPkg.exports)) {
    if (typeof value !== "string") continue
    const file = value.replace("./src/", "./dist/").replace(".ts", "")
    pkg.exports[key] = {
      import: file + ".js",
      types: file + ".d.ts",
    }
  }
  await Bun.write("package.json", JSON.stringify(pkg, null, 2))
  try {
    await $`bun pm pack`
    await $`npm publish *.tgz --tag ${Script.channel} --access public`
  } finally {
    await Bun.write("package.json", originalText)
  }
}
