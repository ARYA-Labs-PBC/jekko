// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import { BoxRenderable, RGBA, TextareaRenderable, MouseEvent, PasteEvent, decodePasteBytes } from "@opentui/core"
import { createEffect, createMemo, onMount, createSignal, onCleanup, on, Show, Switch, Match } from "solid-js"
import "opentui-spinner/solid"
import path from "path"
import { fileURLToPath } from "url"
import { Filesystem } from "@/util/filesystem"
import { useLocal } from "@tui/context/local"
import { tint, useTheme } from "@tui/context/theme"
import { EmptyBorder, SplitBorder } from "@tui/component/border"
import { Spinner } from "@tui/component/spinner"
import { useSDK } from "@tui/context/sdk"
import { useRoute } from "@tui/context/route"
import { useProject } from "@tui/context/project"
import { useSync } from "@tui/context/sync"
import { useEvent } from "@tui/context/event"
import { editorSelectionKey, useEditorContext, type EditorSelection } from "@tui/context/editor"
import { MessageID, PartID } from "@/session/schema"
import { createStore, produce, unwrap } from "solid-js/store"
import { useKeybind } from "@tui/context/keybind"
import { usePromptHistory, type PromptInfo } from "./history"
import { computePromptUsage } from "./usage"
import { computePromptTraits } from "./traits"
import { assign } from "./part"
import { usePromptStash } from "./stash"
import { DialogStash } from "../dialog-stash"
import { type AutocompleteRef, Autocomplete } from "./autocomplete"
import { useCommandDialog } from "../dialog-command"
import { useRenderer, useTerminalDimensions, type JSX } from "@opentui/solid"
import * as Editor from "@tui/util/editor"
import { useExit } from "../../context/exit"
import * as Clipboard from "../../util/clipboard"
import type { FilePart, UserMessage } from "@jekko-ai/sdk/v2"
import { TuiEvent } from "../../event"
import { detectZyal, scanZyalEnvelope, type ZyalDetection } from "@/agent-script/activation"
import { isZyalFlashSourceActive, setZyalFlashSource, textHasZyalSentinel, useZyalFlash } from "@tui/context/zyal-flash"
import { tokenizeYaml } from "@tui/util/yaml-tokenize"
import { iife } from "@/util/iife"
import { Locale } from "@/util/locale"
import { formatDuration } from "@/util/format"
import { createColors, createFrames } from "../../ui/spinner.ts"
import { useDialog } from "@tui/ui/dialog"
import { DialogProvider as DialogProviderConnect } from "../dialog-provider"
import { DialogAlert } from "../../ui/dialog-alert"
import { DialogConfirm } from "../../ui/dialog-confirm"
import { useToast } from "../../ui/toast"
import { useKV } from "../../context/kv"
import { createFadeIn } from "../../util/signal"
import { useTextareaKeybindings } from "../textarea-keybindings"
import { DialogSkill } from "../dialog-skill"
import { openWorkspaceSelect, warpWorkspaceSession, type WorkspaceSelection } from "../dialog-workspace-create"
import { DialogWorkspaceUnavailable } from "../dialog-workspace-unavailable"
import { useArgs } from "@tui/context/args"
import { Flag } from "@jekko-ai/core/flag/flag"
import { WorkspaceLabel, type WorkspaceStatus } from "../workspace-label"
import type { ZyalPreview } from "@/agent-script/schema"

export type PromptProps = {
  sessionID?: string
  workspaceID?: string
  visible?: boolean
  disabled?: boolean
  onSubmit?: () => void
  ref?: (ref: PromptRef | undefined) => void
  hint?: JSX.Element
  right?: JSX.Element
  showSuggestion?: boolean
  promptSuggestions?: {
    normal?: string[]
    shell?: string[]
  }
}

export type PromptRef = {
  focused: boolean
  current: PromptInfo
  set(prompt: PromptInfo): void
  reset(): void
  blur(): void
  focus(): void
  submit(): void
}

type PromptZyalDraft = ZyalDetection | { readonly kind: "draft"; readonly id?: string; readonly complete: boolean }

const money = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
})

function randomIndex(count: number) {
  if (count <= 0) return 0
  return Math.floor(Math.random() * count)
}

function fadeColor(color: RGBA, alpha: number) {
  return RGBA.fromValues(color.r, color.g, color.b, color.a * alpha)
}

function hasEditorRangeSelection(selection: EditorSelection["ranges"][number]) {
  return (
    selection.selection.start.line !== selection.selection.end.line ||
    selection.selection.start.character !== selection.selection.end.character
  )
}

function getEditorRangeLabel(selection: EditorSelection["ranges"][number]) {
  if (!hasEditorRangeSelection(selection)) return
  if (selection.selection.start.line === selection.selection.end.line) return `#${selection.selection.start.line}`
  return `#${selection.selection.start.line}-${selection.selection.end.line}`
}

function zyalPreviewValue(value: string | number | undefined | null) {
  if (value === undefined || value === null || value === "") return "(none)"
  return String(value)
}

function zyalRunCard(preview: ZyalPreview) {
  const incubatorLines = preview.incubator_enabled
    ? [
        `Incubator passes: ${preview.incubator_passes.length}`,
        `Incubator max passes: ${zyalPreviewValue(preview.incubator_budget?.max_passes_per_task)}`,
        `Promotion: ${zyalPreviewValue(preview.promotion_threshold)}`,
        `Exclude: ${zyalPreviewValue(preview.exclusion_summary)}`,
        `Cleanup: ${zyalPreviewValue(preview.cleanup_summary)}`,
        `Readiness: ${zyalPreviewValue(preview.readiness_summary)}`,
        `Incubator write risks: ${preview.incubator_risks.join(", ") || "(none)"}`,
      ]
    : []

  return [
    "Run Card",
    `Objective: ${preview.objective}`,
    `Status: ${preview.armed ? "armed sentinel present" : "preview draft"}`,
    `Loop: ${preview.loop_policy ?? "forever"}`,
    `Stop: ${preview.stop_checks.join(", ") || "(none)"}`,
    `Checkpoint: ${preview.checkpoint_checks.join(", ") || "(none)"}`,
    `Workers: ${preview.worker_count}`,
    `Fleet/Jnoccio: ${zyalPreviewValue(preview.fleet_summary)}`,
    `Research: ${preview.research_enabled ? zyalPreviewValue(preview.research_summary) : "(none)"}`,
    ...(preview.research_enabled
      ? [
          `Research providers: ${zyalPreviewValue(preview.research_provider_summary)}`,
          `Research extraction: ${zyalPreviewValue(preview.research_extraction_summary)}`,
          `Research evidence: ${zyalPreviewValue(preview.research_evidence_summary)}`,
          `Research safety: ${zyalPreviewValue(preview.research_safety_summary)}`,
          `Research budgets: ${zyalPreviewValue(preview.research_budget_summary)}`,
        ]
      : []),
    `Incubator: ${preview.incubator_enabled ? "enabled" : "disabled"}`,
    ...incubatorLines,
    `Arming: ${preview.arming_enabled ? zyalPreviewValue(preview.arming_summary) : "(none)"}`,
    `Capabilities: ${preview.capabilities_rule_count > 0 ? zyalPreviewValue(preview.capabilities_summary) : "(none)"}`,
    `Budgets: ${preview.budgets_enabled ? zyalPreviewValue(preview.budgets_summary) : "(none)"}`,
    `Taint: ${preview.taint_enabled ? zyalPreviewValue(preview.taint_summary) : "(none)"}`,
    `Unsupported feature policy: ${
      preview.unsupported_feature_policy_enabled ? zyalPreviewValue(preview.unsupported_feature_policy_summary) : "(none)"
    }`,
    `Risks: ${preview.risks.join(", ") || "(none)"}`,
    "Preview-only: arming hash, nonce, and accepted-origin policies are not enforced by the start API yet.",
    "Preview-only: taint is parsed and summarized here, but daemon execution does not enforce taint flow yet.",
  ].join("\n")
}

function formatEditorContext(selection: EditorSelection) {
  const selected = selection.ranges.filter(hasEditorRangeSelection)
  if (selected.length === 0)
    return `<system-reminder>Note: The user opened the file "${selection.filePath}". This may or may not be relevant to the current task.</system-reminder>\n`

  const ranges = selected.map((range, index) => {
    const prefix = selected.length > 1 ? `Selection ${index + 1}: ` : ""
    return `Note: The user selected ${prefix}${getEditorRangeLabel(range)} from "${selection.filePath}". \`\`\`${range.text}\`\`\`\n\n`
  })

  return `<system-reminder>${ranges.join("\n")} This may or may not be relevant to the current task.</system-reminder>\n`
}

let stashed: { prompt: PromptInfo; cursor: number } | undefined

export function Prompt(props: PromptProps) {
  let input: TextareaRenderable
  let anchor: BoxRenderable
  let autocomplete: AutocompleteRef

  const keybind = useKeybind()
  const local = useLocal()
  const args = useArgs()
  const sdk = useSDK()
  const editor = useEditorContext()
  const route = useRoute()
  const project = useProject()
  const sync = useSync()
  const dialog = useDialog()
  const toast = useToast()
  // Read flash so this memo invalidates when the daemon flash flips. The
  // signal call is what wires reactivity; we don't need the value itself,
  // we just need the memo to recompute.
  const zyalFlashSignal = useZyalFlash()
  const status = createMemo(() => {
    const raw = sync.data.session_status?.[props.sessionID ?? ""] ?? { type: "idle" }
    if (raw.type !== "idle") return raw
    // ZYAL daemon may be working in a forked active session that differs
    // from this prompt's `sessionID`. The session_status entry on the root
    // therefore stays "idle" and the spinner disappears even though
    // workers are churning. When the daemon flash is on for this session,
    // surface `busy` so the chat shows the processing animation.
    void zyalFlashSignal()
    if (isZyalFlashSourceActive("session:daemon") || isZyalFlashSourceActive("prompt:submitted")) {
      return { type: "busy" } as const
    }
    return raw
  })
  const history = usePromptHistory()
  const stash = usePromptStash()
  const command = useCommandDialog()
  const renderer = useRenderer()
  const dimensions = useTerminalDimensions()
  const { theme, syntax, yamlSyntax, setOverlay } = useTheme()
  const kv = useKV()
  const animationsEnabled = createMemo(() => kv.get("animations_enabled", true))
  const list = createMemo(() => props.promptSuggestions?.normal ?? [])
  const shell = createMemo(() => props.promptSuggestions?.shell ?? [])
  const fileContextEnabled = createMemo(() => kv.get("file_context_enabled", true))
  const [dismissedEditorSelectionKey, setDismissedEditorSelectionKey] = createSignal<string>()
  const editorContext = createMemo(() => {
    const selection = fileContextEnabled() ? editor.selection() : undefined
    if (!selection) return
    return editorSelectionKey(selection) === dismissedEditorSelectionKey() ? undefined : selection
  })
  const editorPath = createMemo(() => editorContext()?.filePath)
  const editorSelectionLabel = createMemo(() => {
    const ranges = editorContext()?.ranges
    if (!ranges) return
    const first = ranges.find(hasEditorRangeSelection) ?? ranges[0]
    if (!first) return
    return [getEditorRangeLabel(first), ranges.length > 1 ? `+${ranges.length - 1}` : undefined]
      .filter(Boolean)
      .join(" ")
  })
  const editorFileLabel = createMemo(() => {
    const value = editorPath()
    if (!value) return
    const filename = path.basename(value)
    const file = /^index\.[^./]+$/.test(filename)
      ? [path.basename(path.dirname(value)), filename].filter(Boolean).join("/")
      : filename
    return `${file.split(path.sep).join("/")}${editorSelectionLabel() ?? ""}`
  })
  const editorFileLabelDisplay = createMemo(() => {
    const file = editorFileLabel()
    if (!file) return
    return Locale.truncateMiddle(file, Math.max(12, Math.min(48, Math.floor(dimensions().width / 3))))
  })
  const editorContextLabelState = createMemo(() => editor.labelState())
  const [auto, setAuto] = createSignal<AutocompleteRef>()
  const [workspaceSelection, setWorkspaceSelection] = createSignal<WorkspaceSelection>()
  const [workspaceCreating, setWorkspaceCreating] = createSignal(false)
  const [workspaceCreatingDots, setWorkspaceCreatingDots] = createSignal(3)
  const [warpNotice, setWarpNotice] = createSignal<string>()
  const currentProviderLabel = createMemo(() => local.model.parsed().provider)
  const hasRightContent = createMemo(() => Boolean(props.right))
  const defaultWorkspaceID = createMemo(() => props.workspaceID ?? project.workspace.current())

  function selectWorkspace(selection: WorkspaceSelection | undefined) {
    setWorkspaceSelection(selection)
  }

  function setCreatingWorkspace(creating: boolean) {
    setWorkspaceCreating(creating)
  }

  function showWarpNotice(name: string) {
    setWarpNotice(`Warped to ${name}`)
    setTimeout(() => setWarpNotice(undefined), 4000)
  }

  async function createWorkspace(selection: Extract<WorkspaceSelection, { type: "new" }>) {
    setCreatingWorkspace(true)
    const result = await sdk.client.experimental.workspace
      .create({ type: selection.workspaceType, branch: null })
      .catch(() => undefined)
    if (result == undefined || result.error || !result.data) {
      selectWorkspace(undefined)
      setCreatingWorkspace(false)
      toast.show({
        message: "Creating workspace failed",
        variant: "error",
      })
      return
    }

    await project.workspace.sync()
    const workspace = result.data
    selectWorkspace({
      type: "existing",
      workspaceID: workspace.id,
      workspaceType: workspace.type,
      workspaceName: workspace.name,
    })
    setCreatingWorkspace(false)
    return workspace
  }

  async function warpSession(selection: WorkspaceSelection) {
    if (!props.sessionID) {
      selectWorkspace(selection)
      dialog.clear()
      if (selection.type === "new") void createWorkspace(selection)
      return
    }
    selectWorkspace(selection)
    dialog.clear()

    const workspace =
      selection.type === "none"
        ? { id: null, name: "local project" }
        : selection.type === "existing"
          ? { id: selection.workspaceID, name: selection.workspaceName }
          : await createWorkspace(selection)
    if (!workspace) return

    const warped = await warpWorkspaceSession({
      dialog,
      sdk,
      sync,
      project,
      toast,
      workspaceID: workspace.id,
      sessionID: props.sessionID,
    })
    if (warped) showWarpNotice(workspace.name)
  }

  createEffect(() => {
    if (!workspaceCreating()) {
      setWorkspaceCreatingDots(3)
      return
    }
    const timer = setInterval(() => setWorkspaceCreatingDots((dots) => (dots % 3) + 1), 1000)
    onCleanup(() => clearInterval(timer))
  })

  function promptModelWarning() {
    toast.show({
      variant: "warning",
      message: "Connect a provider to send prompts",
      duration: 3000,
    })
    if (sync.data.provider.length === 0) {
      dialog.replace(() => <DialogProviderConnect />)
    }
  }

  function dismissEditorContext() {
    setDismissedEditorSelectionKey(editorSelectionKey(editorContext()))
    editor.clearSelection()
  }

  const textareaKeybindings = useTextareaKeybindings()

  const fileStyleId = syntax().getStyleId("extmark.file")!
  const agentStyleId = syntax().getStyleId("extmark.agent")!
  const pasteStyleId = syntax().getStyleId("extmark.paste")!
  let promptPartTypeId = 0
  let yamlTokenTypeId = 0
  const event = useEvent()

  event.on(TuiEvent.PromptAppend.type, (evt) => {
    if (!input || input.isDestroyed) return
    input.insertText(evt.properties.text)
    setTimeout(() => {
      // setTimeout is a alternative and needs to be addressed properly
      if (!input || input.isDestroyed) return
      input.getLayoutNode().markDirty()
      input.gotoBufferEnd()
      renderer.requestRender()
    }, 0)
  })

  createEffect(() => {
    if (!input || input.isDestroyed) return
    if (props.disabled) input.cursorColor = theme.backgroundElement
    if (!props.disabled) input.cursorColor = theme.text
  })

  type LastUserMessageState = { kind: "missing" } | { kind: "present"; message: UserMessage }

  const lastUserMessage = createMemo<LastUserMessageState>(() => {
    if (!props.sessionID) return { kind: "missing" }
    const messages = sync.data.message[props.sessionID]
    const message = messages?.findLast((m): m is UserMessage => m.role === "user")
    return message ? { kind: "present", message } : { kind: "missing" }
  })

  const usage = createMemo(() => {
    const sessionID = props.sessionID
    const messages = sessionID ? sync.data.message[sessionID] ?? [] : []
    return computePromptUsage(sessionID, messages, sync.data.provider)
  })
  const usageLine = createMemo<JSX.Element>(() => {
    const state = usage()
    switch (state.kind) {
      case "ready": {
        const context = state.contextLimit
          ? `${Locale.number(state.tokens)} (${Math.round((state.tokens / state.contextLimit) * 100)}%)`
          : Locale.number(state.tokens)
        const cost = state.cost > 0 ? money.format(state.cost) : undefined
        return (
          <text fg={theme.textMuted} wrapMode="none">
            {[context, cost].filter(Boolean).join(" · ")}
          </text>
        )
      }
      case "missing-session":
      case "missing-assistant":
      case "zero-tokens":
        return (
          <text fg={theme.text}>
            {keybind.print("agent_cycle")} <span style={{ fg: theme.textMuted }}>{state.reason}</span>
          </text>
        )
      default: {
        const exhaustive: never = state
        return exhaustive
      }
    }
  })

  const [store, setStore] = createStore<{
    prompt: PromptInfo
    mode: "normal" | "shell"
    extmarkToPartIndex: Map<number, number>
    interrupt: number
    default_value: number
  }>({
    default_value: randomIndex(list().length),
    prompt: {
      input: "",
      parts: [],
    },
    mode: "normal",
    extmarkToPartIndex: new Map(),
    interrupt: 0,
  })

  const zyalEnvelope = createMemo(() => scanZyalEnvelope(store.prompt.input))
  const [daemonDraft, setDaemonDraft] = createSignal<PromptZyalDraft>({ kind: "none" })
  const isZyalInput = createMemo(() => zyalEnvelope().kind !== "none" || textHasZyalSentinel(store.prompt.input))

  // Paste recognition must be instant. The hot path only scans for the ZYAL
  // envelope; full YAML parse + schema validation is deferred until the close
  // sentinel is present, and submit always re-validates synchronously.
  createEffect(() => {
    const envelope = zyalEnvelope()
    const text = store.prompt.input
    if (envelope.kind === "none" && !textHasZyalSentinel(text)) {
      setDaemonDraft({ kind: "none" })
      return
    }

    setDaemonDraft({
      kind: "draft",
      id: envelope.kind === "zyal" ? envelope.id : undefined,
      complete: envelope.kind === "zyal" && envelope.complete,
    })

    if (envelope.kind !== "zyal" || !envelope.hasClose) return
    const handle = setTimeout(() => {
      if (store.prompt.input !== text) return
      const parsed = detectZyal(text)
      setDaemonDraft(
        parsed.kind === "none"
          ? { kind: "draft", id: envelope.id, complete: envelope.complete }
          : parsed,
      )
    }, 50)
    onCleanup(() => clearTimeout(handle))
  })

  const composedKeybindings = createMemo(() => {
    const base = textareaKeybindings()
    const composing = isZyalInput()
    if (!composing) return base
    // While composing a ZYAL block, bare Enter inserts a newline so terminals
    // that drop bracketed paste don't auto-submit on the first newline of the
    // streamed body. Submit requires a modifier (meta/ctrl/shift+Enter).
    return [
      { name: "return", action: "newline" as const },
      { name: "return", meta: true, action: "submit" as const },
      { name: "return", ctrl: true, action: "submit" as const },
      { name: "return", shift: true, action: "submit" as const },
      ...base.filter((b) => b.name !== "return"),
    ]
  })

  // Flash the TUI gold when the user is composing an ZYAL block.
  createEffect(() => {
    setZyalFlashSource("prompt", isZyalInput())
  })
  onCleanup(() => setZyalFlashSource("prompt", false))

  // Brilliant high-contrast YAML highlights inside the prompt while a ZYAL
  // block is detected. Re-tokenises on every input change, and clears its extmarks the
  // moment the block is removed. Tracks them under their own typeId so the
  // prompt-part extmarks stay untouched.
  //
  // PERF: opentui's ExtmarksController.updateHighlights() walks every extmark
  // and recomputes display offsets per call — O(N × text). Calling create()
  // or delete() N times in a loop is O(N² × text). For a 600-token ZYAL
  // paste that's ~2 billion ops and translates to multi-second hangs. We
  // suppress the per-op rebuild by pausing updateHighlights for the duration
  // of the diff and call it exactly once at the end.
  createEffect(() => {
    if (!input || input.isDestroyed) return
    if (yamlTokenTypeId === 0) return
    const composing = isZyalInput()
    const text = store.prompt.input

    const realUpdate = Reflect.get(input.extmarks, "updateHighlights")
    const canBatch = typeof realUpdate === "function"
    if (canBatch) Reflect.set(input.extmarks, "updateHighlights", () => {})
    try {
      const existing = input.extmarks.getAllForTypeId(yamlTokenTypeId)
      for (const mark of existing) input.extmarks.delete(mark.id)
      if (!composing) return
      const palette = yamlSyntax()
      const tokens = tokenizeYaml(text)
      for (const token of tokens) {
        const styleId = palette.getStyleId(`yaml.${token.scope}`)
        if (styleId === undefined || styleId === null) continue
        input.extmarks.create({
          typeId: yamlTokenTypeId,
          start: token.start,
          end: token.end,
          styleId,
        })
      }
    } finally {
      if (canBatch) {
        Reflect.set(input.extmarks, "updateHighlights", realUpdate)
        realUpdate.call(input.extmarks)
      }
    }
  })

  createEffect(
    on(
      () => props.sessionID,
      () => {
        setStore("default_value", randomIndex(list().length))
      },
      { defer: true },
    ),
  )

  // Initialize agent/model/variant from last user message when session changes
  let syncedSessionID: string | undefined
  createEffect(() => {
    const sessionID = props.sessionID
    const msgState = lastUserMessage()

    if (sessionID !== syncedSessionID) {
      if (!sessionID || msgState.kind === "missing") return

      syncedSessionID = sessionID
      const msg = msgState.message

      // Only set agent if it's a primary agent (not a subagent)
      const isPrimaryAgent = local.agent.list().some((x) => x.name === msg.agent)
      if (msg.agent && isPrimaryAgent) {
        // Keep command line --agent if specified.
        if (!args.agent) local.agent.set(msg.agent)
        if (msg.model) {
          local.model.set(msg.model)
          local.model.variant.set(msg.model.variant)
        }
      }
    }
  })

  command.register(() => {
    return [
      {
        title: "Clear prompt",
        value: "prompt.clear",
        category: "Prompt",
        hidden: true,
        onSelect: (dialog) => {
          input.extmarks.clear()
          input.clear()
          dialog.clear()
        },
      },
      {
        title: "Submit prompt",
        value: "prompt.submit",
        keybind: "input_submit",
        category: "Prompt",
        hidden: true,
        onSelect: async (dialog) => {
          if (!input.focused) return
          const handled = await submit()
          if (!handled) return

          dialog.clear()
        },
      },
      {
        title: "Remove editor context",
        value: "prompt.editor_context.clear",
        category: "Prompt",
        enabled: Boolean(editorContext()),
        onSelect: (dialog) => {
          dismissEditorContext()
          dialog.clear()
        },
      },
      {
        title: "Paste",
        value: "prompt.paste",
        keybind: "input_paste",
        category: "Prompt",
        hidden: true,
        onSelect: async () => {
          const content = await Clipboard.read()
          if (content?.mime.startsWith("image/")) {
            await pasteAttachment({
              filename: "clipboard",
              mime: content.mime,
              content: content.data,
            })
          }
        },
      },
      {
        title: "Interrupt session",
        value: "session.interrupt",
        keybind: "session_interrupt",
        category: "Session",
        hidden: true,
        enabled: status().type !== "idle",
        onSelect: (dialog) => {
          if (autocomplete.visible) return
          if (!input.focused) return
          // this interrupt handling should live in its own command
          if (store.mode === "shell") {
            setStore("mode", "normal")
            return
          }
          if (!props.sessionID) return

          setStore("interrupt", store.interrupt + 1)

          setTimeout(() => {
            setStore("interrupt", 0)
          }, 5000)

          if (store.interrupt >= 2) {
            void sdk.client.session.abort({
              sessionID: props.sessionID,
            })
            setStore("interrupt", 0)
          }
          dialog.clear()
        },
      },
      {
        title: "Open editor",
        category: "Session",
        keybind: "editor_open",
        value: "prompt.editor",
        slash: {
          name: "editor",
        },
        onSelect: async (dialog) => {
          dialog.clear()

          // replace summarized text parts with the actual text
          const text = store.prompt.parts
            .filter((p) => p.type === "text")
            .reduce((acc, p) => {
              if (!p.source) return acc
              return acc.replace(p.source.text.value, p.text)
            }, store.prompt.input)

          const nonTextParts = store.prompt.parts.filter((p) => p.type !== "text")

          const value = text
          const content = await Editor.open({ value, renderer })
          if (!content) return

          input.setText(content)

          // Update positions for nonTextParts based on their location in new content
          // Filter out parts whose virtual text was deleted
          // this handles a case where the user edits the text in the editor
          // such that the virtual text moves around or is deleted
          const updatedNonTextParts = nonTextParts
            .map((part) => {
              let virtualText = ""
              if (part.type === "file" && part.source?.text) {
                virtualText = part.source.text.value
              } else if (part.type === "agent" && part.source) {
                virtualText = part.source.value
              }

              if (!virtualText) return part

              const newStart = content.indexOf(virtualText)
              // if the virtual text is deleted, remove the part
              // jankurai:allow HLT-001-DEAD-MARKER reason=solidjs-render-guard-returns-by-design expires=2027-01-01
              if (newStart === -1) return null

              const newEnd = newStart + virtualText.length

              if (part.type === "file" && part.source?.text) {
                return {
                  ...part,
                  source: {
                    ...part.source,
                    text: {
                      ...part.source.text,
                      start: newStart,
                      end: newEnd,
                    },
                  },
                }
              }

              if (part.type === "agent" && part.source) {
                return {
                  ...part,
                  source: {
                    ...part.source,
                    start: newStart,
                    end: newEnd,
                  },
                }
              }

              return part
            })
            .filter((part) => part !== null)

          setStore("prompt", {
            input: content,
            // keep only the non-text parts because the text parts were
            // already expanded inline
            parts: updatedNonTextParts,
          })
          restoreExtmarksFromParts(updatedNonTextParts)
          input.cursorOffset = Bun.stringWidth(content)
        },
      },
      {
        title: "Skills",
        value: "prompt.skills",
        category: "Prompt",
        slash: {
          name: "skills",
        },
        onSelect: () => {
          dialog.replace(() => (
            <DialogSkill
              onSelect={(skill) => {
                input.setText(`/${skill} `)
                setStore("prompt", {
                  input: `/${skill} `,
                  parts: [],
                })
                input.gotoBufferEnd()
              }}
            />
          ))
        },
      },
      {
        title: "Warp",
        description: "Change the workspace for the session",
        value: "workspace.set",
        category: "Session",
        enabled: Flag.JEKKO_EXPERIMENTAL_WORKSPACES,
        slash: {
          name: "warp",
        },
        onSelect: (dialog) => {
          void openWorkspaceSelect({
            dialog,
            sdk,
            sync,
            toast,
            onSelect: (selection) => {
              void warpSession(selection)
            },
          })
        },
      },
    ]
  })

  const ref: PromptRef = {
    get focused() {
      return input.focused
    },
    get current() {
      return store.prompt
    },
    focus() {
      input.focus()
    },
    blur() {
      input.blur()
    },
    set(prompt) {
      input.setText(prompt.input)
      setStore("prompt", prompt)
      restoreExtmarksFromParts(prompt.parts)
      input.gotoBufferEnd()
    },
    reset() {
      input.clear()
      input.extmarks.clear()
      setStore("prompt", {
        input: "",
        parts: [],
      })
      setStore("extmarkToPartIndex", new Map())
    },
    submit() {
      void submit()
    },
  }

  onMount(() => {
    const saved = stashed
    stashed = undefined
    if (store.prompt.input) return
    if (saved && saved.prompt.input) {
      input.setText(saved.prompt.input)
      setStore("prompt", saved.prompt)
      restoreExtmarksFromParts(saved.prompt.parts)
      input.cursorOffset = saved.cursor
    }
  })

  onCleanup(() => {
    if (store.prompt.input) {
      stashed = { prompt: unwrap(store.prompt), cursor: input.cursorOffset }
    }
    props.ref?.(undefined)
  })

  createEffect(() => {
    if (!input || input.isDestroyed) return
    if (props.visible === false || dialog.stack.length > 0) {
      if (input.focused) input.blur()
      return
    }

    // Slot/plugin updates can remount the background prompt while a dialog is open.
    // Keep focus with the dialog and let the prompt reclaim it after the dialog closes.
    if (!input.focused) input.focus()
  })

  createEffect(() => {
    if (!input || input.isDestroyed) return
    input.traits = computePromptTraits({
      mode: store.mode,
      disabled: !!props.disabled,
      autocompleteVisible: !!auto()?.visible,
    })
  })

  function restoreExtmarksFromParts(parts: PromptInfo["parts"]) {
    input.extmarks.clear()
    setStore("extmarkToPartIndex", new Map())

    parts.forEach((part, partIndex) => {
      let start = 0
      let end = 0
      let virtualText = ""
      let styleId: number | undefined

      if (part.type === "file" && part.source?.text) {
        start = part.source.text.start
        end = part.source.text.end
        virtualText = part.source.text.value
        styleId = fileStyleId
      } else if (part.type === "agent" && part.source) {
        start = part.source.start
        end = part.source.end
        virtualText = part.source.value
        styleId = agentStyleId
      } else if (part.type === "text" && part.source?.text) {
        start = part.source.text.start
        end = part.source.text.end
        virtualText = part.source.text.value
        styleId = pasteStyleId
      }

      if (virtualText) {
        const extmarkId = input.extmarks.create({
          start,
          end,
          virtual: true,
          styleId,
          typeId: promptPartTypeId,
        })
        setStore("extmarkToPartIndex", (map: Map<number, number>) => {
          const newMap = new Map(map)
          newMap.set(extmarkId, partIndex)
          return newMap
        })
      }
    })
  }

  function syncExtmarksWithPromptParts() {
    const allExtmarks = input.extmarks.getAllForTypeId(promptPartTypeId)
    setStore(
      produce((draft) => {
        const newMap = new Map<number, number>()
        const newParts: typeof draft.prompt.parts = []

        for (const extmark of allExtmarks) {
          const partIndex = draft.extmarkToPartIndex.get(extmark.id)
          if (partIndex !== undefined) {
            const part = draft.prompt.parts[partIndex]
            if (part) {
              if (part.type === "agent" && part.source) {
                part.source.start = extmark.start
                part.source.end = extmark.end
              } else if (part.type === "file" && part.source?.text) {
                part.source.text.start = extmark.start
                part.source.text.end = extmark.end
              } else if (part.type === "text" && part.source?.text) {
                part.source.text.start = extmark.start
                part.source.text.end = extmark.end
              }
              newMap.set(extmark.id, newParts.length)
              newParts.push(part)
            }
          }
        }

        draft.extmarkToPartIndex = newMap
        draft.prompt.parts = newParts
      }),
    )
  }

  command.register(() => [
    {
      title: "Stash prompt",
      value: "prompt.stash",
      category: "Prompt",
      enabled: !!store.prompt.input,
      onSelect: (dialog) => {
        if (!store.prompt.input) return
        stash.push({
          input: store.prompt.input,
          parts: store.prompt.parts,
        })
        input.extmarks.clear()
        input.clear()
        setStore("prompt", { input: "", parts: [] })
        setStore("extmarkToPartIndex", new Map())
        dialog.clear()
      },
    },
    {
      title: "Stash pop",
      value: "prompt.stash.pop",
      category: "Prompt",
      enabled: stash.list().length > 0,
      onSelect: (dialog) => {
        const entry = stash.pop()
        if (entry) {
          input.setText(entry.input)
          setStore("prompt", { input: entry.input, parts: entry.parts })
          restoreExtmarksFromParts(entry.parts)
          input.gotoBufferEnd()
        }
        dialog.clear()
      },
    },
    {
      title: "Stash list",
      value: "prompt.stash.list",
      category: "Prompt",
      enabled: stash.list().length > 0,
      onSelect: (dialog) => {
        dialog.replace(() => (
          <DialogStash
            onSelect={(entry) => {
              input.setText(entry.input)
              setStore("prompt", { input: entry.input, parts: entry.parts })
              restoreExtmarksFromParts(entry.parts)
              input.gotoBufferEnd()
            }}
          />
        ))
      },
    },
  ])

  async function submit() {
    setWarpNotice(undefined)

    // IME: double-defer may fire before onContentChange flushes the last
    // composed character (e.g. Korean hangul) to the store, so read
    // plainText directly and sync before any downstream reads.
    if (input && !input.isDestroyed && input.plainText !== store.prompt.input) {
      setStore("prompt", "input", input.plainText)
      syncExtmarksWithPromptParts()
    }
    if (props.disabled) return false
    if (workspaceCreating()) return false
    if (autocomplete?.visible) return false
    if (!store.prompt.input) return false
    const agent = local.agent.current()
    if (!agent) return false
    const trimmed = store.prompt.input.trim()
    if (trimmed === "exit" || trimmed === "quit" || trimmed === ":q") {
      void exit()
      return true
    }
    const selectedModel = local.model.current()
    if (!selectedModel) {
      void promptModelWarning()
      return false
    }

    let daemonMode: ZyalDetection = isZyalInput() ? detectZyal(store.prompt.input) : { kind: "none" }
    if (daemonMode.kind === "invalid") {
      const sendAsPlainText = await DialogConfirm.show(dialog, "Invalid ZYAL", daemonMode.error, "send as plain text")
      if (!sendAsPlainText) return false
      daemonMode = { kind: "none" }
    } else if (daemonMode.kind === "preview") {
      // Auto-approve ZYAL payload submission without showing the preview dialog
    }

    const workspaceSession = props.sessionID ? sync.session.get(props.sessionID) : undefined
    const workspaceID = workspaceSession?.workspaceID
    const workspaceStatus = workspaceID ? (project.workspace.status(workspaceID) ?? "error") : undefined
    if (props.sessionID && workspaceID && workspaceStatus !== "connected") {
      dialog.replace(() => (
        <DialogWorkspaceUnavailable
          onRestore={() => {
            void openWorkspaceSelect({
              dialog,
              sdk,
              sync,
              toast,
              onSelect: (selection) => {
                void warpSession(selection)
              },
            })
            return false
          }}
        />
      ))
      return false
    }

    const variant = local.model.variant.current()
    let sessionID = props.sessionID
    if (sessionID == null) {
      const workspace = workspaceSelection()
      const workspaceID = iife(() => {
        if (!workspace) return defaultWorkspaceID()
        // jankurai:allow HLT-001-DEAD-MARKER reason=solidjs-render-guard-returns-by-design expires=2027-01-01
        if (workspace.type === "none") return undefined
        if (workspace.type === "existing") return workspace.workspaceID
        // jankurai:allow HLT-001-DEAD-MARKER reason=solidjs-render-guard-returns-by-design expires=2027-01-01
        return undefined
      })

      const res = await sdk.client.session.create({
        workspace: workspaceID,
        agent: agent.name,
        model: {
          providerID: selectedModel.providerID,
          id: selectedModel.modelID,
          variant,
        },
      })

      if (res.error) {
        console.log("Creating a session failed:", res.error)

        toast.show({
          message: "Creating a session failed. Open console for more details.",
          variant: "error",
        })

        return true
      }

      sessionID = res.data.id
    }

    const messageID = MessageID.ascending()
    let inputText = store.prompt.input

    // Expand pasted text inline before submitting
    const allExtmarks = input.extmarks.getAllForTypeId(promptPartTypeId)
    const sortedExtmarks = allExtmarks.sort((a: { start: number }, b: { start: number }) => b.start - a.start)

    for (const extmark of sortedExtmarks) {
      const partIndex = store.extmarkToPartIndex.get(extmark.id)
      if (partIndex !== undefined) {
        const part = store.prompt.parts[partIndex]
        if (part?.type === "text" && part.text) {
          const before = inputText.slice(0, extmark.start)
          const after = inputText.slice(extmark.end)
          inputText = before + part.text + after
        }
      }
    }

    // Filter out text parts (pasted content) since they're now expanded inline
    const nonTextParts = store.prompt.parts.filter((part) => part.type !== "text")

    // Capture mode before it gets reset
    const currentMode = store.mode
    const editorSelection = editorContext()
    const editorParts =
      editorSelection && editor.labelState() === "pending"
        ? [
            {
              id: PartID.ascending(),
              type: "text" as const,
              text: formatEditorContext(editorSelection),
              synthetic: true,
              metadata: {
                kind: "editor_context",
                source: editorSelection.source ?? "editor",
                filePath: editorSelection.filePath,
                ranges: editorSelection.ranges,
              },
            },
          ]
        : []

    if (daemonMode.kind === "preview") {
      if (!sessionID) return false
      const response = await sdk.fetch(new URL(`/session/${sessionID}/daemon/start`, sdk.url), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          parts: [{ type: "text", text: inputText }],
        }),
      })
      if (!response.ok) {
        toast.show({
          message: `Failed to start daemon: ${await response.text()}`,
          variant: "error",
        })
        return false
      }
      setOverlay("jekko-gold")
      setZyalFlashSource("prompt:submitted", true)
    } else if (store.mode === "shell") {
      void sdk.client.session.shell({
        sessionID,
        agent: agent.name,
        model: {
          providerID: selectedModel.providerID,
          modelID: selectedModel.modelID,
        },
        command: inputText,
      })
      setStore("mode", "normal")
    } else if (
      inputText.startsWith("/") &&
      iife(() => {
        const firstLine = inputText.split("\n")[0]
        const command = firstLine.split(" ")[0].slice(1)
        return sync.data.command.some((x) => x.name === command)
      })
    ) {
      // Parse command from first line, preserve multi-line content in arguments
      const firstLineEnd = inputText.indexOf("\n")
      const firstLine = firstLineEnd === -1 ? inputText : inputText.slice(0, firstLineEnd)
      const [command, ...firstLineArgs] = firstLine.split(" ")
      const restOfInput = firstLineEnd === -1 ? "" : inputText.slice(firstLineEnd + 1)
      const args = firstLineArgs.join(" ") + (restOfInput ? "\n" + restOfInput : "")

      void sdk.client.session.command({
        sessionID,
        command: command.slice(1),
        arguments: args,
        agent: agent.name,
        model: `${selectedModel.providerID}/${selectedModel.modelID}`,
        messageID,
        variant,
        parts: nonTextParts
          .filter((x) => x.type === "file")
          .map((x) => ({
            id: PartID.ascending(),
            ...x,
          })),
      })
    } else {
      sdk.client.session
        .prompt({
          sessionID,
          ...selectedModel,
          messageID,
          agent: agent.name,
          model: selectedModel,
          variant,
          parts: [
            ...editorParts,
            {
              id: PartID.ascending(),
              type: "text",
              text: inputText,
            },
            ...nonTextParts.map(assign),
          ],
        })
        .catch(() => {})
      if (editorParts.length > 0) editor.markSelectionSent()
    }
    history.append({
      ...store.prompt,
      mode: currentMode,
    })
    input.extmarks.clear()
    setStore("prompt", {
      input: "",
      parts: [],
    })
    setStore("extmarkToPartIndex", new Map())
    props.onSubmit?.()

    // transient shortcut to make sure the message is sent
    if (!props.sessionID) {
      if (editorParts.length > 0) editor.preserveSelectionFromNewSession()
      setTimeout(() => {
        route.navigate({
          type: "session",
          sessionID,
        })
      }, 50)
    }
    input.clear()
    return true
  }
  const exit = useExit()

  function pasteText(text: string, virtualText: string) {
    const currentOffset = input.visualCursor.offset
    const extmarkStart = currentOffset
    const extmarkEnd = extmarkStart + virtualText.length

    input.insertText(virtualText + " ")

    const extmarkId = input.extmarks.create({
      start: extmarkStart,
      end: extmarkEnd,
      virtual: true,
      styleId: pasteStyleId,
      typeId: promptPartTypeId,
    })

    setStore(
      produce((draft) => {
        const partIndex = draft.prompt.parts.length
        draft.prompt.parts.push({
          type: "text" as const,
          text,
          source: {
            text: {
              start: extmarkStart,
              end: extmarkEnd,
              value: virtualText,
            },
          },
        })
        draft.extmarkToPartIndex.set(extmarkId, partIndex)
      }),
    )
  }

  async function pasteAttachment(file: { filename?: string; filepath?: string; content: string; mime: string }) {
    const currentOffset = input.visualCursor.offset
    const extmarkStart = currentOffset
    const pdf = file.mime === "application/pdf"
    const count = store.prompt.parts.filter((x) => {
      if (x.type !== "file") return false
      if (pdf) return x.mime === "application/pdf"
      return x.mime.startsWith("image/")
    }).length
    const virtualText = pdf ? `[PDF ${count + 1}]` : `[Image ${count + 1}]`
    const extmarkEnd = extmarkStart + virtualText.length
    const textToInsert = virtualText + " "

    input.insertText(textToInsert)

    const extmarkId = input.extmarks.create({
      start: extmarkStart,
      end: extmarkEnd,
      virtual: true,
      styleId: pasteStyleId,
      typeId: promptPartTypeId,
    })

    const part: Omit<FilePart, "id" | "messageID" | "sessionID"> = {
      type: "file" as const,
      mime: file.mime,
      filename: file.filename,
      url: `data:${file.mime};base64,${file.content}`,
      source: {
        type: "file",
        path: file.filepath ?? file.filename ?? "",
        text: {
          start: extmarkStart,
          end: extmarkEnd,
          value: virtualText,
        },
      },
    }
    setStore(
      produce((draft) => {
        const partIndex = draft.prompt.parts.length
        draft.prompt.parts.push(part)
        draft.extmarkToPartIndex.set(extmarkId, partIndex)
      }),
    )
    return
  }

  const highlight = createMemo(() => {
    if (keybind.leader) return theme.border
    if (store.mode === "shell") return theme.primary
    const agent = local.agent.current()
    if (!agent) return theme.border
    return local.agent.color(agent.name)
  })

  const showVariant = createMemo(() => {
    const variants = local.model.variant.list()
    if (variants.length === 0) return false
    const current = local.model.variant.current()
    return !!current
  })

  const agentMetaAlpha = createFadeIn(() => !!local.agent.current(), animationsEnabled)
  const modelMetaAlpha = createFadeIn(() => !!local.agent.current() && store.mode === "normal", animationsEnabled)
  const variantMetaAlpha = createFadeIn(
    () => !!local.agent.current() && store.mode === "normal" && showVariant(),
    animationsEnabled,
  )
  const borderHighlight = createMemo(() => {
    const base = tint(theme.border, highlight(), agentMetaAlpha())
    const draft = daemonDraft()
    if (draft.kind === "invalid") return tint(base, theme.error, 0.55)
    if (draft.kind === "preview" || draft.kind === "draft" || isZyalInput()) return tint(base, theme.warning, 0.55)
    return base
  })

  const inputSuggestionText = createMemo(() => {
    // jankurai:allow HLT-001-DEAD-MARKER reason=solidjs-render-guard-returns-by-design expires=2027-01-01
    if (props.showSuggestion === false) return undefined
    if (store.mode === "shell") {
      // jankurai:allow HLT-001-DEAD-MARKER reason=solidjs-render-guard-returns-by-design expires=2027-01-01
      if (!shell().length) return undefined
      const example = shell()[store.default_value % shell().length]
      return `Run a command... "${example}"`
    }
    // jankurai:allow HLT-001-DEAD-MARKER reason=solidjs-render-guard-returns-by-design expires=2027-01-01
    if (!list().length) return undefined
    return `Ask anything... "${list()[store.default_value % list().length]}"`
  })

  const inputProps = {
    default_value: inputSuggestionText(),
  } as any

  const inputTextColor = createMemo(() => {
    if (keybind.leader) return theme.textMuted
    if (!store.prompt.input) return theme.textMuted
    return theme.text
  })

  const workspaceLabel = createMemo<
    | { type: "new"; workspaceType: string }
    | { type: "existing"; workspaceType: string; workspaceName: string; status?: WorkspaceStatus }
    | undefined
  >(() => {
    const selected = workspaceSelection()
    if (!selected) {
      const workspaceID = defaultWorkspaceID()
      if (props.sessionID || !workspaceID) return
      const workspace = project.workspace.get(workspaceID)
      return {
        type: "existing",
        workspaceType: workspace?.type ?? "unknown",
        workspaceName: workspace?.name ?? workspaceID,
        status: project.workspace.status(workspaceID) ?? "error",
      }
    }
    if (selected.type === "none") return
    if (props.sessionID && !workspaceCreating()) return
    if (selected.type === "new") {
      return {
        type: "new",
        workspaceType: selected.workspaceType,
      }
    }
    return {
      type: "existing",
      workspaceType: selected.workspaceType,
      workspaceName: selected.workspaceName,
      status: selected.type === "existing" ? "connected" : undefined,
    }
  })

  const spinnerDef = createMemo(() => {
    const agent = local.agent.current()
    const color = agent ? local.agent.color(agent.name) : theme.border
    return {
      frames: createFrames({
        color,
        style: "blocks",
        inactiveFactor: 0.6,
        // enableFading: false,
        minAlpha: 0.3,
      }),
      color: createColors({
        color,
        style: "blocks",
        inactiveFactor: 0.6,
        // enableFading: false,
        minAlpha: 0.3,
      }),
    }
  })

  return (
    <>
      <Autocomplete
        sessionID={props.sessionID}
        ref={(r) => {
          autocomplete = r
          setAuto(() => r)
        }}
        anchor={() => anchor}
        input={() => input}
        setPrompt={(cb) => {
          setStore("prompt", produce(cb))
        }}
        setExtmark={(partIndex, extmarkId) => {
          setStore("extmarkToPartIndex", (map: Map<number, number>) => {
            const newMap = new Map(map)
            newMap.set(extmarkId, partIndex)
            return newMap
          })
        }}
        value={store.prompt.input}
        fileStyleId={fileStyleId}
        agentStyleId={agentStyleId}
        promptPartTypeId={() => promptPartTypeId}
      />
      <box ref={(r) => (anchor = r)} visible={props.visible !== false}>
        <box
          border={["left"]}
          borderColor={borderHighlight()}
          customBorderChars={{
            ...SplitBorder.customBorderChars,
            bottomLeft: "╹",
          }}
        >
          <box
            paddingLeft={2}
            paddingRight={2}
            paddingTop={1}
            flexShrink={0}
            backgroundColor={theme.backgroundElement}
            flexGrow={1}
          >
            <textarea
              {...inputProps}
              textColor={inputTextColor()}
              focusedTextColor={inputTextColor()}
              minHeight={1}
              maxHeight={6}
              onContentChange={() => {
                const value = input.plainText
                setStore("prompt", "input", value)
                autocomplete.onInput(value)
                syncExtmarksWithPromptParts()
              }}
              keyBindings={composedKeybindings()}
              onKeyDown={async (e) => {
                if (props.disabled) {
                  e.preventDefault()
                  return
                }
                // Check clipboard for images before terminal-handled paste runs.
                // This helps terminals that forward Ctrl+V to the app; Windows
                // Terminal 1.25+ usually handles Ctrl+V before this path.
                if (keybind.match("input_paste", e)) {
                  const content = await Clipboard.read()
                  if (content?.mime.startsWith("image/")) {
                    e.preventDefault()
                    await pasteAttachment({
                      filename: "clipboard",
                      mime: content.mime,
                      content: content.data,
                    })
                    return
                  }
                  // If no image, let the default paste behavior continue
                }
                if (keybind.match("input_clear", e) && store.prompt.input !== "") {
                  input.clear()
                  input.extmarks.clear()
                  setStore("prompt", {
                    input: "",
                    parts: [],
                  })
                  setStore("extmarkToPartIndex", new Map())
                  return
                }
                if (keybind.match("app_exit", e)) {
                  if (store.prompt.input === "") {
                    await exit()
                    // Don't preventDefault - let textarea potentially handle the event
                    e.preventDefault()
                    return
                  }
                }
                if (e.name === "!" && input.visualCursor.offset === 0) {
                  setStore("default_value", randomIndex(shell().length))
                  setStore("mode", "shell")
                  e.preventDefault()
                  return
                }
                if (store.mode === "shell") {
                  if ((e.name === "backspace" && input.visualCursor.offset === 0) || e.name === "escape") {
                    setStore("mode", "normal")
                    e.preventDefault()
                    return
                  }
                }
                if (store.mode === "normal") autocomplete.onKeyDown(e)
                if (!autocomplete.visible) {
                  if (
                    (keybind.match("history_previous", e) && input.cursorOffset === 0) ||
                    (keybind.match("history_next", e) && input.cursorOffset === input.plainText.length)
                  ) {
                    const direction = keybind.match("history_previous", e) ? -1 : 1
                    const item = history.move(direction, input.plainText)
                    if (item.kind === "blocked") return

                    input.setText(item.prompt.input)
                    setStore("prompt", item.prompt)
                    setStore("mode", item.prompt.mode ?? "normal")
                    restoreExtmarksFromParts(item.prompt.parts)
                    e.preventDefault()
                    if (direction === -1) input.cursorOffset = 0
                    if (direction === 1) input.cursorOffset = input.plainText.length
                    return
                  }

                  if (keybind.match("history_previous", e) && input.visualCursor.visualRow === 0) input.cursorOffset = 0
                  if (keybind.match("history_next", e) && input.visualCursor.visualRow === input.height - 1)
                    input.cursorOffset = input.plainText.length
                }
              }}
              onSubmit={() => {
                // IME: double-defer so the last composed character (e.g. Korean
                // hangul) is flushed to plainText before we read it for submission.
                setTimeout(() => setTimeout(() => submit(), 0), 0)
              }}
              onPaste={async (event: PasteEvent) => {
                if (props.disabled) {
                  event.preventDefault()
                  return
                }

                // Normalize line endings at the boundary
                // Windows ConPTY/Terminal often sends CR-only newlines in bracketed paste
                // Replace CRLF first, then any remaining CR
                const normalizedText = decodePasteBytes(event.bytes).replace(/\r\n/g, "\n").replace(/\r/g, "\n")
                const pastedContent = normalizedText.trim()

                // Windows Terminal <1.25 can surface image-only clipboard as an
                // empty bracketed paste. Windows Terminal 1.25+ does not.
                if (!pastedContent) {
                  command.trigger("prompt.paste")
                  return
                }

                // Once we cross an async boundary below, the terminal may perform its
                // default paste unless we suppress it first and handle insertion ourselves.
                event.preventDefault()

                const filepath = iife(() => {
                  const raw = pastedContent.replace(/^['"]+|['"]+$/g, "")
                  if (raw.startsWith("file://")) {
                    try {
                      return fileURLToPath(raw)
                    } catch {}
                  }
                  if (process.platform === "win32") return raw
                  return raw.replace(/\\(.)/g, "$1")
                })
                const isUrl = /^(https?):\/\//.test(filepath)
                if (!isUrl) {
                  try {
                    const mime = await Filesystem.mimeType(filepath)
                    const filename = path.basename(filepath)
                    // Handle SVG as raw text content, not as base64 image
                    if (mime === "image/svg+xml") {
                      const content = await Filesystem.readText(filepath).catch(() => {})
                      if (content) {
                        pasteText(content, `[SVG: ${filename ?? "image"}]`)
                        return
                      }
                    }
                    if (mime.startsWith("image/") || mime === "application/pdf") {
                      const content = await Filesystem.readArrayBuffer(filepath)
                        .then((buffer) => Buffer.from(buffer).toString("base64"))
                        .catch(() => {})
                      if (content) {
                        await pasteAttachment({
                          filename,
                          filepath,
                          mime,
                          content,
                        })
                        return
                      }
                    }
                  } catch {}
                }

                const lineCount = (pastedContent.match(/\n/g)?.length ?? 0) + 1
                const isZyalPaste =
                  textHasZyalSentinel(pastedContent) ||
                  textHasZyalSentinel(store.prompt.input)

                if (
                  !isZyalPaste &&
                  (lineCount >= 3 || pastedContent.length > 150) &&
                  kv.get("paste_summary_enabled", !sync.data.config.experimental?.disable_paste_summary)
                ) {
                  pasteText(pastedContent, `[Pasted ~${lineCount} lines]`)
                  return
                }

                input.insertText(normalizedText)
                setStore("prompt", "input", input.plainText)
                autocomplete.onInput(input.plainText)
                syncExtmarksWithPromptParts()

                // Force layout update and render for the pasted content
                setTimeout(() => {
                  // setTimeout is a alternative and needs to be addressed properly
                  if (!input || input.isDestroyed) return
                  input.getLayoutNode().markDirty()
                  renderer.requestRender()
                }, 0)
              }}
              ref={(r: TextareaRenderable) => {
                input = r
                if (promptPartTypeId === 0) {
                  promptPartTypeId = input.extmarks.registerType("prompt-part")
                }
                if (yamlTokenTypeId === 0) {
                  yamlTokenTypeId = input.extmarks.registerType("yaml-token")
                }
                props.ref?.(ref)
                setTimeout(() => {
                  // setTimeout is a alternative and needs to be addressed properly
                  if (!input || input.isDestroyed) return
                  input.cursorColor = theme.text
                }, 0)
              }}
              onMouseDown={(r: MouseEvent) => r.target?.focus()}
              focusedBackgroundColor={theme.backgroundElement}
              cursorColor={props.disabled ? theme.backgroundElement : theme.text}
              syntaxStyle={isZyalInput() ? yamlSyntax() : syntax()}
            />
            <box flexDirection="row" flexShrink={0} paddingTop={1} gap={1} justifyContent="space-between">
              <box flexDirection="row" gap={1}>
                {local.agent.current() ? (
                  <>
                    <text fg={fadeColor(highlight(), agentMetaAlpha())}>
                      {store.mode === "shell" ? "Shell" : Locale.titlecase(local.agent.current()!.name)}
                    </text>
                    <Show when={store.mode === "normal"}>
                      <box flexDirection="row" gap={1}>
                        <text fg={fadeColor(theme.textMuted, modelMetaAlpha())}>·</text>
                        <text
                          flexShrink={0}
                          fg={fadeColor(keybind.leader ? theme.textMuted : theme.text, modelMetaAlpha())}
                        >
                          {local.model.parsed().model}
                        </text>
                        <text fg={fadeColor(theme.textMuted, modelMetaAlpha())}>{currentProviderLabel()}</text>
                        <Show when={showVariant()}>
                          <text fg={fadeColor(theme.textMuted, variantMetaAlpha())}>·</text>
                          <text>
                            <span style={{ fg: fadeColor(theme.warning, variantMetaAlpha()), bold: true }}>
                              {local.model.variant.current()}
                            </span>
                          </text>
                        </Show>
                      </box>
                    </Show>
                  </>
                ) : (
                  <box height={1} />
                )}
              </box>
              <Show when={isZyalInput()}>
                <box flexDirection="row" gap={1} alignItems="center">
                  {daemonDraft().kind === "invalid" ? (
                    <text fg={theme.error}>
                      <b>✗ ZYAL</b>
                      <span style={{ fg: theme.textMuted }}>
                        {" "}{(daemonDraft() as { kind: "invalid"; error: string }).error.slice(0, 40)}
                      </span>
                    </text>
                  ) : (
                    <text fg={theme.warning}>
                      <b>✓ ZYAL</b>
                    </text>
                  )}
                </box>
              </Show>
              <Show when={hasRightContent()}>
                <box flexDirection="row" gap={1} alignItems="center">
                  {props.right}
                </box>
              </Show>
            </box>
          </box>
        </box>
        <box
          height={1}
          border={["left"]}
          borderColor={borderHighlight()}
          customBorderChars={{
            ...EmptyBorder,
            vertical: theme.backgroundElement.a !== 0 ? "╹" : " ",
          }}
        >
          <box
            height={1}
            border={["bottom"]}
            borderColor={theme.backgroundElement}
            customBorderChars={
              theme.backgroundElement.a !== 0
                ? {
                    ...EmptyBorder,
                    horizontal: "▀",
                  }
                : {
                    ...EmptyBorder,
                    horizontal: " ",
                  }
            }
          />
        </box>
        <box width="100%" flexDirection="row" justifyContent="space-between">
          <Switch>
            <Match when={status().type !== "idle"}>
              <box
                flexDirection="row"
                gap={1}
                flexGrow={1}
                justifyContent={status().type === "retry" ? "space-between" : "flex-start"}
              >
                <box flexShrink={0} flexDirection="row" gap={1}>
                  <box marginLeft={1}>
                    {kv.get("animations_enabled", true) ? (
                      <spinner color={spinnerDef().color} frames={spinnerDef().frames} interval={40} />
                    ) : (
                      <text fg={theme.textMuted}>[⋯]</text>
                    )}
                  </box>
                  <box flexDirection="row" gap={1} flexShrink={0}>
                    {(() => {
                      const retry = createMemo(() => {
                        const s = status()
                        if (s.type !== "retry") return
                        return s
                      })
                      const message = createMemo(() => {
                        const r = retry()
                        if (!r) return
                        if (r.message.includes("exceeded your current quota") && r.message.includes("gemini"))
                          return "gemini is way too hot right now"
                        if (r.message.length > 80) return r.message.slice(0, 80) + "..."
                        return r.message
                      })
                      const isTruncated = createMemo(() => {
                        const r = retry()
                        if (!r) return false
                        return r.message.length > 120
                      })
                      const [seconds, setSeconds] = createSignal(0)
                      onMount(() => {
                        const timer = setInterval(() => {
                          const next = retry()?.next
                          if (next) setSeconds(Math.round((next - Date.now()) / 1000))
                        }, 1000)

                        onCleanup(() => {
                          clearInterval(timer)
                        })
                      })
                      const handleMessageClick = () => {
                        const r = retry()
                        if (!r) return
                        if (isTruncated()) {
                          void DialogAlert.show(dialog, "Retry Error", r.message)
                        }
                      }

                      const retryText = () => {
                        const r = retry()
                        if (!r) return ""
                        const baseMessage = message()
                        const truncatedHint = isTruncated() ? " (click to expand)" : ""
                        const duration = formatDuration(seconds())
                        const retryInfo = ` [retrying ${duration ? `in ${duration} ` : ""}attempt #${r.attempt}]`
                        return baseMessage + truncatedHint + retryInfo
                      }

                      return (
                        <Show when={retry()}>
                          <box onMouseUp={handleMessageClick}>
                            <text fg={theme.error}>{retryText()}</text>
                          </box>
                        </Show>
                      )
                    })()}
                  </box>
                </box>
                <text fg={store.interrupt > 0 ? theme.primary : theme.text}>
                  esc{" "}
                  <span style={{ fg: store.interrupt > 0 ? theme.primary : theme.textMuted }}>
                    {store.interrupt > 0 ? "again to interrupt" : "interrupt"}
                  </span>
                </text>
              </box>
            </Match>
            <Match when={warpNotice()}>
              {(notice) => (
                <box paddingLeft={3}>
                  <text fg={theme.accent}>{notice()}</text>
                </box>
              )}
            </Match>
            <Match when={workspaceLabel()}>
              {(workspace) => (
                <box paddingLeft={3} flexDirection="row" gap={1}>
                  <Show when={workspaceCreating()}>
                    <Spinner color={theme.accent} />
                  </Show>
                  <text fg={workspaceCreating() ? theme.accent : theme.text}>
                    {(() => {
                      const item = workspace()
                      if (item.type === "new") {
                        if (workspaceCreating())
                          return `Creating ${item.workspaceType}${".".repeat(workspaceCreatingDots())}`
                        return (
                          <>
                            Workspace <span style={{ fg: theme.textMuted }}>(new {item.workspaceType})</span>
                          </>
                        )
                      }
                      return (
                        <>
                          Workspace <span style={{ fg: theme.textMuted }}>{item.workspaceName}</span>
                        </>
                      )
                    })()}
                  </text>
                </box>
              )}
            </Match>
            <Match when={true}>{props.hint ?? <text />}</Match>
          </Switch>
          <Show when={status().type !== "retry"}>
            <box gap={2} flexDirection="row">
              <Show when={editorContextLabelState() !== "none" ? editorFileLabelDisplay() : undefined}>
                {(file) => (
                  <text fg={editorContextLabelState() === "pending" ? theme.secondary : theme.textMuted}>{file()}</text>
                )}
              </Show>
              <Switch>
                <Match when={store.mode === "normal"}>
                  {usageLine()}
                  <text fg={theme.text}>
                    {keybind.print("command_list")} <span style={{ fg: theme.textMuted }}>commands</span>
                  </text>
                </Match>
                <Match when={store.mode === "shell"}>
                  <text fg={theme.text}>
                    esc <span style={{ fg: theme.textMuted }}>exit shell mode</span>
                  </text>
                </Match>
              </Switch>
            </box>
          </Show>
        </box>
      </box>
    </>
  )
}
