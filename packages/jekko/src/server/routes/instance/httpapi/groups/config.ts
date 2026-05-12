import { Config } from "@/config/config"
import { Provider } from "@/provider/provider"
import { HttpApi, HttpApiEndpoint, HttpApiError, HttpApiGroup, OpenApi } from "effect/unstable/httpapi"
import { described } from "./metadata"
import { withInstanceGroupDefaults } from "./group"

const root = "/config"

export const ConfigApi = HttpApi.make("config")
  .add(
    withInstanceGroupDefaults(
      HttpApiGroup.make("config").add(
        HttpApiEndpoint.get("get", root, {
          success: described(Config.Info, "Get config info"),
        }).annotateMerge(
          OpenApi.annotations({
            identifier: "config.get",
            summary: "Get configuration",
            description: "Retrieve the current Jekko configuration settings and preferences.",
          }),
        ),
        HttpApiEndpoint.patch("update", root, {
          payload: Config.Info,
          success: described(Config.Info, "Successfully updated config"),
          error: HttpApiError.BadRequest,
        }).annotateMerge(
          OpenApi.annotations({
            identifier: "config.update",
            summary: "Update configuration",
            description: "Update Jekko configuration settings and preferences.",
          }),
        ),
        HttpApiEndpoint.get("providers", `${root}/providers`, {
          success: described(Provider.ConfigProvidersResult, "List of providers"),
        }).annotateMerge(
          OpenApi.annotations({
            identifier: "config.providers",
            summary: "List config providers",
            description: "Get a list of all configured AI providers and their default models.",
          }),
        ),
      ),
      "config",
      "Experimental HttpApi config routes.",
    ),
  )
  .annotateMerge(
    OpenApi.annotations({
      title: "jekko experimental HttpApi",
      version: "0.0.1",
      description: "Experimental HttpApi surface for selected instance routes.",
    }),
  )
