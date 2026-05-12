import { test, expect } from "bun:test"
import { parseGitHubRemote } from "../../src/cli/cmd/github"

test("parses https URL with .git suffix", () => {
  expect(parseGitHubRemote("https://github.com/sst/jekko.git")).toEqual({ owner: "sst", repo: "jekko" })
})

test("parses https URL without .git suffix", () => {
  expect(parseGitHubRemote("https://github.com/sst/jekko")).toEqual({ owner: "sst", repo: "jekko" })
})

test("parses git@ URL with .git suffix", () => {
  expect(parseGitHubRemote("git@github.com:sst/jekko.git")).toEqual({ owner: "sst", repo: "jekko" })
})

test("parses git@ URL without .git suffix", () => {
  expect(parseGitHubRemote("git@github.com:sst/jekko")).toEqual({ owner: "sst", repo: "jekko" })
})

test("parses ssh:// URL with .git suffix", () => {
  expect(parseGitHubRemote("ssh://git@github.com/sst/jekko.git")).toEqual({ owner: "sst", repo: "jekko" })
})

test("parses ssh:// URL without .git suffix", () => {
  expect(parseGitHubRemote("ssh://git@github.com/sst/jekko")).toEqual({ owner: "sst", repo: "jekko" })
})

test("parses http URL", () => {
  expect(parseGitHubRemote("http://github.com/owner/repo")).toEqual({ owner: "owner", repo: "repo" })
})

test("parses URL with hyphenated owner and repo names", () => {
  expect(parseGitHubRemote("https://github.com/my-org/my-repo.git")).toEqual({ owner: "my-org", repo: "my-repo" })
})

test("parses URL with underscores in names", () => {
  expect(parseGitHubRemote("git@github.com:my_org/my_repo.git")).toEqual({ owner: "my_org", repo: "my_repo" })
})

test("parses URL with numbers in names", () => {
  expect(parseGitHubRemote("https://github.com/org123/repo456")).toEqual({ owner: "org123", repo: "repo456" })
})

test("parses repos with dots in the name", () => {
  expect(parseGitHubRemote("https://github.com/socketio/socket.io.git")).toEqual({
    owner: "socketio",
    repo: "socket.io",
  })
  expect(parseGitHubRemote("https://github.com/vuejs/vue.js")).toEqual({
    owner: "vuejs",
    repo: "vue.js",
  })
  expect(parseGitHubRemote("git@github.com:mrdoob/three.js.git")).toEqual({
    owner: "mrdoob",
    repo: "three.js",
  })
  expect(parseGitHubRemote("https://github.com/jashkenas/backbone.git")).toEqual({
    owner: "jashkenas",
    repo: "backbone",
  })
})

test("throws for non-github URLs", () => {
  expect(() => parseGitHubRemote("https://gitlab.com/owner/repo.git")).toThrow()
  expect(() => parseGitHubRemote("git@gitlab.com:owner/repo.git")).toThrow()
  expect(() => parseGitHubRemote("https://bitbucket.org/owner/repo")).toThrow()
})

test("throws for invalid URLs", () => {
  expect(() => parseGitHubRemote("not-a-url")).toThrow()
  expect(() => parseGitHubRemote("")).toThrow()
  expect(() => parseGitHubRemote("github.com")).toThrow()
  expect(() => parseGitHubRemote("https://github.com/")).toThrow()
  expect(() => parseGitHubRemote("https://github.com/owner")).toThrow()
})

test("throws for URLs with extra path segments", () => {
  expect(() => parseGitHubRemote("https://github.com/owner/repo/tree/main")).toThrow()
  expect(() => parseGitHubRemote("https://github.com/owner/repo/blob/main/file.ts")).toThrow()
})
