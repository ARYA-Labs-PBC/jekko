import os from "os"
import { Layer } from "effect"
import { AppFileSystem } from "@jekko-ai/core/filesystem"
import { EffectFlock } from "@jekko-ai/core/util/effect-flock"
import { Global } from "@jekko-ai/core/global"

export type Msg = {
  key: string
  dir: string
  holdMs?: number
  ready?: string
  active?: string
  done?: string
}

export function sleep(ms: number) {
  return new Promise<void>((resolve) => setTimeout(resolve, ms))
}

const testGlobal = Global.layerWith({
  home: os.homedir(),
  data: os.tmpdir(),
  cache: os.tmpdir(),
  config: os.tmpdir(),
  state: os.tmpdir(),
  bin: os.tmpdir(),
  log: os.tmpdir(),
})

export const testLayer = EffectFlock.layer.pipe(Layer.provide(testGlobal), Layer.provide(AppFileSystem.defaultLayer))
