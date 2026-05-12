/** @jsxImportSource @opentui/solid */
import { describe, expect, test } from "bun:test"
import { testRender } from "@opentui/solid"
import { Global } from "@jekko-ai/core/global"
import { onMount } from "solid-js"
import { KVProvider } from "../../../../src/cli/cmd/tui/context/kv"
import { DialogProvider, useDialog } from "../../../../src/cli/cmd/tui/ui/dialog"
import { DialogSecretPrompt } from "../../../../src/cli/cmd/tui/ui/dialog-secret-prompt"
import { ToastProvider } from "../../../../src/cli/cmd/tui/ui/toast"
import { ThemeProvider } from "../../../../src/cli/cmd/tui/context/theme"
import { TuiConfigProvider } from "../../../../src/cli/cmd/tui/context/tui-config"
import { tmpdir } from "../../../fixture/fixture"

const secret = "A".repeat(128)

async function wait(fn: () => boolean, timeout = 2000) {
  const start = Date.now()
  while (!fn()) {
    if (Date.now() - start > timeout) throw new Error("timed out waiting for condition")
    await Bun.sleep(10)
  }
}

async function renderPrompt(onConfirm: (value: string) => void) {
  return await testRender(() => (
    <KVProvider>
      <ToastProvider>
        <TuiConfigProvider config={{}}>
          <ThemeProvider mode="dark">
            <DialogProvider>
              <OpenPrompt onConfirm={onConfirm} />
            </DialogProvider>
          </ThemeProvider>
        </TuiConfigProvider>
      </ToastProvider>
    </KVProvider>
  ))
}

function OpenPrompt(props: { onConfirm: (value: string) => void }) {
  const dialog = useDialog()
  onMount(() => {
    dialog.replace(() => <DialogSecretPrompt title="Unlock Jnoccio Fusion" onConfirm={props.onConfirm} />)
  })
  return <box />
}

describe("DialogSecretPrompt", () => {
  test("submits bracketed paste input without terminal paste artifacts", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    let submitted: string | undefined
    const app = await renderPrompt((value) => {
      submitted = value
    })

    try {
      await app.renderOnce()
      await wait(() => app.captureCharFrame().includes("Paste your 128-character unlock secret"))
      await app.mockInput.pasteBracketedText(secret)
      await app.renderOnce()
      await Bun.sleep(20)
      app.mockInput.pressEnter()
      await wait(() => submitted !== undefined)

      expect(submitted).toBe(secret)
      expect(submitted).toHaveLength(128)
      expect(submitted).not.toContain("\n")
      expect(submitted).not.toContain("%")
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })

  test("sanitizes whitespace and percent bytes from pasted secrets before submit", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    let submitted: string | undefined
    const app = await renderPrompt((value) => {
      submitted = value
    })

    try {
      await app.renderOnce()
      await wait(() => app.captureCharFrame().includes("Paste your 128-character unlock secret"))
      await app.mockInput.pasteBracketedText(` \n%${secret.slice(0, 32)}\r\n${secret.slice(32, 96)}%${secret.slice(96)}\n `)
      await app.renderOnce()
      await Bun.sleep(20)
      app.mockInput.pressEnter()
      await wait(() => submitted !== undefined)

      expect(submitted).toBe(secret)
      expect(submitted).toHaveLength(128)
      expect(submitted).not.toContain("\n")
      expect(submitted).not.toContain("%")
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })

  test("strips keyboard-delivered bracketed paste marker remnants before submit", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    let submitted: string | undefined
    const app = await renderPrompt((value) => {
      submitted = value
    })

    try {
      await app.renderOnce()
      await wait(() => app.captureCharFrame().includes("Paste your 128-character unlock secret"))
      await app.mockInput.typeText(`200${secret}201`)
      await wait(() => app.captureCharFrame().includes("128/128"))
      app.mockInput.pressEnter()
      await wait(() => submitted !== undefined)

      expect(submitted).toBe(secret)
      expect(submitted).toHaveLength(128)
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })
})
