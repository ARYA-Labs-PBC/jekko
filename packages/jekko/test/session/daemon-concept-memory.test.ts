import { describe, expect, test } from "bun:test"
import { DaemonConceptMemory } from "../../src/session/daemon-concept-memory"

describe("DaemonConceptMemory.makeConceptStore", () => {
  test("defineConcept stores a record + recallConcept returns it", () => {
    const store = DaemonConceptMemory.makeConceptStore()
    const c = store.defineConcept({
      conceptID: "domain.auth.session",
      definition: "Auth sessions are JWTs with sliding expiry",
      confidence: 0.9,
    })
    expect(c.conceptID).toBe("domain.auth.session")
    expect(c.confidence).toBe(0.9)
    expect(store.recallConcept("domain.auth.session")?.definition).toContain("JWTs")
  })

  test("rejects defining a concept whose parent does not exist", () => {
    const store = DaemonConceptMemory.makeConceptStore()
    expect(() =>
      store.defineConcept({
        conceptID: "child",
        definition: "x",
        derivedFrom: ["missing"],
      }),
    ).toThrow(/unknown parent/)
  })

  test("buildOn chains a parent without typing derivedFrom by hand", () => {
    const store = DaemonConceptMemory.makeConceptStore()
    store.defineConcept({ conceptID: "p1", definition: "parent" })
    const child = store.buildOn({ parent: "p1", newID: "c1", delta: "child def" })
    expect(child.derivedFrom).toEqual(["p1"])
    expect(child.definition).toBe("child def")
  })

  test("rejects a cycle in the lineage graph", () => {
    const store = DaemonConceptMemory.makeConceptStore()
    store.defineConcept({ conceptID: "a", definition: "" })
    store.defineConcept({ conceptID: "b", definition: "", derivedFrom: ["a"] })
    // attempting to re-define `a` with `b` as parent would close a cycle
    expect(() => store.defineConcept({ conceptID: "a", definition: "", derivedFrom: ["b"] })).toThrow(
      /cycle/,
    )
  })

  test("invalidate hides the concept from recall but preserves the lineage", () => {
    const store = DaemonConceptMemory.makeConceptStore()
    store.defineConcept({ conceptID: "x", definition: "" })
    store.invalidate("x", "superseded")
    expect(store.recallConcept("x")).toBeUndefined()
    expect(store.lineage("x")).toContain("x")
  })

  test("recallByPrefix returns matches sorted by id", () => {
    const store = DaemonConceptMemory.makeConceptStore()
    store.defineConcept({ conceptID: "auth.token", definition: "" })
    store.defineConcept({ conceptID: "auth.session", definition: "" })
    store.defineConcept({ conceptID: "billing.invoice", definition: "" })
    const out = store.recallByPrefix("auth.")
    expect(out.map((r) => r.conceptID)).toEqual(["auth.session", "auth.token"])
  })

  test("lineage walks the transitive parent chain", () => {
    const store = DaemonConceptMemory.makeConceptStore()
    store.defineConcept({ conceptID: "a", definition: "" })
    store.defineConcept({ conceptID: "b", definition: "", derivedFrom: ["a"] })
    store.defineConcept({ conceptID: "c", definition: "", derivedFrom: ["b"] })
    const out = store.lineage("c")
    expect(out).toContain("a")
    expect(out).toContain("b")
    expect(out).toContain("c")
  })

  test("seeding the store with prior records preserves them", () => {
    const store = DaemonConceptMemory.makeConceptStore([
      {
        conceptID: "seeded",
        definition: "from disk",
        derivedFrom: [],
        proofRefs: [],
        confidence: 0.7,
      },
    ])
    expect(store.recallConcept("seeded")?.definition).toBe("from disk")
    expect(store.size()).toBe(1)
  })
})
