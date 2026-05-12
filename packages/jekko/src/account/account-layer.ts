import { Cache, Clock, Duration, Effect, Layer, Option, Schema } from "effect"
import { HttpClient, HttpClientRequest, HttpClientResponse } from "effect/unstable/http"

import { withTransientReadRetry } from "@/util/effect-http-client"
import { AccountRepo, type AccountRow } from "./repo"
import { normalizeServerUrl } from "./url"
import { clientId, ClientId, DeviceAuth, DeviceToken, DeviceTokenError, DeviceTokenRequest, isTokenFresh, mapAccountServiceError, readAccessToken, RemoteConfig, TokenRefresh, TokenRefreshRequest, User } from "./account-helpers"
import { ActiveOrg } from "./account"
import { type AccountError, type AccountID, AccessToken, AccountServiceError, OrgID, Org, Login, PollSuccess } from "./schema"

export const buildAccountLayer = <S>(service: { of: (impl: unknown) => S }) =>
  Layer.effect(
    service,
    Effect.gen(function* () {
      const repo = yield* AccountRepo.Service
      const http = yield* HttpClient.HttpClient
      const httpRead = withTransientReadRetry(http)
      const httpOk = HttpClient.filterStatusOk(http)
      const httpReadOk = HttpClient.filterStatusOk(httpRead)

      const executeRead = (request: HttpClientRequest.HttpClientRequest) =>
        httpRead.execute(request).pipe(mapAccountServiceError("HTTP request failed"))

      const executeReadOk = (request: HttpClientRequest.HttpClientRequest) =>
        httpReadOk.execute(request).pipe(mapAccountServiceError("HTTP request failed"))

      const executeEffectOk = <E>(request: Effect.Effect<HttpClientRequest.HttpClientRequest, E>) =>
        request.pipe(
          Effect.flatMap((req) => httpOk.execute(req)),
          mapAccountServiceError("HTTP request failed"),
        )

      const executeEffect = <E>(request: Effect.Effect<HttpClientRequest.HttpClientRequest, E>) =>
        request.pipe(
          Effect.flatMap((req) => http.execute(req)),
          mapAccountServiceError("HTTP request failed"),
        )

      const refreshToken = Effect.fnUntraced(function* (row: AccountRow) {
        const now = yield* Clock.currentTimeMillis

        const response = yield* executeEffectOk(
          HttpClientRequest.post(`${row.url}/auth/device/token`).pipe(
            HttpClientRequest.acceptJson,
            HttpClientRequest.schemaBodyJson(TokenRefreshRequest)(
              new TokenRefreshRequest({
                grant_type: "refresh_token",
                refresh_token: row.refresh_token,
                client_id: clientId,
              }),
            ),
          ),
        )

        const parsed = yield* HttpClientResponse.schemaBodyJson(TokenRefresh)(response).pipe(
          mapAccountServiceError("Failed to decode response"),
        )

        const refresh = parsed as { expires_in: Duration.Duration; refresh_token: string }
        const expiry = Option.some(now + Duration.toMillis(refresh.expires_in))

        yield* repo.persistToken({
          accountID: row.id,
          token: readAccessToken(refresh),
          refreshToken: refresh.refresh_token,
          expiry,
        } as Parameters<typeof repo.persistToken>[0])

        return readAccessToken(refresh)
      })

      const refreshTokenCache = yield* Cache.make<AccountID, AccessToken, AccountError>({
        capacity: Number.POSITIVE_INFINITY,
        timeToLive: Duration.zero,
        lookup: Effect.fnUntraced(function* (accountID) {
          const maybeAccount = yield* repo.getRow(accountID)
          if (Option.isNone(maybeAccount)) {
            return yield* Effect.fail(new AccountServiceError({ message: "Account not found during token refresh" }))
          }

          const account = maybeAccount.value
          const now = yield* Clock.currentTimeMillis
          if (isTokenFresh(account.token_expiry, now)) {
            return readAccessToken(account as Record<string, unknown>)
          }

          return yield* refreshToken(account)
        }),
      })

      const resolveToken = Effect.fnUntraced(function* (row: AccountRow) {
        const now = yield* Clock.currentTimeMillis
        if (isTokenFresh(row.token_expiry, now)) {
          return readAccessToken(row as Record<string, unknown>)
        }

        return yield* Cache.get(refreshTokenCache, row.id)
      })

      const resolveAccess = Effect.fnUntraced(function* (accountID: AccountID) {
        const maybeAccount = yield* repo.getRow(accountID)
        if (Option.isNone(maybeAccount)) return Option.none()

        const account = maybeAccount.value
        const token = yield* resolveToken(account)
        return Option.some({ account, token })
      })

      const fetchOrgs = Effect.fnUntraced(function* (url: string, token: AccessToken) {
        const response = yield* executeReadOk(
          HttpClientRequest.get(`${url}/api/orgs`).pipe(
            HttpClientRequest.acceptJson,
            HttpClientRequest.bearerToken(token),
          ),
        )

        return yield* HttpClientResponse.schemaBodyJson(Schema.Array(Org))(response).pipe(
          mapAccountServiceError("Failed to decode response"),
        )
      })

      const fetchUser = Effect.fnUntraced(function* (url: string, token: AccessToken) {
        const response = yield* executeReadOk(
          HttpClientRequest.get(`${url}/api/user`).pipe(
            HttpClientRequest.acceptJson,
            HttpClientRequest.bearerToken(token),
          ),
        )

        return yield* HttpClientResponse.schemaBodyJson(User)(response).pipe(
          mapAccountServiceError("Failed to decode response"),
        )
      })

      const token = Effect.fn("Account.token")((accountID: AccountID) =>
        resolveAccess(accountID).pipe(Effect.map(Option.map((r) => r.token))),
      )

      const activeOrg = Effect.fn("Account.activeOrg")(function* () {
        const activeAccount = yield* repo.active()
        if (Option.isNone(activeAccount)) return Option.none<ActiveOrg>()

        const account = activeAccount.value
        if (!account.active_org_id) return Option.none<ActiveOrg>()

        const accountOrgs = yield* orgs(account.id)
        const org = accountOrgs.find((item) => item.id === account.active_org_id)
        if (!org) return Option.none<ActiveOrg>()

        return Option.some({ account, org })
      })

      const orgsByAccount = Effect.fn("Account.orgsByAccount")(function* () {
        const accounts = yield* repo.list()
        return yield* Effect.forEach(
          accounts,
          (account) =>
            orgs(account.id).pipe(
              Effect.catch(() => Effect.succeed([] as readonly Org[])),
              Effect.map((orgs) => ({ account, orgs })),
            ),
          { concurrency: 3 },
        )
      })

      const orgs = Effect.fn("Account.orgs")(function* (accountID: AccountID) {
        const resolved = yield* resolveAccess(accountID)
        if (Option.isNone(resolved)) return []

        const { account, token } = resolved.value

        return yield* fetchOrgs(account.url, token)
      })

      const config = Effect.fn("Account.config")(function* (accountID: AccountID, orgID: OrgID) {
        const resolved = yield* resolveAccess(accountID)
        if (Option.isNone(resolved)) return Option.none()

        const { account, token } = resolved.value

        const response = yield* executeRead(
          HttpClientRequest.get(`${account.url}/api/config`).pipe(
            HttpClientRequest.acceptJson,
            HttpClientRequest.bearerToken(token),
            HttpClientRequest.setHeaders({ "x-org-id": orgID }),
          ),
        )

        if (response.status === 404) return Option.none()

        const ok = yield* HttpClientResponse.filterStatusOk(response).pipe(mapAccountServiceError())

        const parsed = yield* HttpClientResponse.schemaBodyJson(RemoteConfig)(ok).pipe(
          mapAccountServiceError("Failed to decode response"),
        )
        return Option.some(parsed.config)
      })

      const login = Effect.fn("Account.login")(function* (server: string) {
        const normalizedServer = normalizeServerUrl(server)
        const response = yield* executeEffectOk(
          HttpClientRequest.post(`${normalizedServer}/auth/device/code`).pipe(
            HttpClientRequest.acceptJson,
            HttpClientRequest.schemaBodyJson(ClientId)(new ClientId({ client_id: clientId })),
          ),
        )

        const parsed = yield* HttpClientResponse.schemaBodyJson(DeviceAuth)(response).pipe(
          mapAccountServiceError("Failed to decode response"),
        )
        return new Login({
          code: parsed.device_code,
          user: parsed.user_code,
          url: `${normalizedServer}${parsed.verification_uri_complete}`,
          server: normalizedServer,
          expiry: parsed.expires_in,
          interval: parsed.interval,
        })
      })

      const poll = Effect.fn("Account.poll")(function* (input: Login) {
        const response = yield* executeEffect(
          HttpClientRequest.post(`${input.server}/auth/device/token`).pipe(
            HttpClientRequest.acceptJson,
            HttpClientRequest.schemaBodyJson(DeviceTokenRequest)(
              new DeviceTokenRequest({
                grant_type: "urn:ietf:params:oauth:grant-type:device_code",
                device_code: input.code,
                client_id: clientId,
              }),
            ),
          ),
        )

        const parsed = yield* HttpClientResponse.schemaBodyJson(DeviceToken)(response).pipe(
          mapAccountServiceError("Failed to decode response"),
        )

        if (parsed instanceof DeviceTokenError) return parsed.toPollResult()
        const success = parsed as { expires_in: Duration.Duration; refresh_token: string }
        const token = readAccessToken(success as Record<string, unknown>)

        const user = fetchUser(input.server, token)
        const orgs = fetchOrgs(input.server, token)

        const [account, remoteOrgs] = yield* Effect.all([user, orgs], { concurrency: 2 })

        // pending: When there are multiple orgs, let the user choose
        const firstOrgID = remoteOrgs.length > 0 ? Option.some(remoteOrgs[0].id) : Option.none<OrgID>()

        const now = yield* Clock.currentTimeMillis
        const expiry = now + Duration.toMillis(success.expires_in)
        const refreshToken = success.refresh_token

        yield* repo.persistAccount({
          id: account.id,
          email: account.email,
          url: input.server,
          token,
          refreshToken,
          expiry,
          orgID: firstOrgID,
        } as Parameters<typeof repo.persistAccount>[0])

        return new PollSuccess({ email: account.email })
      })

        return service.of({ active: repo.active, activeOrg, list: repo.list, orgsByAccount, remove: repo.remove, use: repo.use, orgs, config, token, login, poll })
    }),
  )

