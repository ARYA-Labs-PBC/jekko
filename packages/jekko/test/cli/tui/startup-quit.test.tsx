/** @jsxImportSource @opentui/solid */
import { expect, test } from "bun:test"
import { testRender } from "@opentui/solid"
import { useStartupQuit } from "../../../src/cli/cmd/tui/component/startup-quit"

async function wait(fn: () => boolean, timeout = 2000) {
  const start = Date.now()
  while (!fn()) {
    if (Date.now() - start > timeout) throw new Error("timed out waiting for condition")
    await Bun.sleep(10)
  }
}

function Probe(props: { onQuit: () => void }) {
  useStartupQuit(props.onQuit)
  return <box />
}

test("startup load screen quits on q", async () => {
  let quitCount = 0
  const app = await testRender(() => <Probe onQuit={() => quitCount++} />, { width: 20, height: 5 })

  try {
    await app.renderOnce()
    await app.mockInput.typeText("q")
    await wait(() => quitCount === 1)
    expect(quitCount).toBe(1)
  } finally {
    app.renderer.destroy()
  }
})
