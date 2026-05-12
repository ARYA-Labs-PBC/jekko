import { createMemo, For } from "solid-js"
import { EmptyNotice, PURPLE, type PanelProps } from "./panel-common"
import { fmtN, textBar, truncate } from "./utils"
import { tokenModels } from "./panel-model"

export function TokenVaultPanel(props: PanelProps) {
  const theme = () => props.api.theme.current

  const sorted = createMemo(() => tokenModels(props.snapshot.models, props.state.searchQuery()))
  const maxTok = createMemo(() => Math.max(1, ...sorted().map((m) => m.total_tokens)))
  const totalStolen = createMemo(() => sorted().reduce((s, m) => s + m.total_tokens, 0))

  return (
    <box flexDirection="column" width="100%">
      <box paddingBottom={1}>
        <text fg={theme().text}>
          <b>💰 Token Vault — {fmtN(totalStolen())} Stolen</b>
        </text>
      </box>
      {sorted().length > 0 ? (
        <For each={sorted()}>
          {(model, idx) => {
            const isSelected = () => idx() === props.state.selectedIndex()
            const efficiency =
              model.success_count > 0 ? Math.round(model.total_tokens / model.success_count) : 0
            const bar = textBar(model.total_tokens, maxTok(), 20)

            return (
              <box
                flexDirection="row"
                width="100%"
                gap={1}
                backgroundColor={isSelected() ? theme().backgroundElement : undefined}
                paddingLeft={1}
                paddingRight={1}
              >
                <box flexGrow={1} flexDirection="column">
                  <text fg={isSelected() ? theme().selectedListItemText : theme().text}>
                    {truncate(model.display_name, 24)}
                  </text>
                  <text fg={theme().textMuted}>
                    {fmtN(model.prompt_tokens)} prompt · {fmtN(model.completion_tokens)} comp ·{" "}
                    {fmtN(efficiency)} tok/call
                  </text>
                </box>
                <text fg={PURPLE} flexShrink={0}>
                  {bar}
                </text>
                <text fg={theme().text} flexShrink={0}>
                  {fmtN(model.total_tokens)}
                </text>
              </box>
            )
          }}
        </For>
      ) : (
        <EmptyNotice api={props.api} message="No tokens consumed yet" />
      )}
    </box>
  )
}
