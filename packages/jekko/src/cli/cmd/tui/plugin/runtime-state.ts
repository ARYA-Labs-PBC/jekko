import type { RuntimeState } from "./runtime-core"

type RuntimeSession = {
  dir: string
  loaded: Promise<void> | undefined
  runtime: RuntimeState | undefined
}

const session: RuntimeSession = {
  dir: "",
  loaded: undefined,
  runtime: undefined,
}

export function getRuntimeDir() {
  return session.dir
}

export function setRuntimeDir(dir: string) {
  session.dir = dir
}

export function getRuntimeLoad() {
  return session.loaded
}

export function setRuntimeLoad(loaded: Promise<void> | undefined) {
  session.loaded = loaded
}

export function getRuntimeState() {
  return session.runtime
}

export function setRuntimeState(runtime: RuntimeState | undefined) {
  session.runtime = runtime
}
