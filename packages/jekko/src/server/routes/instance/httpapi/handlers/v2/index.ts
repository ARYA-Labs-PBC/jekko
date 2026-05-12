import { SessionV2 } from "@/v2/session"
import { Layer } from "effect"
import { messageHandlers } from "./message"
import { sessionHandlers } from "./session"

export const v2Handlers = Layer.mergeAll(sessionHandlers, messageHandlers).pipe(Layer.provide(SessionV2.defaultLayer))
