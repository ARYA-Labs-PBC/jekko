import { createMemo, For } from "solid-js"
import { RGBA } from "@opentui/core"
import { EmptyNotice, type PanelProps } from "./panel-common"
import { activeAgents } from "./panel-model"
import { agentColor, fmtN, fmtTime, truncate } from "./utils"

export function AgentsPanel(props: PanelProps) {
  const theme = () => props.api.theme.current
  const agents = createMemo(() => activeAgents(props.snapshot.active_agents))

  return (
    <box flexDirection="column" width="100%">
      <box flexDirection="row" justifyContent="space-between" paddingBottom={1}>
        <text fg={theme().text}>
          <b>⚙ Active Agents — {props.snapshot.agent_count}/{props.snapshot.max_agents}</b>
        </text>
        <text fg={theme().textMuted}>
          Instances: x{props.snapshot.instance_count}/{props.snapshot.max_instances}
          {" · "}
          {props.snapshot.instance_role}
          {props.snapshot.worker_threads > 0 ? ` · ${props.snapshot.worker_threads} threads` : ""}
        </text>
      </box>

      <box flexDirection="row" width="100%" gap={1} paddingLeft={1}>
        <text fg={theme().textMuted} width={20}>
          Agent ID
        </text>
        <text fg={theme().textMuted} width={12}>
          Client
        </text>
        <text fg={theme().textMuted} width={8}>
          PID
        </text>
        <text fg={theme().textMuted} width={10}>
          Version
        </text>
        <text fg={theme().textMuted} width={8}>
          Calls
        </text>
        <text fg={theme().textMuted} width={10}>
          Last Seen
        </text>
      </box>

      {agents().length > 0 ? (
        <For each={agents()}>
          {(agent, idx) => {
            const isSelected = () => idx() === props.state.selectedIndex()
            const color = RGBA.fromHex(agentColor(agent.agent_id))
            return (
              <box
                flexDirection="row"
                width="100%"
                gap={1}
                backgroundColor={isSelected() ? theme().backgroundElement : undefined}
                paddingLeft={1}
              >
                <text fg={color} width={20}>
                  {truncate(agent.agent_id, 18)}
                </text>
                <text fg={theme().text} width={12}>
                  {truncate(agent.agent_client ?? "—", 10)}
                </text>
                <text fg={theme().text} width={8}>
                  {agent.pid ?? "—"}
                </text>
                <text fg={theme().text} width={10}>
                  {truncate(agent.version ?? "—", 8)}
                </text>
                <text fg={theme().text} width={8}>
                  {fmtN(agent.request_count)}
                </text>
                <text fg={theme().textMuted} width={10}>
                  {fmtTime(agent.last_seen)}
                </text>
              </box>
            )
          }}
        </For>
      ) : (
        <EmptyNotice api={props.api} message="No active agents" />
      )}
    </box>
  )
}
