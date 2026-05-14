/** @jsxImportSource @opentui/solid */
import { expect, test } from "bun:test"
import { RGBA } from "@opentui/core"
import { testRender } from "@opentui/solid"
import { FooterBand } from "../../../src/cli/cmd/tui/component/footer-band"

function rgbaBytes(color: { buffer: ArrayLike<number> }) {
  return Array.from(color.buffer).join(",")
}

test("footer band paints a full-width background behind its content", async () => {
  const background = RGBA.fromHex("#112233")
  const border = RGBA.fromHex("#445566")
  const app = await testRender(
    () => (
      <box width="100%" height="100%">
        <FooterBand backgroundColor={background} borderColor={border}>
          <box width="100%" height={1}>
            <text>hi</text>
          </box>
        </FooterBand>
      </box>
    ),
    { width: 20, height: 5 },
  )

  try {
    await app.renderOnce()
    const frame = app.captureSpans()
    const line = frame.lines.find((row) => row.spans.some((span) => span.text.includes("hi")))
    expect(line).toBeDefined()
    const bandSpan = line!.spans.find((span) => span.width === frame.cols && rgbaBytes(span.bg) === rgbaBytes(background))
    expect(bandSpan).toBeDefined()
  } finally {
    app.renderer.destroy()
  }
})
