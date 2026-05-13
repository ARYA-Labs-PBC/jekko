import { createSessionBodyState } from "./session-body-core"
import { SessionBodyView } from "./session-body-view"

export function Session() {
  const state = createSessionBodyState()
  return <SessionBodyView {...state} />
}

/**
 * Reusable session body component. Mounts the full session pipeline
 * (message renderers, diff/tool/reasoning rendering, daemon banner,
 * permission/question prompts, and the Prompt input) for an arbitrary
 * sessionID — not tied to the active route. Used by the Phase 6 shell
 * activity-feed plugin to drop the session UI into the shell route's
 * CENTER region.
 */
export function SessionBody(props: { sessionID: string }) {
  const state = createSessionBodyState({ sessionID: () => props.sessionID })
  return <SessionBodyView {...state} />
}

