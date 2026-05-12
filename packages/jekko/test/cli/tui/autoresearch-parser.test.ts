import { describe, expect, test } from "bun:test"
import { parseBestState, parseScoreboard } from "../../../src/cli/cmd/tui/context/autoresearch-parser"
import { detectAutoResearch } from "../../../src/cli/cmd/tui/context/autoresearch-state"

describe("AutoResearch parsers", () => {
  test("parses the current scoreboard.tsv format", () => {
    const scores = parseScoreboard([
      "rank\tname\tsource\tci95_low\ttotal\tstress_total\tgate_count\tcost_usd\tdelta\tstatus",
      "1\tlane-a\tpass\t12.5\t98.5\t96.5\t3\t1.2500\t+0.5\tpass",
      "2\tlane-b\tfail\t10.0\t91.0\t90.0\t0\t0.2500\t-7.0\tfail",
    ].join("\n"))

    expect(scores).toHaveLength(2)
    expect(scores[0]).toMatchObject({
      laneId: "lane-a",
      rank: 1,
      score: 98.5,
      ci95Low: 12.5,
      stressTotal: 96.5,
      gateCount: 3,
      costUsd: 1.25,
      delta: 0.5,
      status: "pass",
    })
    expect(scores[1]).toMatchObject({
      laneId: "lane-b",
      rank: 2,
      score: 91,
      status: "fail",
    })
  })

  test("parses nested best-state winner/selected/current records", () => {
    const best = parseBestState(JSON.stringify({
      winner: {
        score: 101.25,
        lane_id: "lane-c",
        iteration: 4,
        timestamp: 12345,
      },
      selected: {
        score: 99.0,
        lane_id: "lane-b",
      },
      current: {
        score: 98.0,
        lane_id: "lane-a",
      },
    }))

    expect(best).toEqual({
      score: 101.25,
      laneId: "lane-c",
      iteration: 4,
      timestamp: 12345,
      source: "winner",
    })
  })
})

describe("AutoResearch detection", () => {
  test("derives the daemon directory from reduce command flags", () => {
    const run = {
      id: "memory-benchmark-chase",
      status: "running",
      spec_json: {
        job: { name: "Memory benchmark chase" },
        experiments: {
          scoring: {
            command: "test -f .jekko/daemon/memory-benchmark-chase/scoreboard.tsv",
            goal_direction: "maximize",
          },
          lanes: [{ id: "lane-a" }],
        },
        fan_out: {
          reduce: {
            command:
              "cargo run -- --scoreboard .jekko/daemon/memory-benchmark-chase/scoreboard.tsv --best-state .jekko/daemon/memory-benchmark-chase/best-state.json",
          },
        },
      },
    }

    const detected = detectAutoResearch(run)
    expect(detected).toEqual({
      runId: "memory-benchmark-chase",
      daemonDir: ".jekko/daemon/memory-benchmark-chase",
      jobName: "Memory benchmark chase",
      goalDirection: "maximize",
      totalLanes: 1,
    })
  })
})
