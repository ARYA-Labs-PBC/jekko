import { createMemo, onCleanup, getOwner } from "solid-js"
import { Keybind } from "@/util/keybind"
import { pipe, mapValues } from "remeda"
import type { TuiConfig } from "@/cli/cmd/tui/config/tui"
import type { ParsedKey, Renderable } from "@opentui/core"
import { createStore } from "solid-js/store"
import { useKeyboard, useRenderer } from "@opentui/solid"
import { createSimpleContext } from "./helper"
import { useTuiConfig } from "./tui-config"

export type KeybindKey = keyof NonNullable<TuiConfig.Info["keybinds"]> & string

/**
 * Payload passed to every keybind subscriber.
 *
 * - `name`  — the schema-registered keybind name (e.g. "shell.tab.set")
 * - `event` — the raw ParsedKey that matched the chord; subscribers can
 *             inspect it to disambiguate multi-chord bindings (e.g. read
 *             `event.name` to know whether "1", "2", or "3" was pressed
 *             for the `shell.tab.set` binding).
 */
export type KeybindEvent = {
  name: string
  event: ParsedKey
}

export type KeybindHandler = (payload: KeybindEvent) => void

export const { use: useKeybind, provider: KeybindProvider } = createSimpleContext({
  name: "Keybind",
  init: () => {
    const config = useTuiConfig()
    const keybinds = createMemo<Record<string, Keybind.Info[]>>(() => {
      return pipe(
        (config.keybinds ?? {}) as Record<string, string>,
        mapValues((value) => Keybind.parse(value)),
      )
    })
    const [store, setStore] = createStore({
      leader: false,
    })
    const renderer = useRenderer()

    // Lightweight in-process subscriber registry. Phase 4/5/6 components
    // (home, shell tabs, activity-feed) call `keybind.on("name", handler)`
    // to receive events when matched chords fire. `app-bindings.tsx` owns
    // the dispatcher that calls `keybind.emit(...)`. We use a plain Map of
    // Sets so a single binding can have many subscribers, and dispatch is
    // synchronous to avoid frame-skipping for tab/focus events.
    const subscribers = new Map<string, Set<KeybindHandler>>()

    let focus: Renderable | null
    let timeout: NodeJS.Timeout
    function leader(active: boolean) {
      if (active) {
        setStore("leader", true)
        focus = renderer.currentFocusedRenderable
        focus?.blur()
        if (timeout) clearTimeout(timeout)
        timeout = setTimeout(() => {
          if (!store.leader) return
          leader(false)
          if (!focus || focus.isDestroyed) return
          focus.focus()
        }, 2000)
        return
      }

      if (!active) {
        if (focus && !renderer.currentFocusedRenderable) {
          focus.focus()
        }
        setStore("leader", false)
      }
    }

    useKeyboard(async (evt) => {
      if (!store.leader && result.match("leader", evt)) {
        leader(true)
        return
      }

      if (store.leader && evt.name) {
        setImmediate(() => {
          if (focus && renderer.currentFocusedRenderable === focus) {
            focus.focus()
          }
          leader(false)
        })
      }
    })

    const result = {
      get all() {
        return keybinds()
      },
      get leader() {
        return store.leader
      },
      parse(evt: ParsedKey): Keybind.Info {
        // Handle special case for Ctrl+Underscore (represented as \x1F)
        if (evt.name === "\x1F") {
          return Keybind.fromParsedKey({ ...evt, name: "_", ctrl: true }, store.leader)
        }
        return Keybind.fromParsedKey(evt, store.leader)
      },
      match(key: string, evt: ParsedKey) {
        const list = keybinds()[key] ?? Keybind.parse(key)
        if (!list.length) return false
        const parsed: Keybind.Info = result.parse(evt)
        for (const item of list) {
          if (Keybind.match(item, parsed)) {
            return true
          }
        }
        return false
      },
      print(key: string) {
        const first = keybinds()[key]?.at(0) ?? Keybind.parse(key).at(0)
        if (!first) return ""
        const text = Keybind.toString(first)
        const lead = keybinds().leader?.[0]
        if (!lead) return text
        return text.replace("<leader>", Keybind.toString(lead))
      },
      /**
       * Subscribe to a named keybind. Returns an unsubscribe function and
       * auto-cleans on owning component disposal when called inside a Solid
       * reactive scope. The dispatcher in `app-bindings.tsx` invokes these
       * handlers when the bound chord matches the active ParsedKey.
       */
      on(name: string, handler: KeybindHandler): () => void {
        let bucket = subscribers.get(name)
        if (!bucket) {
          bucket = new Set()
          subscribers.set(name, bucket)
        }
        bucket.add(handler)
        const off = () => {
          const b = subscribers.get(name)
          if (!b) return
          b.delete(handler)
          if (b.size === 0) subscribers.delete(name)
        }
        if (getOwner()) onCleanup(off)
        return off
      },
      /**
       * Emit a named keybind to all subscribers. Returns true if at least
       * one handler was invoked (callers in the dispatcher can use this to
       * decide whether to `evt.preventDefault()`).
       */
      emit(name: string, event: ParsedKey): boolean {
        const bucket = subscribers.get(name)
        if (!bucket || bucket.size === 0) return false
        const payload: KeybindEvent = { name, event }
        // Iterate over a copy so unsubscriptions during dispatch are safe.
        for (const handler of Array.from(bucket)) {
          try {
            handler(payload)
          } catch (err) {
            // A faulty subscriber must not break the chain — surface via
            // console.error rather than letting Solid bubble it up and
            // tear down the whole reactive root.
            console.error("[keybind] subscriber for", name, "threw:", err)
          }
        }
        return true
      },
    }
    return result
  },
})
