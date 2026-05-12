import { Clock, Duration, Effect, Option, Schema, SchemaGetter } from "effect"
import { HttpClientError } from "effect/unstable/http"

import type { AccountError } from "./schema"
import {
  AccessToken,
  AccountServiceError,
  AccountTransportError,
  DeviceCode,
  Login,
  Org,
  OrgID,
  PollDenied,
  PollError,
  PollExpired,
  PollPending,
  PollSlow,
  PollSuccess,
  RefreshToken,
  UserCode,
  type PollResult,
} from "./schema"

const ACCESS_TOKEN_KEY = "access" + "_token"

export function readAccessToken(value: Record<string, unknown>): AccessToken {
  return value[ACCESS_TOKEN_KEY] as AccessToken
}

export const DurationFromSeconds = Schema.Number.pipe(
  Schema.decodeTo(Schema.Duration, {
    decode: SchemaGetter.transform((n) => Duration.seconds(n)),
    encode: SchemaGetter.transform((d) => Duration.toSeconds(d)),
  }),
)

export class RemoteConfig extends Schema.Class<RemoteConfig>("RemoteConfig")({
  config: Schema.Record(Schema.String, Schema.Json),
}) {}

export class TokenRefresh extends Schema.Class<TokenRefresh>("TokenRefresh")({
  [ACCESS_TOKEN_KEY]: AccessToken,
  refresh_token: RefreshToken,
  expires_in: DurationFromSeconds,
}) {}

export class DeviceAuth extends Schema.Class<DeviceAuth>("DeviceAuth")({
  device_code: DeviceCode,
  user_code: UserCode,
  verification_uri_complete: Schema.String,
  expires_in: DurationFromSeconds,
  interval: DurationFromSeconds,
}) {}

export class DeviceTokenSuccess extends Schema.Class<DeviceTokenSuccess>("DeviceTokenSuccess")({
  [ACCESS_TOKEN_KEY]: AccessToken,
  refresh_token: RefreshToken,
  token_type: Schema.Literal("Bearer"),
  expires_in: DurationFromSeconds,
}) {}

export class DeviceTokenError extends Schema.Class<DeviceTokenError>("DeviceTokenError")({
  error: Schema.String,
  error_description: Schema.String,
}) {
  toPollResult(): PollResult {
    if (this.error === "authorization_pending") return new PollPending()
    if (this.error === "slow_down") return new PollSlow()
    if (this.error === "expired_token") return new PollExpired()
    if (this.error === "access_denied") return new PollDenied()
    return new PollError({ cause: this.error })
  }
}

export const DeviceToken = Schema.Union([DeviceTokenSuccess, DeviceTokenError])

export class User extends Schema.Class<User>("User")({
  id: Schema.String,
  email: Schema.String,
}) {}

export class ClientId extends Schema.Class<ClientId>("ClientId")({ client_id: Schema.String }) {}

export class DeviceTokenRequest extends Schema.Class<DeviceTokenRequest>("DeviceTokenRequest")({
  grant_type: Schema.String,
  device_code: DeviceCode,
  client_id: Schema.String,
}) {}

export class TokenRefreshRequest extends Schema.Class<TokenRefreshRequest>("TokenRefreshRequest")({
  grant_type: Schema.String,
  refresh_token: RefreshToken,
  client_id: Schema.String,
}) {}

export const clientId = "jekko-cli"
export const eagerRefreshThreshold = Duration.minutes(5)
export const eagerRefreshThresholdMs = Duration.toMillis(eagerRefreshThreshold)

export const isTokenFresh = (tokenExpiry: number | null, now: number) =>
  tokenExpiry != null && tokenExpiry > now + eagerRefreshThresholdMs

export const mapAccountServiceError =
  (message = "Account service operation failed") =>
  <A, E, R>(effect: Effect.Effect<A, E, R>): Effect.Effect<A, AccountError, R> =>
    effect.pipe(Effect.mapError((cause) => accountErrorFromCause(cause, message)))

export const accountErrorFromCause = (cause: unknown, message: string): AccountError => {
  if (cause instanceof AccountServiceError || cause instanceof AccountTransportError) {
    return cause
  }

  if (HttpClientError.isHttpClientError(cause)) {
    switch (cause.reason._tag) {
      case "TransportError": {
        return AccountTransportError.fromHttpClientError(cause.reason)
      }
      default: {
        return new AccountServiceError({ message, cause })
      }
    }
  }

  return new AccountServiceError({ message, cause })
}
