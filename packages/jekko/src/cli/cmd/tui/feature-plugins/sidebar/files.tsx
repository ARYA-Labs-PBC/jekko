import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { createMemo, For, Show, createSignal } from "solid-js"

const id = "internal:sidebar-files"

const PREVIEW_LIMIT = 10

function View(props: { api: TuiPluginApi; session_id: string }) {
  const theme = () => props.api.theme.current
  const list = createMemo(() => props.api.state.session.diff(props.session_id))
  const isLong = createMemo(() => list().length > PREVIEW_LIMIT)
  const [expanded, setExpanded] = createSignal(false)
  const visible = createMemo(() => {
    if (!isLong() || expanded()) return list()
    return list().slice(0, PREVIEW_LIMIT)
  })
  const hiddenCount = createMemo(() => Math.max(0, list().length - PREVIEW_LIMIT))

  return (
    <Show when={list().length > 0}>
      <box>
        <box
          flexDirection="row"
          gap={1}
          onMouseDown={() => isLong() && setExpanded((x) => !x)}
        >
          <Show when={isLong()}>
            <text fg={theme().text}>{expanded() ? "▼" : "▶"}</text>
          </Show>
          <text fg={theme().text}>
            <b>Modified Files</b>
          </text>
          <text fg={theme().textMuted}>({list().length})</text>
        </box>
        <For each={visible()}>
          {(item) => (
            <box flexDirection="row" gap={1} justifyContent="space-between">
              <text fg={theme().textMuted} wrapMode="none">
                {item.file}
              </text>
              <box flexDirection="row" gap={1} flexShrink={0}>
                <Show when={item.additions}>
                  <text fg={theme().diffAdded}>+{item.additions}</text>
                </Show>
                <Show when={item.deletions}>
                  <text fg={theme().diffRemoved}>-{item.deletions}</text>
                </Show>
              </box>
            </box>
          )}
        </For>
        <Show when={isLong() && !expanded()}>
          <text
            fg={theme().textMuted}
            onMouseDown={() => setExpanded(true)}
          >
            … {hiddenCount()} more files
          </text>
        </Show>
      </box>
    </Show>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 500,
    slots: {
      sidebar_content(_ctx, props) {
        return <View api={api} session_id={props.session_id} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
