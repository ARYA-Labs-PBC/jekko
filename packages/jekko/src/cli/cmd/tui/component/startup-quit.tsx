import { useKeyboard } from "@opentui/solid"

export function useStartupQuit(onQuit?: () => void) {
  useKeyboard((evt) => {
    if (evt.defaultPrevented) return
    if (evt.name?.toLowerCase() !== "q") return
    evt.preventDefault()
    onQuit?.()
  })
}
