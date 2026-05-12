import { createSignal, For, Match, Show, Switch } from "solid-js"
import { RGBA } from "@opentui/core"
import { SplitBorder } from "@tui/component/border"
import { Prompt } from "@tui/component/prompt"
import { DialogConfirm } from "@tui/ui/dialog-confirm"
import { Toast } from "../../ui/toast"
import { PermissionPrompt } from "./permission"
import { QuestionPrompt } from "./question"
import { DaemonBanner } from "./daemon-banner"
import { SubagentFooter } from "./subagent-footer.tsx"
import { Sidebar } from "./sidebar"
import { DialogMessage } from "./dialog-message"
import { TuiPluginRuntime } from "@/cli/cmd/tui/plugin/runtime"
import { AssistantMessage as AssistantMessageView, UserMessage as UserMessageView } from "./session-renderers"
import { useCommandDialog } from "@tui/component/dialog-command"
import { useDialog } from "../../ui/dialog"

type Props = any

export function SessionView(props: Props) {
  const command = useCommandDialog()
  const dialog = useDialog()

  return (
    <props.context.Provider
      value={{
        get width() {
          return props.contentWidth()
        },
        sessionID: props.route.sessionID,
        conceal: props.conceal,
        showThinking: props.showThinking,
        showTimestamps: props.showTimestamps,
        showDetails: props.showDetails,
        showGenericToolOutput: props.showGenericToolOutput,
        diffWrapMode: props.diffWrapMode,
        providers: props.providers,
        sync: props.sync,
        tui: props.tuiConfig,
      }}
    >
      <box flexDirection="row">
        <box flexGrow={1} paddingBottom={1} paddingLeft={2} paddingRight={2} gap={1}>
          <Show when={props.session()}>
            <scrollbox
              ref={props.setScroll}
              viewportOptions={{
                paddingRight: props.showScrollbar() ? 1 : 0,
              }}
              verticalScrollbarOptions={{
                paddingLeft: 1,
                visible: props.showScrollbar(),
                trackOptions: {
                  backgroundColor: props.theme.backgroundElement,
                  foregroundColor: props.theme.border,
                },
              }}
              stickyScroll={true}
              stickyStart="bottom"
              flexGrow={1}
              scrollAcceleration={props.scrollAcceleration()}
            >
              <box height={1} />
              <For each={props.messages()}>
                {(message, index) => (
                  <Switch>
                    <Match when={message.id === props.revert()?.messageID}>
                      {(function () {
                        const [hover, setHover] = createSignal(false)
                        const handleUnrevert = async () => {
                          const confirmed = await DialogConfirm.show(
                            dialog,
                            "Confirm Redo",
                            "Are you sure you want to restore the reverted messages?",
                          )
                          if (confirmed) command.trigger("session.redo")
                        }

                        return (
                          <box
                            onMouseOver={() => setHover(true)}
                            onMouseOut={() => setHover(false)}
                            onMouseUp={handleUnrevert}
                            marginTop={1}
                            flexShrink={0}
                            border={["left"]}
                            customBorderChars={SplitBorder.customBorderChars}
                            borderColor={props.theme.backgroundPanel}
                          >
                            <box
                              paddingTop={1}
                              paddingBottom={1}
                              paddingLeft={2}
                              backgroundColor={hover() ? props.theme.backgroundElement : props.theme.backgroundPanel}
                            >
                              <text fg={props.theme.textMuted}>{props.revert()!.reverted.length} message reverted</text>
                              <text fg={props.theme.textMuted}>
                                <span style={{ fg: props.theme.text }}>{props.keybind.print("messages_redo")}</span> or
                                /redo to restore
                              </text>
                              <Show when={props.revert()!.diffFiles?.length}>
                                <box marginTop={1}>
                                  <For each={props.revert()!.diffFiles}>
                                    {(file) => (
                                      <text fg={props.theme.text}>
                                        {file.filename}
                                        <Show when={file.additions > 0}>
                                          <span style={{ fg: props.theme.diffAdded }}> +{file.additions}</span>
                                        </Show>
                                        <Show when={file.deletions > 0}>
                                          <span style={{ fg: props.theme.diffRemoved }}> -{file.deletions}</span>
                                        </Show>
                                      </text>
                                    )}
                                  </For>
                                </box>
                              </Show>
                            </box>
                          </box>
                        )
                      })()}
                    </Match>
                    <Match when={props.revert()?.messageID && message.id >= props.revert()!.messageID}>
                      <></>
                    </Match>
                    <Match when={message.role === "user"}>
                      {message.role === "user" ? (
                        <UserMessageView
                          index={index()}
                          onMouseUp={() => {
                            if (props.renderer.getSelection()?.getSelectedText()) return
                            dialog.replace(() => (
                              <DialogMessage
                                messageID={message.id}
                                sessionID={props.route.sessionID}
                                setPrompt={(promptInfo) => props.prompt?.set(promptInfo)}
                              />
                            ))
                          }}
                          message={message}
                          parts={props.sync.data.part[message.id] ?? []}
                          pending={props.pending()}
                        />
                      ) : null}
                    </Match>
                    <Match when={message.role === "assistant"}>
                      {message.role === "assistant" ? (
                        <AssistantMessageView
                          last={props.lastAssistant()?.id === message.id}
                          message={message}
                          parts={props.sync.data.part[message.id] ?? []}
                        />
                      ) : null}
                    </Match>
                  </Switch>
                )}
              </For>
            </scrollbox>
            <box flexShrink={0}>
              <Show when={props.daemonRun()}>
                {(run) => <DaemonBanner run={run()} />}
              </Show>
              <Show when={props.permissions().length > 0}>
                <PermissionPrompt request={props.permissions()[0]} />
              </Show>
              <Show when={props.permissions().length === 0 && props.questions().length > 0}>
                <QuestionPrompt request={props.questions()[0]} />
              </Show>
              <Show when={props.session()?.parentID}>
                <SubagentFooter />
              </Show>
              <Show when={props.visible()}>
                <TuiPluginRuntime.Slot
                  name="session_prompt"
                  mode="replace"
                  session_id={props.route.sessionID}
                  visible={props.visible()}
                  disabled={props.disabled()}
                  on_submit={props.toBottom}
                  ref={props.bind}
                >
                  <Prompt
                    visible={props.visible()}
                    ref={props.bind}
                    disabled={props.disabled()}
                    onSubmit={() => {
                      props.toBottom()
                    }}
                    sessionID={props.route.sessionID}
                    right={<TuiPluginRuntime.Slot name="session_prompt_right" session_id={props.route.sessionID} />}
                  />
                </TuiPluginRuntime.Slot>
              </Show>
            </box>
          </Show>
          <Toast />
        </box>
        <Show when={props.sidebarVisible()}>
          <Switch>
            <Match when={props.wide()}>
              <Sidebar sessionID={props.route.sessionID} />
            </Match>
            <Match when={!props.wide()}>
              <box
                position="absolute"
                top={0}
                left={0}
                right={0}
                bottom={0}
                alignItems="flex-end"
                backgroundColor={RGBA.fromInts(0, 0, 0, 70)}
              >
                <Sidebar sessionID={props.route.sessionID} />
              </box>
            </Match>
          </Switch>
        </Show>
      </box>
    </props.context.Provider>
  )
}
