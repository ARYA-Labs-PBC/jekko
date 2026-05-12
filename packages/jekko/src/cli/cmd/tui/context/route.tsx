import { createSignal } from "solid-js"
import { createStore, reconcile } from "solid-js/store"
import { createSimpleContext } from "./helper"
import type { PromptInfo } from "../component/prompt/history"

export type HomeRoute = {
  type: "home"
  prompt?: PromptInfo
}

export type SessionRoute = {
  type: "session"
  sessionID: string
  prompt?: PromptInfo
}

export type PluginRoute = {
  type: "plugin"
  id: string
  data?: Record<string, unknown>
}

export type Route = HomeRoute | SessionRoute | PluginRoute

export const { use: useRoute, provider: RouteProvider } = createSimpleContext({
  name: "Route",
  init: (props: { initialRoute?: Route }) => {
    const [store, setStore] = createStore<Route>(
      props.initialRoute ??
        (process.env["JEKKO_ROUTE"]
          ? JSON.parse(process.env["JEKKO_ROUTE"])
          : {
              type: "home",
            }),
    )

    const [previousRoute, setPreviousRoute] = createSignal<Route | null>(null)

    return {
      get data() {
        return store
      },
      get previous() {
        return previousRoute()
      },
      navigate(route: Route) {
        // Save the current route as previous before switching
        const current = { ...store } as Route
        setPreviousRoute(current)
        setStore(reconcile(route))
      },
      /** Navigate back to the previous route. Falls back to Home. */
      navigateBack() {
        const prev = previousRoute()
        if (prev && prev.type !== "home") {
          setPreviousRoute(null)
          setStore(reconcile(prev))
        } else {
          setPreviousRoute(null)
          setStore(reconcile({ type: "home" }))
        }
      },
    }
  },
})

export type RouteContext = ReturnType<typeof useRoute>

export function useRouteData<T extends Route["type"]>(type: T) {
  const route = useRoute()
  return route.data as Extract<Route, { type: typeof type }>
}
