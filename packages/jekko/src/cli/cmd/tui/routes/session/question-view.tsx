import { For, Show } from "solid-js"
import { selectedForeground, tint, useTheme } from "../../context/theme"
import { SplitBorder } from "../../component/border"
import type { QuestionPromptController } from "./question-controller"

export function QuestionPromptView(props: QuestionPromptController) {
  const { theme } = useTheme()

  return (
    <box
      backgroundColor={theme.backgroundPanel}
      border={["left"]}
      borderColor={theme.accent}
      customBorderChars={SplitBorder.customBorderChars}
    >
      <box gap={1} paddingLeft={1} paddingRight={3} paddingTop={1} paddingBottom={1}>
        <Show when={!props.single()}>
          <box flexDirection="row" gap={1} paddingLeft={1}>
            <For each={props.questions()}>
              {(q, index) => {
                const isActive = () => index() === props.store.tab
                const isAnswered = () => (props.store.answers[index()]?.length ?? 0) > 0
                return (
                  <box
                    paddingLeft={1}
                    paddingRight={1}
                    backgroundColor={
                      isActive()
                        ? theme.accent
                        : props.tabHover() === index()
                          ? theme.backgroundElement
                          : theme.backgroundPanel
                    }
                    onMouseOver={() => props.setTabHover(index())}
                    onMouseOut={() => props.setTabHover(null)}
                    onMouseUp={() => props.selectTab(index())}
                  >
                    <text
                      fg={
                        isActive()
                          ? selectedForeground(theme, theme.accent)
                          : isAnswered()
                            ? theme.text
                            : theme.textMuted
                      }
                    >
                      {q.header}
                    </text>
                  </box>
                )
              }}
            </For>
            <box
              paddingLeft={1}
              paddingRight={1}
              backgroundColor={
                props.confirm() ? theme.accent : props.tabHover() === "confirm" ? theme.backgroundElement : theme.backgroundPanel
              }
              onMouseOver={() => props.setTabHover("confirm")}
              onMouseOut={() => props.setTabHover(null)}
              onMouseUp={() => props.selectTab(props.questions().length)}
            >
              <text fg={props.confirm() ? selectedForeground(theme, theme.accent) : theme.textMuted}>Confirm</text>
            </box>
          </box>
        </Show>

        <Show when={!props.confirm()}>
          <box paddingLeft={1} gap={1}>
            <box>
              <text fg={theme.text}>
                {props.question()?.question}
                {props.multi() ? " (select all that apply)" : ""}
              </text>
            </box>
            <box>
              <For each={props.options()}>
                {(opt, i) => {
                  const active = () => i() === props.store.selected
                  const picked = () => props.store.answers[props.store.tab]?.includes(opt.label) ?? false
                  return (
                    <box
                      onMouseOver={() => props.moveTo(i())}
                      onMouseDown={() => props.moveTo(i())}
                      onMouseUp={() => props.selectOption()}
                    >
                      <box flexDirection="row">
                        <box backgroundColor={active() ? theme.backgroundElement : undefined} paddingRight={1}>
                          <text fg={active() ? tint(theme.textMuted, theme.secondary, 0.6) : theme.textMuted}>
                            {`${i() + 1}.`}
                          </text>
                        </box>
                        <box backgroundColor={active() ? theme.backgroundElement : undefined}>
                          <text fg={active() ? theme.secondary : picked() ? theme.success : theme.text}>
                            {props.multi() ? `[${picked() ? "✓" : " "}] ${opt.label}` : opt.label}
                          </text>
                        </box>
                        <Show when={!props.multi()}>
                          <text fg={theme.success}>{picked() ? "✓" : ""}</text>
                        </Show>
                      </box>

                      <box paddingLeft={3}>
                        <text fg={theme.textMuted}>{opt.description}</text>
                      </box>
                    </box>
                  )
                }}
              </For>
              <Show when={props.custom()}>
                <box
                  onMouseOver={() => props.moveTo(props.options().length)}
                  onMouseDown={() => props.moveTo(props.options().length)}
                  onMouseUp={() => props.selectOption()}
                >
                  <box flexDirection="row">
                    <box backgroundColor={props.other() ? theme.backgroundElement : undefined} paddingRight={1}>
                      <text fg={props.other() ? tint(theme.textMuted, theme.secondary, 0.6) : theme.textMuted}>
                        {`${props.options().length + 1}.`}
                      </text>
                    </box>
                    <box backgroundColor={props.other() ? theme.backgroundElement : undefined}>
                      <text fg={props.other() ? theme.secondary : props.customPicked() ? theme.success : theme.text}>
                        {props.multi() ? `[${props.customPicked() ? "✓" : " "}] Type your own answer` : "Type your own answer"}
                      </text>
                    </box>

                    <Show when={!props.multi()}>
                      <text fg={theme.success}>{props.customPicked() ? "✓" : ""}</text>
                    </Show>
                  </box>
                  <Show when={props.store.editing}>
                    <box paddingLeft={3}>
                      <textarea
                        ref={props.bindTextarea}
                        initialValue={props.input()}
                        {...props.textareaProps}
                        minHeight={1}
                        maxHeight={6}
                        textColor={theme.text}
                        focusedTextColor={theme.text}
                        cursorColor={theme.primary}
                        keyBindings={props.bindings()}
                      />
                    </box>
                  </Show>
                  <Show when={!props.store.editing && props.input()}>
                    <box paddingLeft={3}>
                      <text fg={theme.textMuted}>{props.input()}</text>
                    </box>
                  </Show>
                </box>
              </Show>
            </box>
          </box>
        </Show>

        <Show when={props.confirm() && !props.single()}>
          <box paddingLeft={1}>
            <text fg={theme.text}>Review</text>
          </box>
          <For each={props.questions()}>
            {(q, index) => {
              const value = () => props.store.answers[index()]?.join(", ") ?? ""
              const answered = () => Boolean(value())
              return (
                <box paddingLeft={1}>
                  <text>
                    <span style={{ fg: theme.textMuted }}>{q.header}:</span>{" "}
                    <span style={{ fg: answered() ? theme.text : theme.error }}>
                      {answered() ? value() : "(not answered)"}
                    </span>
                  </text>
                </box>
              )
            }}
          </For>
        </Show>
      </box>
      <box
        flexDirection="row"
        flexShrink={0}
        gap={1}
        paddingLeft={2}
        paddingRight={3}
        paddingBottom={1}
        justifyContent="space-between"
      >
        <box flexDirection="row" gap={2}>
          <Show when={!props.single()}>
            <text fg={theme.text}>
              {"⇆"} <span style={{ fg: theme.textMuted }}>tab</span>
            </text>
          </Show>
          <Show when={!props.confirm()}>
            <text fg={theme.text}>
              {"↑↓"} <span style={{ fg: theme.textMuted }}>select</span>
            </text>
          </Show>
          <text fg={theme.text}>
            enter{" "}
            <span style={{ fg: theme.textMuted }}>
              {props.confirm() ? "submit" : props.multi() ? "toggle" : props.single() ? "submit" : "confirm"}
            </span>
          </text>

          <text fg={theme.text}>
            esc <span style={{ fg: theme.textMuted }}>dismiss</span>
          </text>
        </box>
      </box>
    </box>
  )
}
