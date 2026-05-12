// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { useSyncV2 } from "@tui/context/sync"
import { SplitBorder } from "@tui/component/border"
import { Spinner } from "@tui/component/spinner"
import { useTheme } from "@tui/context/theme"
import { useLocal } from "@tui/context/local"
import { useKeyboard, useTerminalDimensions, type JSX } from "@opentui/solid"
import { Locale } from "@/util/locale"
import stripAnsi from "strip-ansi"
import type { SyntaxStyle } from "@opentui/core"
import type {
  SessionMessage,
  SessionMessageAgentSwitched,
  SessionMessageAssistant,
  SessionMessageAssistantReasoning,
  SessionMessageAssistantText,
  SessionMessageAssistantTool,
  SessionMessageCompaction,
  SessionMessageModelSwitched,
  SessionMessageShell,
  SessionMessageSynthetic,
  SessionMessageUser,
} from "@jekko-ai/sdk/v2"
import { createEffect, createMemo, createSignal, For, Match, Show, Switch } from "solid-js"
import { AssistantTool } from "./session-debug-tools"

const id = "internal:session-v2-debug"
const route = "session.v2.messages"

function currentSessionID(api: TuiPluginApi) {
  const current = api.route.current
  if (current.name !== "session") return
  const sessionID = current.params?.sessionID
  return typeof sessionID === "string" ? sessionID : undefined
}

function View(props: { api: TuiPluginApi; sessionID: string }) {
  const sync = useSyncV2()
  const dimensions = useTerminalDimensions()
  const { theme, syntax, subtleSyntax } = useTheme()
  const messages = createMemo(() => sync.data.messages[props.sessionID] ?? [])
  const renderedMessages = createMemo(() => messages().toReversed())
  const lastAssistant = createMemo(() => renderedMessages().findLast((message) => message.type === "assistant"))
  const lastUserCreated = (index: number) =>
    renderedMessages()
      .slice(0, index)
      .findLast((message) => message.type === "user")?.time.created

  createEffect(() => {
    void sync.session.message.sync(props.sessionID)
  })

  useKeyboard((event) => {
    if (event.name !== "escape") return
    event.preventDefault()
    event.stopPropagation()
    props.api.route.navigate("session", { sessionID: props.sessionID })
  })

  return (
    <box width={dimensions().width} height={dimensions().height} backgroundColor={theme.background}>
      <box flexDirection="row">
        <box flexGrow={1} paddingBottom={1} paddingLeft={2} paddingRight={2} gap={1}>
          <scrollbox
            viewportOptions={{ paddingRight: 0 }}
            verticalScrollbarOptions={{ visible: false }}
            stickyScroll={true}
            stickyStart="bottom"
            flexGrow={1}
          >
            <box height={1} />
            <Show when={messages().length === 0}>
              <MissingData label="Messages" detail="No v2 messages loaded from useSyncV2 yet." />
            </Show>
            <For each={renderedMessages()}>
              {(message, index) => (
                <Switch>
                  <Match when={message.type === "user"}>
                    <UserMessage message={message as SessionMessageUser} index={index()} />
                  </Match>
                  <Match when={message.type === "assistant"}>
                    <AssistantMessage
                      message={message as SessionMessageAssistant}
                      last={lastAssistant()?.id === message.id}
                      syntax={syntax()}
                      subtleSyntax={subtleSyntax()}
                      start={lastUserCreated(index())}
                    />
                  </Match>
                  <Match when={message.type === "synthetic"}>
                    <></>
                  </Match>
                  <Match when={message.type === "shell"}>
                    <ShellMessage message={message as SessionMessageShell} />
                  </Match>
                  <Match when={message.type === "compaction"}>
                    <CompactionMessage message={message as SessionMessageCompaction} />
                  </Match>
                  <Match when={message.type === "agent-switched"}>
                    <AgentSwitchedMessage message={message as SessionMessageAgentSwitched} />
                  </Match>
                  <Match when={message.type === "model-switched"}>
                    <ModelSwitchedMessage message={message as SessionMessageModelSwitched} />
                  </Match>
                  <Match when={true}>
                    <UnknownMessage message={message} />
                  </Match>
                </Switch>
              )}
            </For>
          </scrollbox>
          <MissingData
            label="Session prompt, permission prompt, question prompt, sidebar"
            detail="The v2 message endpoint only exposes messages, so these session UI regions cannot be rendered here. Press Esc to return to the live session."
          />
        </box>
      </box>
    </box>
  )
}

function MissingData(props: { label: string; detail: string }) {
  const { theme } = useTheme()
  return (
    <box
      border={["left"]}
      customBorderChars={SplitBorder.customBorderChars}
      borderColor={theme.warning}
      backgroundColor={theme.backgroundPanel}
      paddingLeft={2}
      paddingTop={1}
      paddingBottom={1}
      marginTop={1}
      flexShrink={0}
    >
      <text fg={theme.text}>
        <span style={{ bg: theme.warning, fg: theme.background, bold: true }}> MISSING DATA </span> {props.label}
      </text>
      <text fg={theme.textMuted}>{props.detail}</text>
    </box>
  )
}

function UserMessage(props: { message: SessionMessageUser; index: number }) {
  const { theme } = useTheme()
  const attachments = createMemo(() => [...(props.message.files ?? []), ...(props.message.agents ?? [])])
  return (
    <box
      id={props.message.id}
      border={["left"]}
      borderColor={theme.secondary}
      customBorderChars={SplitBorder.customBorderChars}
      marginTop={props.index === 0 ? 0 : 1}
      flexShrink={0}
      paddingTop={1}
      paddingBottom={1}
      paddingLeft={2}
      backgroundColor={theme.backgroundPanel}
    >
      <text fg={theme.text}>{props.message.text}</text>
      <Show when={attachments().length}>
        <box flexDirection="row" paddingTop={1} gap={1} flexWrap="wrap">
          <For each={props.message.files ?? []}>
            {(file) => (
              <text fg={theme.text}>
                <span style={{ bg: theme.secondary, fg: theme.background }}> {file.mime} </span>
                <span style={{ bg: theme.backgroundElement, fg: theme.textMuted }}> {file.name ?? file.uri} </span>
              </text>
            )}
          </For>
          <For each={props.message.agents ?? []}>
            {(agent) => (
              <text fg={theme.text}>
                <span style={{ bg: theme.accent, fg: theme.background }}> agent </span>
                <span style={{ bg: theme.backgroundElement, fg: theme.textMuted }}> {agent.name} </span>
              </text>
            )}
          </For>
        </box>
      </Show>
    </box>
  )
}

function ShellMessage(props: { message: SessionMessageShell }) {
  const { theme } = useTheme()
  const output = createMemo(() => stripAnsi(props.message.output.trim()))
  const [expanded, setExpanded] = createSignal(false)
  const lines = createMemo(() => output().split("\n"))
  const overflow = createMemo(() => lines().length > 10)
  const limited = createMemo(() => {
    if (expanded() || !overflow()) return output()
    return [...lines().slice(0, 10), "…"].join("\n")
  })
  return (
    <BlockTool
      title="# Shell"
      spinner={!props.message.time.completed}
      onClick={overflow() ? () => setExpanded((prev) => !prev) : undefined}
    >
      <box gap={1}>
        <text fg={theme.text}>$ {props.message.command}</text>
        <Show when={output()}>
          <text fg={theme.text}>{limited()}</text>
        </Show>
        <Show when={overflow()}>
          <text fg={theme.textMuted}>{expanded() ? "Click to collapse" : "Click to expand"}</text>
        </Show>
      </box>
    </BlockTool>
  )
}

function BlockTool(props: {
  title: string
  children: JSX.Element
  onClick?: () => void
  spinner?: boolean
}) {
  const { theme } = useTheme()
  return (
    <box
      border={["left"]}
      paddingTop={1}
      paddingBottom={1}
      paddingLeft={2}
      marginTop={1}
      gap={1}
      backgroundColor={theme.backgroundPanel}
      customBorderChars={SplitBorder.customBorderChars}
      borderColor={theme.background}
      onMouseUp={() => props.onClick?.()}
      flexShrink={0}
    >
      {props.spinner ? (
        <Spinner color={theme.textMuted}>{props.title.replace(/^# /, "")}</Spinner>
      ) : (
        <text paddingLeft={3} fg={theme.textMuted}>
          {props.title}
        </text>
      )}
      {props.children}
    </box>
  )
}

function CompactionMessage(props: { message: SessionMessageCompaction }) {
  const { theme, syntax } = useTheme()
  return (
    <box
      marginTop={1}
      border={["top"]}
      title={props.message.reason === "auto" ? " Auto Compaction " : " Compaction "}
      titleAlignment="center"
      borderColor={theme.borderActive}
      flexShrink={0}
    >
      <Show when={props.message.summary}>
        {(summary) => (
          <box paddingLeft={3} paddingTop={1}>
            <code
              filetype="markdown"
              drawUnstyledText={false}
              streaming={false}
              syntaxStyle={syntax()}
              content={summary().trim()}
              conceal={true}
              fg={theme.text}
            />
          </box>
        )}
      </Show>
    </box>
  )
}

function AgentSwitchedMessage(props: { message: SessionMessageAgentSwitched }) {
  const { theme } = useTheme()
  const local = useLocal()
  return (
    <box paddingLeft={3} marginTop={1} flexShrink={0}>
      <text>
        <span style={{ fg: local.agent.color(props.message.agent) }}>▣ </span>
        <span style={{ fg: theme.textMuted }}>Switched agent to </span>
        <span style={{ fg: theme.text }}>{Locale.titlecase(props.message.agent)}</span>
      </text>
    </box>
  )
}

function ModelSwitchedMessage(props: { message: SessionMessageModelSwitched }) {
  const { theme } = useTheme()
  const model = createMemo(() => {
    const variant = props.message.model.variant ? `/${props.message.model.variant}` : ""
    return `${props.message.model.providerID}/${props.message.model.id}${variant}`
  })
  return (
    <box paddingLeft={3} marginTop={1} flexShrink={0}>
      <text>
        <span style={{ fg: theme.secondary }}>◇ </span>
        <span style={{ fg: theme.textMuted }}>Switched model to </span>
        <span style={{ fg: theme.text }}>{model()}</span>
      </text>
    </box>
  )
}

function UnknownMessage(props: { message: SessionMessage }) {
  return <MissingData label="Unknown message type" detail={JSON.stringify(props.message)} />
}

function AssistantMessage(props: {
  message: SessionMessageAssistant
  last: boolean
  syntax: SyntaxStyle
  subtleSyntax: SyntaxStyle
  start?: number
}) {
  const { theme } = useTheme()
  const local = useLocal()
  const duration = createMemo(() => {
    if (!props.message.time.completed) return 0
    return props.message.time.completed - (props.start ?? props.message.time.created)
  })
  const model = createMemo(() => {
    const variant = props.message.model.variant ? `/${props.message.model.variant}` : ""
    return `${props.message.model.providerID}/${props.message.model.id}${variant}`
  })
  const final = createMemo(() => props.message.finish && !["tool-calls", "unknown"].includes(props.message.finish))
  return (
    <>
      <For each={props.message.content}>
        {(part) => (
          <Switch>
            <Match when={part.type === "text"}>
              <AssistantText part={part as SessionMessageAssistantText} syntax={props.syntax} />
            </Match>
            <Match when={part.type === "reasoning"}>
              <AssistantReasoning part={part as SessionMessageAssistantReasoning} subtleSyntax={props.subtleSyntax} />
            </Match>
            <Match when={part.type === "tool"}>
              <AssistantTool part={part as SessionMessageAssistantTool} />
            </Match>
          </Switch>
        )}
      </For>
      <Show when={props.message.content.length === 0}>
        <MissingData label="Assistant content" detail={`Assistant message ${props.message.id} has no content items.`} />
      </Show>
      <Show when={props.message.error}>
        <box
          border={["left"]}
          paddingTop={1}
          paddingBottom={1}
          paddingLeft={2}
          marginTop={1}
          backgroundColor={theme.backgroundPanel}
          customBorderChars={SplitBorder.customBorderChars}
          borderColor={theme.error}
          flexShrink={0}
        >
          <text fg={theme.textMuted}>{props.message.error}</text>
        </box>
      </Show>
      <Show when={props.last || final() || props.message.error}>
        <box paddingLeft={3} flexShrink={0}>
          <text marginTop={1}>
            <span style={{ fg: local.agent.color(props.message.agent) }}>▣ </span>
            <span style={{ fg: theme.text }}>{Locale.titlecase(props.message.agent)}</span>
            <span style={{ fg: theme.textMuted }}> · {model()}</span>
            <Show when={duration()}>
              <span style={{ fg: theme.textMuted }}> · {Locale.duration(duration())}</span>
            </Show>
          </text>
        </box>
      </Show>
    </>
  )
}

function AssistantText(props: { part: SessionMessageAssistantText; syntax: SyntaxStyle }) {
  const { theme } = useTheme()
  return (
    <Show when={props.part.text.trim()}>
      <box paddingLeft={3} marginTop={1} flexShrink={0} id="text">
        <code
          filetype="markdown"
          drawUnstyledText={false}
          streaming={true}
          syntaxStyle={props.syntax}
          content={props.part.text.trim()}
          conceal={true}
          fg={theme.text}
        />
      </box>
    </Show>
  )
}

function AssistantReasoning(props: { part: SessionMessageAssistantReasoning; subtleSyntax: SyntaxStyle }) {
  const { theme } = useTheme()
  const content = createMemo(() => props.part.text.replace("[REDACTED]", "").trim())
  return (
    <Show when={content()}>
      <box
        paddingLeft={2}
        marginTop={1}
        flexDirection="column"
        border={["left"]}
        customBorderChars={SplitBorder.customBorderChars}
        borderColor={theme.backgroundElement}
        flexShrink={0}
      >
        <code
          filetype="markdown"
          drawUnstyledText={false}
          streaming={true}
          syntaxStyle={props.subtleSyntax}
          content={"_Thinking:_ " + content()}
          conceal={true}
          fg={theme.textMuted}
        />
      </box>
    </Show>
  )
}

const tui: TuiPlugin = async (api) => {
  api.route.register([
    {
      name: route,
      render(input) {
        const sessionID = input.params?.sessionID
        if (typeof sessionID !== "string") {
          return <text fg={api.theme.current.error}>Missing sessionID</text>
        }
        return <View api={api} sessionID={sessionID} />
      },
    },
  ])

  api.command.register(() => [
    {
      title: "View v2 session messages",
      value: route,
      category: "Debug",
      suggested: api.route.current.name === "session",
      enabled: api.route.current.name === "session",
      onSelect() {
        const sessionID = currentSessionID(api)
        if (!sessionID) return
        api.route.navigate(route, { sessionID })
        api.ui.dialog.clear()
      },
    },
  ])
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
