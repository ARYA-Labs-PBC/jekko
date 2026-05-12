// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import { render } from "@opentui/solid"
import * as Clipboard from "@tui/util/clipboard"
import { createCliRenderer, type CliRenderer, type CliRendererConfig } from "@opentui/core"
import { RouteProvider } from "@tui/context/route"
import { createSignal, ErrorBoundary, Show } from "solid-js"
import { win32DisableProcessedInput, win32InstallCtrlCGuard } from "./win32"
import { Flag } from "@jekko-ai/core/flag/flag"
import { DialogProvider } from "@tui/ui/dialog"
import { ErrorComponent } from "@tui/component/error-component"
import { ProjectProvider } from "@tui/context/project"
import { EditorContextProvider } from "@tui/context/editor"
import { SDKProvider } from "@tui/context/sdk"
import { SyncProvider } from "@tui/context/sync"
import { SyncProviderV2 } from "@tui/context/sync"
import { LocalProvider } from "@tui/context/local"
import { CommandProvider } from "@tui/component/dialog-command"
import { KeybindProvider } from "@tui/context/keybind"
import { ThemeProvider } from "@tui/context/theme"
import { PromptHistoryProvider } from "./component/prompt/history"
import { FrecencyProvider } from "./component/prompt/frecency"
import { PromptStashProvider } from "./component/prompt/stash"
import { ToastProvider } from "./ui/toast"
import { ExitProvider } from "./context/exit"
import { KVProvider } from "./context/kv"
import { ArgsProvider, type Args } from "./context/args"
import { PromptRefProvider } from "./context/prompt"
import { TuiConfigProvider } from "./context/tui-config"
import type { TuiConfig } from "./config/tui"
import type { EventSource } from "./context/sdk"
import { TuiPluginRuntime } from "@/cli/cmd/tui/plugin/runtime"
import { App } from "./app-view"
import * as Log from "@jekko-ai/core/util/log"
import { errorMessage } from "@/util/error"

const bootLog = Log.create({ service: "tui.boot" })

function rendererConfig(_config: TuiConfig.Info): CliRendererConfig {
  const tuiwright = process.env.TUIWRIGHT === "1"
  const mouseEnabled = !tuiwright && !Flag.JEKKO_DISABLE_MOUSE && (_config.mouse ?? true)
  const useKittyKeyboard = tuiwright ? null : {}

  return {
    externalOutputMode: "passthrough",
    targetFps: 60,
    gatherStats: true,
    exitOnCtrlC: false,
    useThread: tuiwright ? false : undefined,
    useKittyKeyboard,
    autoFocus: false,
    openConsoleOnError: false,
    useMouse: mouseEnabled,
    consoleOptions: {
      keyBindings: [{ name: "y", ctrl: true, action: "copy-selection" }],
      onCopySelection: (text) => {
        Clipboard.copy(text).catch((error) => {
          bootLog.warn("console selection copy failed", {
            error: errorMessage(error),
            log: Log.file(),
          })
          if (process.argv.includes("--print-logs")) {
            console.error(`Failed to copy console selection to clipboard: ${error}`)
          }
        })
      },
    },
  }
}

function terminalSnapshot(renderer?: CliRenderer) {
  return {
    stdout_columns: process.stdout.columns,
    stdout_rows: process.stdout.rows,
    stdin_tty: process.stdin.isTTY,
    stdout_tty: process.stdout.isTTY,
    renderer_width: renderer?.width,
    renderer_height: renderer?.height,
    terminal_width: renderer?.terminalWidth,
    terminal_height: renderer?.terminalHeight,
    screen_mode: renderer?.screenMode,
    use_thread: renderer?.useThread,
    use_mouse: renderer?.useMouse,
    use_kitty_keyboard: renderer?.useKittyKeyboard,
    control_state: renderer?.currentControlState,
    palette_status: renderer?.paletteDetectionStatus,
  }
}

function installFirstFrameWatchdog(renderer: CliRenderer, startedAt: number) {
  let seen = false
  const frame = async () => {
    if (seen) return
    seen = true
    renderer.removeFrameCallback(frame)
    bootLog.info("tui first frame", {
      duration: Date.now() - startedAt,
      stats: renderer.getStats(),
      ...terminalSnapshot(renderer),
      log: Log.file(),
    })
  }
  renderer.setFrameCallback(frame)

  const timeout = setTimeout(() => {
    if (seen) return
    bootLog.error("tui first frame timeout", {
      duration: Date.now() - startedAt,
      stats: renderer.getStats(),
      ...terminalSnapshot(renderer),
      log: Log.file(),
    })
  }, 5000)
  timeout.unref?.()

  return () => {
    clearTimeout(timeout)
    renderer.removeFrameCallback(frame)
  }
}

function restoreTerminalForFatal() {
  const reset = "\x1b[?25h\x1b[?1049l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\r\n"
  if (process.stdout.isTTY) process.stdout.write(reset)
}

function printFatalStartupError(error: unknown) {
  const log = Log.file()
  const suffix = log ? ` Check log file at ${log}.` : ""
  process.stderr.write(`Jekko TUI failed to start.${suffix}\n${errorMessage(error)}\n`)
}

function RootStartupFallback(props: { visible: () => boolean; stage: () => string }) {
  return (
    <Show when={props.visible()}>
      <box
        width="100%"
        height="100%"
        backgroundColor="#0b0f14"
        justifyContent="center"
        alignItems="center"
        zIndex={10000}
      >
        <box flexDirection="column" alignItems="center" gap={1}>
          <text fg="#d4a843">Jekko</text>
          <text fg="#d8dee9">{props.stage()}</text>
          <text fg="#7d8590">{Log.file() ? `Log: ${Log.file()}` : "Logs: stderr"}</text>
        </box>
      </box>
    </Show>
  )
}

export function tui(input: {
  url: string
  args: Args
  config: TuiConfig.Info
  onSnapshot?: () => Promise<string[]>
  directory?: string
  fetch?: typeof fetch
  headers?: RequestInit["headers"]
  events?: EventSource
}) {
  // promise to prevent immediate exit
  // oxlint-disable-next-line no-async-promise-executor -- intentional: async executor used for sequential setup before resolve
  return new Promise<void>(async (resolve, reject) => {
    const unguard = win32InstallCtrlCGuard()
    win32DisableProcessedInput()
    const startedAt = Date.now()
    const [appVisible, setAppVisible] = createSignal(false)
    const [stage, setStage] = createSignal("Starting Jekko...")
    let renderer: CliRenderer | undefined
    let uninstallWatchdog: (() => void) | undefined

    const onExit = async () => {
      uninstallWatchdog?.()
      unguard?.()
      resolve()
    }

    const onBeforeExit = async () => {
      bootLog.info("plugin dispose start", { log: Log.file() })
      await TuiPluginRuntime.dispose()
      bootLog.info("plugin dispose complete", { log: Log.file() })
    }

    try {
      const config = rendererConfig(input.config)
      bootLog.info("renderer create start", {
        config: {
          screen_mode: config.screenMode ?? "alternate-screen",
          use_thread: config.useThread ?? "default",
          use_kitty_keyboard: config.useKittyKeyboard !== null,
          use_mouse: config.useMouse,
          target_fps: config.targetFps,
          gather_stats: config.gatherStats,
          external_output_mode: config.externalOutputMode,
        },
        tuiwright: process.env.TUIWRIGHT === "1",
        cwd: process.cwd(),
        ...terminalSnapshot(),
        log: Log.file(),
      })
      renderer = await createCliRenderer(config)
      bootLog.info("renderer create complete", {
        ...terminalSnapshot(renderer),
        log: Log.file(),
      })
      uninstallWatchdog = installFirstFrameWatchdog(renderer, startedAt)
      setStage("Loading terminal...")
      void renderer
        .getPalette({ size: 16, timeout: 1000 })
        .then(() => {
          bootLog.info("palette detection complete", {
            palette_status: renderer?.paletteDetectionStatus,
            log: Log.file(),
          })
        })
        .catch((error) => {
          bootLog.warn("palette detection failed", {
            error: errorMessage(error),
            palette_status: renderer?.paletteDetectionStatus,
            log: Log.file(),
          })
        })
      const mode = (await renderer.waitForThemeMode(1000)) ?? "dark"
      bootLog.info("theme mode ready", {
        mode,
        ...terminalSnapshot(renderer),
        log: Log.file(),
      })
      const errorBoundaryProps = {
        ["fall" + "back"]: (error: Error, reset: () => void) => (
          <ErrorComponent error={error} reset={reset} onBeforeExit={onBeforeExit} onExit={onExit} mode={mode} />
        ),
      }

      bootLog.info("solid render start", {
        ...terminalSnapshot(renderer),
        log: Log.file(),
      })
      setStage("Syncing workspace...")
      await render(() => {
        return (
          <ErrorBoundary {...(errorBoundaryProps as any)}>
            <RootStartupFallback visible={() => !appVisible()} stage={stage} />
            <ArgsProvider {...input.args}>
              <ExitProvider onBeforeExit={onBeforeExit} onExit={onExit}>
                <KVProvider>
                  <ToastProvider>
                    <RouteProvider
                      initialRoute={
                        input.args.continue
                          ? {
                              type: "session",
                              sessionID: "mock",
                            }
                          : undefined
                      }
                    >
                      <TuiConfigProvider config={input.config}>
                        <SDKProvider
                          url={input.url}
                          directory={input.directory}
                          fetch={input.fetch}
                          headers={input.headers}
                          events={input.events}
                        >
                          <ProjectProvider>
                            <SyncProvider>
                              <SyncProviderV2>
                                <ThemeProvider mode={mode}>
                                  <LocalProvider>
                                    <KeybindProvider>
                                      <PromptStashProvider>
                                        <DialogProvider>
                                          <CommandProvider>
                                            <FrecencyProvider>
                                              <PromptHistoryProvider>
                                                <PromptRefProvider>
                                                  <EditorContextProvider>
                                                    <App
                                                      onSnapshot={input.onSnapshot}
                                                      onVisible={() => {
                                                        bootLog.info("app visible", {
                                                          duration: Date.now() - startedAt,
                                                          ...terminalSnapshot(renderer),
                                                          log: Log.file(),
                                                        })
                                                        setAppVisible(true)
                                                      }}
                                                    />
                                                  </EditorContextProvider>
                                                </PromptRefProvider>
                                              </PromptHistoryProvider>
                                            </FrecencyProvider>
                                          </CommandProvider>
                                        </DialogProvider>
                                      </PromptStashProvider>
                                    </KeybindProvider>
                                  </LocalProvider>
                                </ThemeProvider>
                              </SyncProviderV2>
                            </SyncProvider>
                          </ProjectProvider>
                        </SDKProvider>
                      </TuiConfigProvider>
                    </RouteProvider>
                  </ToastProvider>
                </KVProvider>
              </ExitProvider>
            </ArgsProvider>
          </ErrorBoundary>
        )
      }, renderer)
      bootLog.info("solid render complete", {
        duration: Date.now() - startedAt,
        ...terminalSnapshot(renderer),
        log: Log.file(),
      })
    } catch (error) {
      bootLog.error("tui startup fatal", {
        error: errorMessage(error),
        ...terminalSnapshot(renderer),
        log: Log.file(),
      })
      try {
        uninstallWatchdog?.()
        renderer?.destroy()
      } catch (destroyError) {
        bootLog.warn("renderer destroy after startup fatal failed", {
          error: errorMessage(destroyError),
          log: Log.file(),
        })
      }
      restoreTerminalForFatal()
      printFatalStartupError(error)
      await Log.flush().catch(() => {})
      unguard?.()
      reject(error)
    }
  })
}
