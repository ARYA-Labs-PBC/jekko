import { Effect, Layer } from "effect"
import { InstanceStore } from "./instance-store"
import { InstanceBootstrap } from "./bootstrap"

export const layer = Layer.unwrap(
  Effect.promise(async () =>
    InstanceStore.defaultLayer.pipe(Layer.provide(InstanceBootstrap.defaultLayer)),
  ),
)

export * as InstanceLayer from "./instance-layer"
