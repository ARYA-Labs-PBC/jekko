import { createSignal, Show } from "solid-js"
import type { JSX } from "solid-js"
import type { SessionMessageAssistantTool } from "@jekko-ai/sdk/v2"
import { SplitBorder } from "@tui/component/border"
import { TerminalText } from "@tui/routes/session/session-renderers"
import { useTheme } from "@tui/context/theme"

export function BlockTool(props: {
  title: string
  children: JSX.Element
  onClick?: () => void
  spinner?: boolean
}) {
  const { theme } = useTheme()
  const [hover, setHover] = createSignal(false)

  return (
    <box
      border={["left"]}
      paddingTop={1}
      paddingBottom={1}
      paddingLeft={2}
      marginTop={1}
      gap={1}
      backgroundColor={hover() ? theme.backgroundMenu : theme.backgroundPanel}
      customBorderChars={SplitBorder.customBorderChars}
      borderColor={theme.background}
      onMouseOver={() => props.onClick && setHover(true)}
      onMouseOut={() => setHover(false)}
      onMouseUp={() => props.onClick?.()}
    >
      <Show when={props.spinner}>
        <text fg={theme.textMuted}>{props.title.replace(/^# /, "")}</text>
      </Show>
      <Show when={!props.spinner}>
        <text paddingLeft={3} fg={theme.textMuted}>
          {props.title}
        </text>
      </Show>
      {props.children}
    </box>
  )
}

export function AssistantTool(props: { part: SessionMessageAssistantTool }) {
  const { theme } = useTheme()
  const inputText = () => JSON.stringify(props.part.state.input, null, 2)
  const outputText = () => {
    if (props.part.state.status !== "completed") return ""
    return JSON.stringify(props.part.state.structured ?? props.part.state.content, null, 2)
  }
  const errorText = () => (props.part.state.status === "error" ? props.part.state.error : undefined)

  return (
    <BlockTool title={`# ${props.part.name}`} spinner={props.part.state.status === "running"}>
      <Show when={inputText()}>
        <box gap={1} marginTop={1}>
          <text fg={theme.textMuted}>Input</text>
          <TerminalText text={inputText()} muted={false} />
        </box>
      </Show>
      <Show when={outputText()}>
        <box gap={1} marginTop={1}>
          <text fg={theme.textMuted}>Output</text>
          <TerminalText text={outputText()} muted={false} />
        </box>
      </Show>
      <Show when={errorText()}>
        <text fg={theme.error}>{errorText()}</text>
      </Show>
    </BlockTool>
  )
}
