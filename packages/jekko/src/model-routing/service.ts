import { Effect } from "effect"
import { ulid } from "ulid"
import { readModelKeyStatuses } from "@/model-setup/model-keys"
import { routePublicModel } from "./public-router"
import type { ProviderModelRef, RoutingDecision, RoutingPurpose } from "./types"

export function route(input: {
  purpose: RoutingPurpose
  requested: ProviderModelRef
}): Effect.Effect<RoutingDecision> {
  return Effect.promise(() => readModelKeyStatuses()).pipe(
    Effect.map((status) =>
      routePublicModel({
        purpose: input.purpose,
        requested: input.requested,
        statuses: status.statuses,
        developerUnlocked: status.developerUnlocked,
        receiptID: ulid(),
      }),
    ),
  )
}
