import { useRoute } from "@tui/context/route"
import { useTerminalDimensions, useRenderer, TimeToFirstDraw } from "@opentui/solid"
import {
  Switch,
  Match,
  createEffect,
  createMemo,
  createSignal,
  onMount,
  batch,
  Show,
  on,
} from "solid-js"
import { Flag } from "@jekko-ai/core/flag/flag"
import { useDialog } from "@tui/ui/dialog"
import { DialogProvider as DialogProviderList } from "@tui/component/dialog-provider"
import { PluginRouteMissing } from "@tui/component/plugin-route-missing"
import { useEvent } from "@tui/context/event"
import { useSDK } from "@tui/context/sdk"
import { StartupLoading } from "@tui/component/startup-loading"
import { useSync } from "@tui/context/sync"
import { useLocal } from "@tui/context/local"
import { useCommandDialog } from "@tui/component/dialog-command"
import { KeybindProvider, useKeybind } from "@tui/context/keybind"
import { ThemeProvider, useTheme } from "@tui/context/theme"
import { Home } from "@tui/routes/home"
import { Session } from "@tui/routes/session"
import { useToast } from "./ui/toast"
import { ExitProvider, useExit } from "./context/exit"
import { KVProvider, useKV } from "./context/kv"
import { Provider } from "@/provider/provider"
import { ArgsProvider, useArgs, type Args } from "./context/args"
import { useTuiConfig } from "./context/tui-config"
import { createTuiApi } from "@/cli/cmd/tui/plugin/api"
import { TuiPluginRuntime } from "@/cli/cmd/tui/plugin/runtime"
import type { RouteMap } from "@/cli/cmd/tui/plugin/api-helpers"
import { setupAppBindings } from "./app-bindings"
import { bootJnoccioFusion } from "./context/jnoccio-boot"
import { NavigationHeader } from "@tui/component/navigation-header"
import * as Log from "@jekko-ai/core/util/log"
import { errorMessage } from "@/util/error"

const bootLog = Log.create({ service: "tui.boot" })

export function App(props: { onSnapshot?: () => Promise<string[]>; onVisible?: () => void }) {
  const tuiConfig = useTuiConfig()
  const route = useRoute()
  const dimensions = useTerminalDimensions()
  const renderer = useRenderer()
  const dialog = useDialog()
  const local = useLocal()
  const kv = useKV()
  const command = useCommandDialog()
  const keybind = useKeybind()
  const event = useEvent()
  const sdk = useSDK()
  const toast = useToast()
  const themeState = useTheme()
  const { theme, mode, setMode, locked, lock, unlock } = themeState
  const sync = useSync()
  const exit = useExit()
  const routes: RouteMap = new Map()
  const [routeRev, setRouteRev] = createSignal(0)
  const routeView = (name: string) => {
    routeRev()
    return routes.get(name)?.at(-1)?.render
  }
  let loggedDimensions = false
  createEffect(() => {
    if (loggedDimensions) return
    const size = dimensions()
    if (!size.width || !size.height) return
    loggedDimensions = true
    bootLog.info("app dimensions ready", {
      width: size.width,
      height: size.height,
      log: Log.file(),
    })
  })

  const api = createTuiApi({
    command,
    tuiConfig,
    dialog,
    keybind,
    kv,
    route,
    routes,
    bump: () => setRouteRev((x) => x + 1),
    event,
    sdk,
    sync,
    theme: themeState,
    toast,
    renderer,
  })
  const [ready, setReady] = createSignal(process.env.JEKKO_FAST_BOOT === "1")
  const pluginStartedAt = Date.now()
  bootLog.info("plugin init start", {
    fast_boot: process.env.JEKKO_FAST_BOOT === "1",
    pure: process.env.JEKKO_PURE === "1",
    log: Log.file(),
  })
  TuiPluginRuntime.init({
    api,
    config: tuiConfig,
  })
    .catch((error) => {
      bootLog.error("plugin init failed", {
        error: errorMessage(error),
        log: Log.file(),
      })
      if (process.argv.includes("--print-logs")) {
        console.error("Failed to load TUI plugins", error)
      }
    })
    .finally(() => {
      bootLog.info("plugin init complete", {
        duration: Date.now() - pluginStartedAt,
        log: Log.file(),
      })
      setReady(true)
    })

  bootJnoccioFusion()

  const [terminalTitleEnabled, setTerminalTitleEnabled] = createSignal(kv.get("terminal_title_enabled", true))
  const [pasteSummaryEnabled, setPasteSummaryEnabled] = createSignal(
    kv.get("paste_summary_enabled", !sync.data.config.experimental?.disable_paste_summary),
  )

  setupAppBindings({
    tuiConfig,
    route,
    dialog,
    local,
    kv,
    command,
    event,
    sdk,
    toast,
    renderer,
    sync,
    exit,
    mode,
    setMode: (next) => {
      if (next === "dark" || next === "light") setMode(next)
    },
    locked,
    lock,
    unlock,
    terminalTitleEnabled,
    setTerminalTitleEnabled,
    pasteSummaryEnabled,
    setPasteSummaryEnabled,
    onSnapshot: props.onSnapshot,
  })

  const args = useArgs()
  onMount(() => {
    bootLog.info("app mount", {
      route: route.data.type,
      sync_status: sync.status,
      log: Log.file(),
    })
    props.onVisible?.()
    batch(() => {
      if (args.agent) local.agent.set(args.agent)
      if (args.model) {
        const { providerID, modelID } = Provider.parseModel(args.model)
        if (!providerID || !modelID)
          return toast.show({
            variant: "warning",
            message: `Invalid model format: ${args.model}`,
            duration: 3000,
          })
        local.model.set({ providerID, modelID }, { recent: true })
      }
      if (args.sessionID && !args.fork) {
        route.navigate({
          type: "session",
          sessionID: args.sessionID,
        })
      }
    })
  })

  createEffect(() => {
    bootLog.info("sync status visible", {
      status: sync.status,
      route: route.data.type,
      ready: ready(),
      log: Log.file(),
    })
  })

  let continued = false
  createEffect(() => {
    if (continued || sync.status === "loading" || !args.continue) return
    const match = sync.data.session
      .toSorted((a, b) => b.time.updated - a.time.updated)
      .find((x) => x.parentID === undefined)?.id
    if (match) {
      continued = true
      if (args.fork) {
        void sdk.client.session.fork({ sessionID: match }).then((result) => {
          if (result.data?.id) {
            route.navigate({ type: "session", sessionID: result.data.id })
          } else {
            toast.show({ message: "Failed to fork session", variant: "error" })
          }
        })
      } else {
        route.navigate({ type: "session", sessionID: match })
      }
    }
  })

  let forked = false
  createEffect(() => {
    if (forked || sync.status !== "complete" || !args.sessionID || !args.fork) return
    forked = true
    void sdk.client.session.fork({ sessionID: args.sessionID }).then((result) => {
      if (result.data?.id) {
        route.navigate({ type: "session", sessionID: result.data.id })
      } else {
        toast.show({ message: "Failed to fork session", variant: "error" })
      }
    })
  })

  createEffect(
    on(
      () => sync.status === "complete" && sync.data.provider.length === 0,
      (isEmpty, wasEmpty) => {
        if (!isEmpty || wasEmpty) return
        dialog.replace(() => <DialogProviderList />)
      },
    ),
  )

  const plugin = createMemo(() => {
    if (!ready()) return
    if (route.data.type !== "plugin") return
    const render = routeView(route.data.id)
    if (!render) return <PluginRouteMissing id={route.data.id} onHome={() => route.navigate({ type: "home" })} />
    return render({ params: route.data.data })
  })

  return (
    <box
      width={dimensions().width}
      height={dimensions().height}
      backgroundColor={theme.background}
      flexDirection="column"
    >
      <Show when={Flag.JEKKO_SHOW_TTFD}>
        <TimeToFirstDraw />
      </Show>
      <Show when={!ready()}>
        <box
          position="absolute"
          zIndex={4000}
          left={0}
          right={0}
          top={0}
          bottom={0}
          justifyContent="center"
          alignItems="center"
        >
          <box backgroundColor={theme.backgroundPanel} paddingLeft={2} paddingRight={2} paddingTop={1} paddingBottom={1}>
            <text fg={theme.textMuted}>Loading TUI plugins...</text>
          </box>
        </box>
      </Show>
      <Show when={ready()}>
        <NavigationHeader />
        <box flexGrow={1} minHeight={0} flexDirection="column">
          <Switch>
            <Match when={route.data.type === "home"}>
              <Home />
            </Match>
            <Match when={route.data.type === "session"}>
              <Session />
            </Match>
          </Switch>
          {plugin()}
        </box>
      </Show>
      <StartupLoading ready={ready} />
    </box>
  )
}
