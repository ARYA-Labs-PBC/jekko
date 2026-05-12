// Concept memory primitives. Concepts are durable, recallable units the
// incubator builds on across iterations — distinct from per-task memory in
// that they are named (`concept_id`) and linked into a lineage graph via
// `derived_from`.
//
// PR4 ships pure functions over a lightweight `ConceptRecord` shape plus an
// invariant-checker for the lineage graph. Persistence lives in
// `daemon-forever-store.ts` (Drizzle bridge over `daemon_concept` +
// `daemon_concept_link`). Daemon-incubator pass handlers call these directly
// to define / recall / build-on concepts at pass boundaries.

export type ConceptID = string

export type ConceptRecord = {
  conceptID: ConceptID
  definition: string
  /** Parent concepts this one builds on. */
  derivedFrom: ConceptID[]
  /** Artifact / pass / receipt ids that justify the concept. */
  proofRefs: string[]
  confidence: number
  /** When non-null, the concept is invalidated and should not be recalled. */
  invalidatedAt?: number
  invalidatedReason?: string
}

export type DefineConceptInput = {
  conceptID: ConceptID
  definition: string
  derivedFrom?: readonly ConceptID[]
  proofRefs?: readonly string[]
  confidence?: number
}

export type BuildOnInput = {
  parent: ConceptID
  newID: ConceptID
  delta: string
  proofRefs?: readonly string[]
  confidence?: number
}

export class ConceptError extends Error {
  readonly _tag = "ConceptError"
}

export type ConceptStore = {
  defineConcept: (input: DefineConceptInput) => ConceptRecord
  recallConcept: (id: ConceptID) => ConceptRecord | undefined
  recallByPrefix: (prefix: string) => ConceptRecord[]
  buildOn: (input: BuildOnInput) => ConceptRecord
  invalidate: (id: ConceptID, reason: string, at?: number) => ConceptRecord | undefined
  lineage: (id: ConceptID) => ConceptID[]
  size: () => number
}

/**
 * Pure in-memory concept store. The daemon Effect Service backs `defineConcept`
 * etc. with a SQLite-persisted shadow, but the in-memory variant is what unit
 * tests exercise and what `daemon-task-memory` consumes to render the
 * `<concepts-in-scope>` block.
 */
export function makeConceptStore(seed?: readonly ConceptRecord[]): ConceptStore {
  const records = new Map<ConceptID, ConceptRecord>()
  if (seed) {
    for (const s of seed) {
      records.set(s.conceptID, normalizeRecord(s))
    }
  }

  function ensureNoCycle(newID: ConceptID, parents: ConceptID[]): void {
    // Walk parents transitively; if we hit `newID`, the new concept would
    // close a cycle. We disallow that — concepts must form a DAG.
    const visit = new Set<ConceptID>()
    const stack: ConceptID[] = [...parents]
    while (stack.length > 0) {
      const id = stack.pop() as ConceptID
      if (id === newID) {
        throw new ConceptError(`concept lineage cycle: ${newID} -> ... -> ${parents.join(",")}`)
      }
      if (visit.has(id)) continue
      visit.add(id)
      const r = records.get(id)
      if (r) stack.push(...r.derivedFrom)
    }
  }

  return {
    defineConcept(input) {
      if (!input.conceptID || !input.conceptID.trim()) {
        throw new ConceptError("conceptID required")
      }
      const derivedFrom = uniqueList(input.derivedFrom)
      // Validate every parent exists. This is strict on purpose — referencing
      // an unknown concept is almost always a typo on the agent side.
      for (const parent of derivedFrom) {
        if (!records.has(parent)) {
          throw new ConceptError(`unknown parent concept: ${parent}`)
        }
      }
      ensureNoCycle(input.conceptID, derivedFrom)
      const record: ConceptRecord = normalizeRecord({
        conceptID: input.conceptID,
        definition: input.definition,
        derivedFrom,
        proofRefs: uniqueList(input.proofRefs),
        confidence: input.confidence ?? 0.5,
      })
      records.set(record.conceptID, record)
      return record
    },
    recallConcept(id) {
      const record = records.get(id)
      if (!record) return undefined
      if (record.invalidatedAt !== undefined) return undefined
      return record
    },
    recallByPrefix(prefix) {
      const out: ConceptRecord[] = []
      for (const record of records.values()) {
        if (record.invalidatedAt !== undefined) continue
        if (record.conceptID.startsWith(prefix)) out.push(record)
      }
      out.sort((a, b) => a.conceptID.localeCompare(b.conceptID))
      return out
    },
    buildOn(input) {
      if (!records.has(input.parent)) {
        throw new ConceptError(`unknown parent concept: ${input.parent}`)
      }
      return this.defineConcept({
        conceptID: input.newID,
        definition: input.delta,
        derivedFrom: [input.parent],
        proofRefs: input.proofRefs,
        confidence: input.confidence,
      })
    },
    invalidate(id, reason, at) {
      const record = records.get(id)
      if (!record) return undefined
      const updated: ConceptRecord = {
        ...record,
        invalidatedAt: at ?? Date.now(),
        invalidatedReason: reason,
      }
      records.set(id, updated)
      return updated
    },
    lineage(id) {
      const out: ConceptID[] = []
      const visit = new Set<ConceptID>()
      const stack: ConceptID[] = [id]
      while (stack.length > 0) {
        const next = stack.pop() as ConceptID
        if (visit.has(next)) continue
        visit.add(next)
        out.push(next)
        const r = records.get(next)
        if (r) stack.push(...r.derivedFrom)
      }
      return out
    },
    size() {
      return records.size
    },
  }
}

function normalizeRecord(record: ConceptRecord): ConceptRecord {
  return {
    conceptID: record.conceptID,
    definition: record.definition,
    derivedFrom: uniqueList(record.derivedFrom),
    proofRefs: uniqueList(record.proofRefs),
    confidence: clamp01(record.confidence ?? 0.5),
    invalidatedAt: record.invalidatedAt,
    invalidatedReason: record.invalidatedReason,
  }
}

function uniqueList<T>(input?: readonly T[]): T[] {
  if (!input || input.length === 0) return []
  return Array.from(new Set(input))
}

function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0
  if (value < 0) return 0
  if (value > 1) return 1
  return value
}

export * as DaemonConceptMemory from "./daemon-concept-memory"
