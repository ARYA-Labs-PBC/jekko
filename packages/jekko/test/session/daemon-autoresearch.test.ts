import { describe, expect, test } from "bun:test"
import { Effect } from "effect"
import fs from "fs/promises"
import path from "path"
import { InstanceRef } from "../../src/effect/instance-ref"
import { resolveInstanceRoot } from "../../src/project/instance-root"
import {
  normalizeDaemonSpec,
  resolveDaemonModel,
  runAutoResearch,
} from "../../src/session/daemon-autoresearch"
import { tmpdir } from "../fixture/fixture"

function baseSpec() {
  return {
    version: "v1",
    intent: "daemon",
    confirm: "RUN_FOREVER",
    id: "autoresearch-smoke",
    job: {
      name: "AutoResearch smoke",
      objective: "Exercise the lane executor.",
    },
    models: {
      builder: {
        provider: "openai",
        model: "gpt-4o",
      },
      critic: {
        provider: "anthropic",
        model: "claude-3-5-sonnet",
        must_use_different_provider: true,
        must_differ_from_builder: true,
      },
      profiles: {
        builder: {
          provider: "openai",
          model: "gpt-4o",
        },
        critic: {
          provider: "anthropic",
          model: "claude-3-5-sonnet",
        },
      },
    },
    fleet: {
      max_workers: 2,
    },
    experiments: {
      scoring: {
        command: "echo scoring",
        goal_direction: "maximize",
      },
      lanes: [
        {
          id: "lane-a",
          hypothesis: "Try the first lane",
          prompt_strategy: "default",
          isolation: "git_worktree",
          timeout: "5 minutes",
        },
      ],
      max_parallel: 1,
    },
    fan_out: {
      split: {
        shell: "echo split",
      },
      reduce: {
        command: "echo reduce",
      },
      worker: {
        max_parallel: 1,
        timeout: "5 minutes",
      },
    },
  } as any
}

describe("daemon autoresearch", () => {
  test("normalizes legacy provider profiles to Jnoccio", () => {
    const normalized = normalizeDaemonSpec(baseSpec())
    expect(resolveDaemonModel()).toEqual({
      providerID: "jnoccio",
      modelID: "jnoccio-fusion",
    })
    expect(normalized.spec.models.profiles.builder).toEqual({
      provider: "jnoccio",
      model: "jnoccio-fusion",
    })
    expect(normalized.spec.models.profiles.critic).toEqual({
      provider: "jnoccio",
      model: "jnoccio-fusion",
    })
    expect(normalized.spec.models.critic.must_use_different_provider).toBe(false)
    expect(normalized.spec.models.critic.must_differ_from_builder).toBe(false)
    expect(normalized.spec.fleet.jnoccio.base_url).toBe("http://127.0.0.1:4317")
  })

  test("runs the lane executor, writes artifacts, and routes prompts through Jnoccio", async () => {
    await using tmp = await tmpdir()
    const directory = tmp.path
    const rootCtx = {
      directory,
      worktree: directory,
      project: {
        id: "proj-autoresearch-smoke",
        worktree: directory,
        sandboxes: [],
        time: { created: Date.now(), updated: Date.now() },
      },
    } as any

    const spec = normalizeDaemonSpec(baseSpec()).spec
    const run = {
      id: "run-autoresearch-smoke",
      iteration: 0,
      active_session_id: "ses-autoresearch-smoke",
      root_session_id: "ses-autoresearch-smoke",
      status: "running",
      phase: "running",
    } as any

    const events: Array<{ eventType: string; payload: Record<string, unknown> }> = []
    const iterations: Array<Record<string, unknown>> = []
    const promptCalls: Array<Record<string, unknown>> = []
    const shellCalls: string[] = []
    const laneDir = path.join(directory, "lane-worktree")

    const result = await Effect.runPromise(
      runAutoResearch({
        run,
        spec,
        store: {
          appendEvent: (input: any) =>
            Effect.sync(() => {
              events.push({ eventType: input.eventType, payload: input.payload })
              return input
            }),
          appendIteration: (input: any) =>
            Effect.sync(() => {
              iterations.push(input)
              return input
            }),
        } as any,
        sessions: {
          fork: () => Effect.succeed({ id: "lane-session-1" }),
        } as any,
        prompt: {
          prompt: (input: any) =>
            Effect.sync(() => {
              promptCalls.push(input)
            }),
          loopResult: () =>
            Effect.succeed({
              message: {
                info: {
                  id: "assistant-1",
                  role: "assistant",
                  finish: "stop",
                  tokens: { input: 12, output: 34, total: 46 },
                  cost: 1.25,
                },
              },
              terminal: "assistant_stop",
            }),
        } as any,
        checks: {
          runShellCheck: (input: any) =>
            Effect.sync(() => {
              shellCalls.push(input.command)
              if (input.command === "git diff --binary HEAD") {
                return {
                  exitCode: 0,
                  matched: true,
                  stdout: "diff --git a/example.txt b/example.txt\n--- a/example.txt\n+++ b/example.txt\n",
                }
              }
              return { exitCode: 0, matched: true, stdout: "ok" }
            }),
        } as any,
        worktree: {
          create: ({ name }: any) =>
            Effect.sync(() => ({
              directory: laneDir,
              branch: `lane/${name}`,
            })),
        } as any,
        instanceStore: {
          provide: (input: any, effect: Effect.Effect<unknown>) =>
            effect.pipe(
              Effect.provideService(InstanceRef, {
                directory: input.directory,
                worktree: input.worktree,
                project: input.project,
              } as any),
            ),
        } as any,
        transitionRun: (_runID: string, patch: any) =>
          Effect.sync(() => ({
            ...run,
            ...patch,
          })),
      }).pipe(Effect.provideService(InstanceRef, rootCtx)),
    )

    expect(promptCalls).toHaveLength(1)
    expect(promptCalls[0].model).toEqual({
      providerID: "jnoccio",
      modelID: "jnoccio-fusion",
    })
    expect(shellCalls).toContain("echo split")
    expect(shellCalls).toContain("echo scoring")
    expect(shellCalls).toContain("echo reduce")
    expect(shellCalls).toContain("git diff --binary HEAD")

    const artifactRoot = path.join(directory, ".jekko", "daemon", run.id)
    expect(result.artifactRoot).toBe(artifactRoot)
    expect(resolveInstanceRoot({ directory, worktree: "/" })).toBe(directory)
    expect(await fs.readFile(path.join(artifactRoot, "scoreboard.tsv"), "utf8")).toContain("rank\tname\tsource")
    expect(await fs.readFile(path.join(artifactRoot, "best-state.json"), "utf8")).toContain("\"winner\"")
    expect(await fs.readFile(path.join(artifactRoot, "promotion-decision.json"), "utf8")).toContain("\"promoted\"")
    expect(await fs.readFile(path.join(artifactRoot, "best.patch"), "utf8")).toContain("diff --git")
    expect(await fs.readFile(path.join(artifactRoot, "negative-memory.jsonl"), "utf8")).toContain("\n")
    expect(await fs.readFile(path.join(artifactRoot, "curriculum-proposals.json"), "utf8")).toContain("lane-a")
    expect(await fs.readFile(path.join(artifactRoot, "reports", "final-score.json"), "utf8")).toContain("lane-a")
    expect(events.map((event) => event.eventType)).toContain("autoresearch.started")
    expect(events.map((event) => event.eventType)).toContain("autoresearch.lane.finished")
    expect(events.map((event) => event.eventType)).toContain("autoresearch.reduce.finished")
    expect(iterations[0]).toMatchObject({
      runID: run.id,
      terminalReason: "autoresearch",
    })
  })
})
