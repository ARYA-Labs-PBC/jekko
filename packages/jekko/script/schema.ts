#!/usr/bin/env bun

import { z } from "zod"
import { Config } from "@/config/config"
import { TuiConfig } from "../src/cli/cmd/tui/config/tui"

const JsonSchemaObject = z.object({}).passthrough()

function isObjectSchema(schema: z.ZodType<unknown>): schema is z.ZodObject<z.ZodRawShape> {
  return schema.type === "object"
}

function postProcessJsonSchema(value: unknown): void {
  if (!value || typeof value !== "object") return

  if (Array.isArray(value)) {
    for (const item of value) {
      postProcessJsonSchema(item)
    }
    return
  }

  const schema = value as {
    type?: unknown
    additionalProperties?: unknown
    examples?: unknown
    description?: unknown
    default?: unknown
    [key: string]: unknown
  }

  if (schema.type === "object" && schema.additionalProperties === undefined) {
    schema.additionalProperties = false
  }

  if (schema.type === "string" && schema.default) {
    if (!Array.isArray(schema.examples)) {
      schema.examples = [schema.default]
    }

    schema.description = [typeof schema.description === "string" ? schema.description : "", `default: \`${String(schema.default)}\``]
      .filter(Boolean)
      .join("\n\n")
      .trim()
  }

  for (const child of Object.values(schema)) {
    postProcessJsonSchema(child)
  }
}

function generate(schema: z.ZodType<unknown>) {
  if (!isObjectSchema(schema)) {
    throw new Error("generate expects an object schema")
  }

  const result = JsonSchemaObject.parse(z.toJSONSchema(schema, { io: "input" }))
  postProcessJsonSchema(result)

  // used for json lsps since config supports jsonc
  result.allowComments = true
  result.allowTrailingCommas = true

  return result
}

const configFile = process.argv[2]
const tuiFile = process.argv[3]

console.log(configFile)
await Bun.write(configFile, JSON.stringify(generate(Config.Info.zod), null, 2))

if (tuiFile) {
  console.log(tuiFile)
  await Bun.write(tuiFile, JSON.stringify(generate(TuiConfig.Info), null, 2))
}
