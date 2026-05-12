import type { Hooks, PluginInput } from "@jekko-ai/plugin"

export async function AzureAuthPlugin(_input: PluginInput): Promise<Hooks> {
  const prompts = []
  if (!process.env.AZURE_RESOURCE_NAME) {
    prompts.push({
      type: "text" as const,
      key: "resourceName",
      message: "Enter Azure Resource Name",
      default_value: "e.g. my-models",
    })
  }

  return {
    auth: {
      provider: "azure",
      methods: [
        {
          type: "api",
          label: "API key",
          prompts,
        },
      ],
    },
  }
}
