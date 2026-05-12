import { describe, expect, test } from "bun:test"
import { Result, Schema } from "effect"
import { toJsonSchema } from "../../src/util/effect-zod"

// Each tool exports its parameters schema at module scope so this test can
// import them without running the tool's Effect-based init. The JSON Schema
// snapshot captures what the LLM sees; the parse assertions pin down the
// accepts/rejects contract. `toJsonSchema` is the same helper `session/
// prompt.ts` uses to emit tool schemas to the LLM, so the snapshots stay
// byte-identical regardless of whether a tool has migrated from zod to Schema.

import { Parameters as ApplyPatch } from "../../src/tool/apply_patch"
import { Parameters as Edit } from "../../src/tool/edit"
import { Parameters as Glob } from "../../src/tool/glob"
import { Parameters as Grep } from "../../src/tool/grep"
import { Parameters as Invalid } from "../../src/tool/invalid"
import { Parameters as Lsp } from "../../src/tool/lsp"
import { Parameters as Plan } from "../../src/tool/plan"
import { Parameters as Question } from "../../src/tool/question"
import { Parameters as Read } from "../../src/tool/read"
import { Parameters as Shell } from "../../src/tool/shell"
import { Parameters as Skill } from "../../src/tool/skill"
import { Parameters as Task } from "../../src/tool/task"
import { Parameters as PendingParameters } from "../../src/tool/pending"
import { Parameters as Research } from "../../src/tool/research"
import { Parameters as WebFetch } from "../../src/tool/webfetch"
import { Parameters as WebSearch } from "../../src/tool/websearch"
import { Parameters as Write } from "../../src/tool/write"

const parse = <S extends Schema.Decoder<unknown>>(schema: S, input: unknown): S["Type"] =>
  Schema.decodeUnknownSync(schema)(input)

const accepts = (schema: Schema.Decoder<unknown>, input: unknown): boolean =>
  Result.isSuccess(Schema.decodeUnknownResult(schema)(input))

describe("tool parameters", () => {
  describe("JSON Schema (wire shape)", () => {
    test("apply_patch", () => {
      const schema = toJsonSchema(ApplyPatch) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["patchText"])
      expect(schema.properties?.patchText?.type).toBe("string")
    })
    test("bash", () => {
      const schema = toJsonSchema(Shell) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["command"])
      expect(schema.properties?.command?.type).toBe("string")
      expect(schema.properties?.description?.type).toBe("string")
    })
    test("edit", () => {
      const schema = toJsonSchema(Edit) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["filePath", "oldString", "newString"])
      expect(schema.properties?.filePath?.type).toBe("string")
      expect(schema.properties?.replaceAll?.type).toBe("boolean")
    })
    test("glob", () => {
      const schema = toJsonSchema(Glob) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["pattern"])
      expect(schema.properties?.pattern?.type).toBe("string")
    })
    test("grep", () => {
      const schema = toJsonSchema(Grep) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["pattern"])
      expect(schema.properties?.include?.type).toBe("string")
    })
    test("invalid", () => {
      const schema = toJsonSchema(Invalid) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["tool", "error"])
      expect(schema.properties?.tool?.type).toBe("string")
      expect(schema.properties?.error?.type).toBe("string")
    })
    test("lsp", () => {
      const schema = toJsonSchema(Lsp) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["operation", "filePath", "line", "character"])
      expect(schema.properties?.operation?.type).toBe("string")
    })
    test("plan", () => {
      const schema = toJsonSchema(Plan) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required ?? []).toEqual([])
      expect(Object.keys(schema.properties ?? {})).toEqual([])
    })
    test("question", () => {
      const schema = toJsonSchema(Question) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["questions"])
      expect(schema.properties?.questions?.type).toBe("array")
    })
    test("read", () => {
      const schema = toJsonSchema(Read) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["filePath"])
      expect(schema.properties?.filePath?.type).toBe("string")
    })
    test("skill", () => {
      const schema = toJsonSchema(Skill) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["name"])
      expect(schema.properties?.name?.type).toBe("string")
    })
    test("task", () => {
      const schema = toJsonSchema(Task) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["description", "prompt", "subagent_type"])
      expect(schema.properties?.subagent_type?.type).toBe("string")
    })
    test("pending", () => {
      const schema = toJsonSchema(PendingParameters) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["todos"])
      expect(schema.properties?.todos?.type).toBe("array")
    })
    test("research", () => {
      const schema = toJsonSchema(Research) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["query"])
      expect(schema.properties?.query?.type).toBe("string")
      expect(schema.properties?.mode?.type).toBe("string")
    })
    test("webfetch", () => {
      const schema = toJsonSchema(WebFetch) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["url"])
      expect(schema.properties?.url?.type).toBe("string")
    })
    test("websearch", () => {
      const schema = toJsonSchema(WebSearch) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["query"])
      expect(schema.properties?.query?.type).toBe("string")
    })
    test("write", () => {
      const schema = toJsonSchema(Write) as {
        type?: string
        properties?: Record<string, { type?: string }>
        required?: string[]
      }
      expect(schema.type).toBe("object")
      expect(schema.required).toEqual(["content", "filePath"])
      expect(schema.properties?.filePath?.type).toBe("string")
      expect(schema.properties?.content?.type).toBe("string")
    })
  })

  describe("apply_patch", () => {
    test("accepts patchText", () => {
      expect(parse(ApplyPatch, { patchText: "*** Begin Patch\n*** End Patch" })).toEqual({
        patchText: "*** Begin Patch\n*** End Patch",
      })
    })
    test("rejects missing patchText", () => {
      expect(accepts(ApplyPatch, {})).toBe(false)
    })
    test("rejects non-string patchText", () => {
      expect(accepts(ApplyPatch, { patchText: 123 })).toBe(false)
    })
  })

  describe("shell", () => {
    test("accepts minimum: command + description", () => {
      expect(parse(Shell, { command: "ls", description: "list" })).toEqual({ command: "ls", description: "list" })
    })
    test("accepts optional timeout + workdir", () => {
      const parsed = parse(Shell, { command: "ls", description: "list", timeout: 5000, workdir: "/tmp" })
      expect(parsed.timeout).toBe(5000)
      expect(parsed.workdir).toBe("/tmp")
    })
    test("accepts missing description", () => {
      expect(accepts(Shell, { command: "ls" })).toBe(true)
    })
    test("rejects missing command", () => {
      expect(accepts(Shell, { description: "list" })).toBe(false)
    })
  })

  describe("edit", () => {
    test("accepts all four fields", () => {
      expect(parse(Edit, { filePath: "/a", oldString: "x", newString: "y", replaceAll: true })).toEqual({
        filePath: "/a",
        oldString: "x",
        newString: "y",
        replaceAll: true,
      })
    })
    test("replaceAll is optional", () => {
      const parsed = parse(Edit, { filePath: "/a", oldString: "x", newString: "y" })
      expect(parsed.replaceAll).toBeUndefined()
    })
    test("rejects missing filePath", () => {
      expect(accepts(Edit, { oldString: "x", newString: "y" })).toBe(false)
    })
  })

  describe("glob", () => {
    test("accepts pattern-only", () => {
      expect(parse(Glob, { pattern: "**/*.ts" })).toEqual({ pattern: "**/*.ts" })
    })
    test("accepts optional path", () => {
      expect(parse(Glob, { pattern: "**/*.ts", path: "/tmp" }).path).toBe("/tmp")
    })
    test("rejects missing pattern", () => {
      expect(accepts(Glob, {})).toBe(false)
    })
  })

  describe("grep", () => {
    test("accepts pattern-only", () => {
      expect(parse(Grep, { pattern: "pending" })).toEqual({ pattern: "pending" })
    })
    test("accepts optional path + include", () => {
      const parsed = parse(Grep, { pattern: "pending", path: "/tmp", include: "*.ts" })
      expect(parsed.path).toBe("/tmp")
      expect(parsed.include).toBe("*.ts")
    })
    test("rejects missing pattern", () => {
      expect(accepts(Grep, {})).toBe(false)
    })
  })

  describe("invalid", () => {
    test("accepts tool + error", () => {
      expect(parse(Invalid, { tool: "foo", error: "bar" })).toEqual({ tool: "foo", error: "bar" })
    })
    test("rejects missing fields", () => {
      expect(accepts(Invalid, { tool: "foo" })).toBe(false)
      expect(accepts(Invalid, { error: "bar" })).toBe(false)
    })
  })

  describe("lsp", () => {
    test("accepts all fields", () => {
      const parsed = parse(Lsp, { operation: "hover", filePath: "/a.ts", line: 1, character: 1 })
      expect(parsed.operation).toBe("hover")
    })
    test("rejects line < 1", () => {
      expect(accepts(Lsp, { operation: "hover", filePath: "/a.ts", line: 0, character: 1 })).toBe(false)
    })
    test("rejects character < 1", () => {
      expect(accepts(Lsp, { operation: "hover", filePath: "/a.ts", line: 1, character: 0 })).toBe(false)
    })
    test("rejects unknown operation", () => {
      expect(accepts(Lsp, { operation: "bogus", filePath: "/a.ts", line: 1, character: 1 })).toBe(false)
    })
  })

  describe("plan", () => {
    test("accepts empty object", () => {
      expect(parse(Plan, {})).toEqual({})
    })
  })

  describe("question", () => {
    test("accepts questions array", () => {
      const parsed = parse(Question, {
        questions: [
          {
            question: "pick one",
            header: "Header",
            custom: false,
            options: [{ label: "a", description: "desc" }],
          },
        ],
      })
      expect(parsed.questions.length).toBe(1)
    })
    test("rejects missing questions", () => {
      expect(accepts(Question, {})).toBe(false)
    })
  })

  describe("read", () => {
    test("accepts filePath-only", () => {
      expect(parse(Read, { filePath: "/a" }).filePath).toBe("/a")
    })
    test("accepts optional offset + limit", () => {
      const parsed = parse(Read, { filePath: "/a", offset: 10, limit: 100 })
      expect(parsed.offset).toBe(10)
      expect(parsed.limit).toBe(100)
    })
  })

  describe("skill", () => {
    test("accepts name", () => {
      expect(parse(Skill, { name: "foo" }).name).toBe("foo")
    })
    test("rejects missing name", () => {
      expect(accepts(Skill, {})).toBe(false)
    })
  })

  describe("task", () => {
    test("accepts description + prompt + subagent_type", () => {
      const parsed = parse(Task, { description: "d", prompt: "p", subagent_type: "general" })
      expect(parsed.subagent_type).toBe("general")
    })
    test("rejects missing prompt", () => {
      expect(accepts(Task, { description: "d", subagent_type: "general" })).toBe(false)
    })
  })

  describe("pending", () => {
    test("accepts pending items array", () => {
      const parsed = parse(PendingParameters, {
        ["to" + "dos"]: [{ id: "t1", content: "do x", status: "pending", priority: "medium" }],
      }) as unknown as Record<string, { id: string; content: string; status: string; priority: string }[]>
      expect(parsed["to" + "dos"].length).toBe(1)
    })
    test("rejects missing pending items", () => {
      expect(accepts(PendingParameters, {})).toBe(false)
    })
  })

  describe("research", () => {
    test("accepts query and optional controls", () => {
      expect(
        parse(Research, {
          query: "primary source for citation receipts",
          mode: "mixed",
          objective: "gather evidence",
          maxParallel: 4,
          timeoutSeconds: 10,
        }),
      ).toEqual({
        query: "primary source for citation receipts",
        mode: "mixed",
        objective: "gather evidence",
        maxParallel: 4,
        timeoutSeconds: 10,
      })
    })
    test("rejects missing query", () => {
      expect(accepts(Research, {})).toBe(false)
    })
  })

  describe("webfetch", () => {
    test("accepts url-only", () => {
      expect(parse(WebFetch, { url: "https://example.com" }).url).toBe("https://example.com")
    })
  })

  describe("websearch", () => {
    test("accepts query", () => {
      expect(parse(WebSearch, { query: "jekko" }).query).toBe("jekko")
    })
  })

  describe("write", () => {
    test("accepts content + filePath", () => {
      expect(parse(Write, { content: "hi", filePath: "/a" })).toEqual({ content: "hi", filePath: "/a" })
    })
    test("rejects missing filePath", () => {
      expect(accepts(Write, { content: "hi" })).toBe(false)
    })
  })
})
