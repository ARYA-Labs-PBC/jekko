/** @jsxImportSource @opentui/solid */
import { VignetteEffect } from "@opentui/core"
import type { TuiPlugin, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { cfg, bind, names } from "./tui-smoke-shared"
import { Modal, Screen } from "./tui-smoke-render"
import { reg, slot } from "./tui-smoke-slots"

const tui: TuiPlugin = async (api, options, meta) => {
  if (options?.enabled === false) return

  await api.theme.install("./smoke-theme.json")
  api.theme.set("smoke-theme")

  const value = cfg(options ?? undefined)
  const route = names(value)
  const keys = api.keybind.create(bind, value.keybinds)
  const fx = new VignetteEffect(value.vignette)
  const post = fx.apply.bind(fx)
  api.renderer.addPostProcessFn(post)
  api.lifecycle.onDispose(() => {
    api.renderer.removePostProcessFn(post)
  })

  api.route.register([
    {
      name: route.screen,
      render: ({ params }) => <Screen api={api} input={value} route={route} keys={keys} meta={meta} params={params} />,
    },
    {
      name: route.modal,
      render: ({ params }) => <Modal api={api} input={value} route={route} keys={keys} params={params} />,
    },
  ])

  reg(api, value, keys)
  for (const item of slot(api, value)) {
    api.slots.register(item)
  }
}

const plugin: TuiPluginModule & { id: string } = {
  id: "tui-smoke",
  tui,
}

export default plugin
