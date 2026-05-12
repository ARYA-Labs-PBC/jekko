import { createSessionBodyState } from "./session-body-core"
import { SessionBodyView } from "./session-body-view"

export function Session() {
  const state = createSessionBodyState()
  return <SessionBodyView {...state} />
}

