import { afterEach, describe, expect, test } from "bun:test"
import { Option, Redacted } from "effect"
import { Flag } from "@jekko-ai/core/flag/flag"
import { ServerAuth } from "../../src/server/auth"

const original = {
  JEKKO_SERVER_PASSWORD: Flag.JEKKO_SERVER_PASSWORD,
  JEKKO_SERVER_USERNAME: Flag.JEKKO_SERVER_USERNAME,
}
const authKey = "password"

afterEach(() => {
  Flag.JEKKO_SERVER_PASSWORD = original.JEKKO_SERVER_PASSWORD
  Flag.JEKKO_SERVER_USERNAME = original.JEKKO_SERVER_USERNAME
})

describe("ServerAuth", () => {
  test("does not emit auth headers without a password", () => {
    Flag.JEKKO_SERVER_PASSWORD = undefined
    Flag.JEKKO_SERVER_USERNAME = "alice"

    expect(ServerAuth.header()).toBeUndefined()
    expect(ServerAuth.headers()).toBeUndefined()
  })

  test("defaults to the jekko username", () => {
    Flag.JEKKO_SERVER_PASSWORD = "example-password"
    Flag.JEKKO_SERVER_USERNAME = undefined

    expect(ServerAuth.headers()).toEqual({
      Authorization: `Basic ${Buffer.from("jekko:example-password").toString("base64")}`,
    })
  })

  test("uses the configured username", () => {
    Flag.JEKKO_SERVER_PASSWORD = "example-password"
    Flag.JEKKO_SERVER_USERNAME = "alice"

    expect(ServerAuth.headers()).toEqual({
      Authorization: `Basic ${Buffer.from("alice:example-password").toString("base64")}`,
    })
  })

  test("prefers explicit credentials", () => {
    Flag.JEKKO_SERVER_PASSWORD = "example-password"
    Flag.JEKKO_SERVER_USERNAME = "alice"

    expect(ServerAuth.headers({ [authKey]: "example-cli-password", username: "bob" })).toEqual({
      Authorization: `Basic ${Buffer.from("bob:example-cli-password").toString("base64")}`,
    })
  })

  test("validates decoded credentials against effect config", () => {
    const config = { [authKey]: Option.some("example-password"), username: "alice" }

    expect(ServerAuth.required(config)).toBe(true)
    expect(ServerAuth.authorized({ username: "alice", [authKey]: Redacted.make("example-password") }, config)).toBe(
      true,
    )
    expect(ServerAuth.authorized({ username: "jekko", [authKey]: Redacted.make("example-password") }, config)).toBe(
      false,
    )
  })
})
