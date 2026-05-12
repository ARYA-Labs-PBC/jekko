import { batch, createEffect, createMemo } from "solid-js"
import { createStore } from "solid-js/store"
import { uniqueBy } from "remeda"
import path from "path"
import { Global } from "@jekko-ai/core/global"
import { iife } from "@/util/iife"
import { Filesystem } from "@/util/filesystem"
import { useArgs } from "./args"
import {
  chooseModelResolution,
  resolveModelChoice,
  resolveProviderModel,
  resolveRecentModel,
  resolvedModelResolution,
  type ModelResolution,
  type ProviderModel,
} from "./local-model-resolution"

type LocalModelDeps = {
  sync: any
  sdk: any
  toast: any
  agent: {
    current: () => { name: string; model?: ProviderModel } | undefined
  }
  parseModel: (model: string) => ProviderModel
  isModelValid: (model: ProviderModel) => boolean
}

export function createLocalModel({ sync, sdk, toast, agent, parseModel, isModelValid }: LocalModelDeps) {
  return iife(() => {
    const [modelStore, setModelStore] = createStore<{
      ready: boolean
      model: Record<string, ProviderModel>
      recent: ProviderModel[]
      favorite: ProviderModel[]
      variant: Record<string, string | undefined>
    }>({
      ready: false,
      model: {},
      recent: [],
      favorite: [],
      variant: {},
    })

    const filePath = path.join(Global.Path.state, "model.json")
    const state = { pending: false }

    function save() {
      if (!modelStore.ready) {
        state.pending = true
        return
      }
      state.pending = false
      void Filesystem.writeJson(filePath, {
        recent: modelStore.recent,
        favorite: modelStore.favorite,
        variant: modelStore.variant,
      })
    }

    Filesystem.readJson(filePath)
      .then((x: any) => {
        if (Array.isArray(x.recent)) setModelStore("recent", x.recent)
        if (Array.isArray(x.favorite)) setModelStore("favorite", x.favorite)
        if (typeof x.variant === "object" && x.variant !== null) setModelStore("variant", x.variant)
      })
      .catch(() => {})
      .finally(() => {
        setModelStore("ready", true)
        if (state.pending) save()
      })

    const args = useArgs()

    const fallbackModel = createMemo(() =>
      chooseModelResolution([
        resolveModelChoice(args.model, { source: "args", parseModel, isModelValid }),
        resolveModelChoice(sync.data.config.model, { source: "config", parseModel, isModelValid }),
        resolveRecentModel(modelStore.recent, isModelValid),
        resolveProviderModel(sync),
      ]),
    )

    const currentModel = createMemo<ModelResolution>(() => {
      const a = agent.current()
      if (a && modelStore.model[a.name]) return resolvedModelResolution("agent", modelStore.model[a.name])
      if (a && a.model && isModelValid(a.model)) return resolvedModelResolution("agent-model", a.model)
      return fallbackModel()
    })

    const currentResolvedModel = createMemo(() => {
      const value = currentModel()
      return value.kind === "resolved" ? value.model : undefined
    })

    function selectedVariantKey() {
      const m = currentResolvedModel()
      return `${m?.providerID ?? ""}/${m?.modelID ?? ""}`
    }

    let lastMissingModelNotice = ""
    let lastMissingModelNoticeAt = 0
    createEffect(() => {
      const value = currentModel()
      if (value.kind === "resolved") return

      const now = Date.now()
      const key = `${value.source}:${value.reason}`
      if (lastMissingModelNotice === key && now - lastMissingModelNoticeAt < value.retryAfterMs) return

      lastMissingModelNotice = key
      lastMissingModelNoticeAt = now
      toast.show({
        variant: "info",
        message: `${value.reason}. ${value.repairHint}`,
        duration: 4000,
      })
    })

    return {
      current: currentResolvedModel,
      resolution: currentModel,
      get ready() {
        return modelStore.ready
      },
      recent() {
        return modelStore.recent
      },
      favorite() {
        return modelStore.favorite
      },
      parsed: createMemo(() => {
        const value = currentModel()
        if (value.kind === "missing") {
          return {
            provider: "Connect a provider",
            model: value.repairHint,
            reasoning: false,
          }
        }
        const provider = sync.data.provider.find((x: any) => x.id === value.model.providerID)
        const info = provider?.models[value.model.modelID]
        return {
          provider: provider?.name ?? value.model.providerID,
          model: info?.name ?? value.model.modelID,
          reasoning: info?.capabilities?.reasoning ?? false,
        }
      }),
      cycle(direction: 1 | -1) {
        const current = currentModel()
        if (current.kind === "missing") return
        const recent = modelStore.recent
        const index = recent.findIndex((x) => x.providerID === current.model.providerID && x.modelID === current.model.modelID)
        if (index === -1) return
        let next = index + direction
        if (next < 0) next = recent.length - 1
        if (next >= recent.length) next = 0
        const val = recent[next]
        if (!val) return
        const a = agent.current()
        if (!a) return
        setModelStore("model", a.name, { ...val })
      },
      cycleFavorite(direction: 1 | -1) {
        const favorites = modelStore.favorite.filter((item) => isModelValid(item))
        if (!favorites.length) {
          toast.show({
            variant: "info",
            message: "Add a favorite model to use this shortcut",
            duration: 3000,
          })
          return
        }
        const current = currentModel()
        let index = -1
        if (current.kind === "resolved") {
          index = favorites.findIndex((x) => x.providerID === current.model.providerID && x.modelID === current.model.modelID)
        }
        if (index === -1) {
          index = direction === 1 ? 0 : favorites.length - 1
        } else {
          index += direction
          if (index < 0) index = favorites.length - 1
          if (index >= favorites.length) index = 0
        }
        const next = favorites[index]
        if (!next) return
        const a = agent.current()
        if (!a) return
        setModelStore("model", a.name, { ...next })
        const uniq = uniqueBy([next, ...modelStore.recent], (x) => `${x.providerID}/${x.modelID}`)
        if (uniq.length > 10) uniq.pop()
        setModelStore(
          "recent",
          uniq.map((x) => ({ providerID: x.providerID, modelID: x.modelID })),
        )
        save()
      },
      set(model: ProviderModel, options?: { recent?: boolean }) {
        batch(() => {
          if (!isModelValid(model)) {
            toast.show({
              message: `Model ${model.providerID}/${model.modelID} is not valid`,
              variant: "warning",
              duration: 3000,
            })
            return
          }
          const a = agent.current()
          if (!a) return
          setModelStore("model", a.name, model)
          if (options?.recent) {
            const uniq = uniqueBy([model, ...modelStore.recent], (x) => `${x.providerID}/${x.modelID}`)
            if (uniq.length > 10) uniq.pop()
            setModelStore(
              "recent",
              uniq.map((x) => ({ providerID: x.providerID, modelID: x.modelID })),
            )
            save()
          }
        })
      },
      toggleFavorite(model: ProviderModel) {
        batch(() => {
          if (!isModelValid(model)) {
            toast.show({
              message: `Model ${model.providerID}/${model.modelID} is not valid`,
              variant: "warning",
              duration: 3000,
            })
            return
          }
          const exists = modelStore.favorite.some(
            (x) => x.providerID === model.providerID && x.modelID === model.modelID,
          )
          const next = exists
            ? modelStore.favorite.filter((x) => x.providerID !== model.providerID || x.modelID !== model.modelID)
            : [model, ...modelStore.favorite]
          setModelStore(
            "favorite",
            next.map((x) => ({ providerID: x.providerID, modelID: x.modelID })),
          )
          save()
        })
      },
      variant: {
        selected() {
          return modelStore.variant[selectedVariantKey()]
        },
        current() {
          const v = this.selected()
          return v && this.list().includes(v) ? v : undefined
        },
        list() {
          const m = currentResolvedModel()
          if (!m) return []
          const provider = sync.data.provider.find((x: any) => x.id === m.providerID)
          const info = provider?.models[m.modelID]
          return info?.variants ? Object.keys(info.variants) : []
        },
        set(value: string | undefined) {
          const m = currentResolvedModel()
          if (!m) return
          const key = `${m.providerID}/${m.modelID}`
          setModelStore("variant", key, value ?? "default")
          save()
        },
        cycle() {
          const variants = this.list()
          if (variants.length === 0) return
          const current = this.current()
          if (!current) {
            this.set(variants[0])
            return
          }
          const index = variants.indexOf(current)
          if (index === -1 || index === variants.length - 1) {
            this.set(undefined)
            return
          }
          this.set(variants[index + 1])
        },
      },
    }
  })
}
