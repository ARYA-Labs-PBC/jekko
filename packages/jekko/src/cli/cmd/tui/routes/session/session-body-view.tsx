import { SessionView } from "./session-view"
import { context } from "./context"
import type { createSessionBodyState } from "./session-body-core"
import { createEffect, createMemo, on } from "solid-js"
import { getRevertDiffFiles } from "../../util/revert-diff"

type SessionBodyState = ReturnType<typeof createSessionBodyState>

export function SessionBodyView(props: SessionBodyState) {
  const revertInfo = createMemo(() => props.session()?.revert)
  const revertMessageID = createMemo(() => revertInfo()?.messageID)

  const revertDiffFiles = createMemo(() => getRevertDiffFiles(revertInfo()?.diff ?? ""))

  const revertRevertedMessages = createMemo(() => {
    const messageID = revertMessageID()
    if (!messageID) return []
    return props.messages().filter((x) => x.id >= messageID && x.role === "user")
  })

  const revert = createMemo(() => {
    const info = revertInfo()
    if (!info) return
    if (!info.messageID) return
    return {
      messageID: info.messageID,
      reverted: revertRevertedMessages(),
      diff: info.diff,
      diffFiles: revertDiffFiles(),
    }
  })

  createEffect(on(() => props.route.sessionID, props.toBottom))

  return (
    <SessionView
      context={context}
      route={props.route}
      session={props.session}
      messages={props.messages}
      permissions={props.permissions}
      questions={props.questions}
      visible={props.visible}
      disabled={props.disabled}
      pending={props.pending}
      lastAssistant={props.lastAssistant}
      revert={revert}
      contentWidth={props.contentWidth}
      setScroll={(r) => {
        props.setScroll(r)
      }}
      showScrollbar={props.showScrollbar}
      theme={props.theme}
      scrollAcceleration={props.scrollAcceleration}
      daemonRun={props.daemonRun}
      sidebarVisible={props.sidebarVisible}
      wide={props.wide}
      keybind={props.keybind}
      prompt={props.prompt}
      bind={props.bind}
      toBottom={props.toBottom}
      renderer={props.renderer}
      conceal={props.conceal}
      showThinking={props.showThinking}
      showTimestamps={props.showTimestamps}
      showDetails={props.showDetails}
      showGenericToolOutput={props.showGenericToolOutput}
      diffWrapMode={props.diffWrapMode}
      providers={props.providers}
      sync={props.sync}
      tuiConfig={props.tuiConfig}
    />
  )
}
