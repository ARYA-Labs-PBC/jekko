import type { RGBA } from "@opentui/core"
import type { JSX } from "@opentui/solid"

export function FooterBand(props: {
  backgroundColor: RGBA
  borderColor: RGBA
  children: JSX.Element
}) {
  return (
    <box
      width="100%"
      flexShrink={0}
      backgroundColor={props.backgroundColor}
      border={["top"]}
      borderColor={props.borderColor}
    >
      {props.children}
    </box>
  )
}
