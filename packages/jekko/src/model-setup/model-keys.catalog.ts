export type CatalogEntry = {
  providerID: string
  envNames: string[]
  signupUrl?: string
  recommendedModelID?: string
  priority: number
  companionEnvNames?: string[]
}

export const CATALOG: CatalogEntry[] = [
  {
    providerID: "openai",
    envNames: ["OPENAI_API_KEY"],
    signupUrl: "https://platform.openai.com/api-keys",
    recommendedModelID: "gpt-5.3-codex",
    priority: 90,
  },
  {
    providerID: "anthropic",
    envNames: ["ANTHROPIC_API_KEY"],
    signupUrl: "https://console.anthropic.com/settings/keys",
    recommendedModelID: "claude-sonnet-4-5",
    priority: 88,
  },
  {
    providerID: "google",
    envNames: ["GOOGLE_GENERATIVE_AI_API_KEY", "GEMINI_API_KEY", "GOOGLE_API_KEY"],
    signupUrl: "https://aistudio.google.com/apikey",
    recommendedModelID: "gemini-2.5-flash",
    priority: 86,
  },
  {
    providerID: "openrouter",
    envNames: ["OPENROUTER_API_KEY"],
    signupUrl: "https://openrouter.ai/keys",
    recommendedModelID: "openrouter-gpt-oss-120b-free",
    priority: 80,
  },
  {
    providerID: "groq",
    envNames: ["GROQ_API_KEY"],
    signupUrl: "https://console.groq.com/keys",
    recommendedModelID: "groq-qwen3-32b",
    priority: 78,
  },
  {
    providerID: "cerebras",
    envNames: ["CEREBRAS_API_KEY"],
    signupUrl: "https://cloud.cerebras.ai",
    recommendedModelID: "cerebras-qwen-3-235b-a22b-instruct-2507",
    priority: 77,
  },
  {
    providerID: "mistral",
    envNames: ["MISTRAL_API_KEY"],
    signupUrl: "https://console.mistral.ai/api-keys",
    recommendedModelID: "mistral-devstral-latest",
    priority: 76,
  },
  {
    providerID: "github",
    envNames: ["GITHUB_TOKEN"],
    signupUrl: "https://github.com/marketplace/models",
    recommendedModelID: "github-codestral-2501",
    priority: 75,
  },
  {
    providerID: "nvidia",
    envNames: ["NVIDIA_API_KEY"],
    signupUrl: "https://build.nvidia.com",
    recommendedModelID: "nvidia-deepseek-v4-pro",
    priority: 74,
  },
  {
    providerID: "fireworks",
    envNames: ["FIREWORKS_API_KEY"],
    signupUrl: "https://fireworks.ai/pricing",
    recommendedModelID: "fireworks-deepseek-v4-pro",
    priority: 73,
  },
  {
    providerID: "dashscope",
    envNames: ["DASHSCOPE_API_KEY"],
    signupUrl: "https://www.alibabacloud.com/help/en/model-studio/qwen-coder",
    recommendedModelID: "alibaba-qwen3-coder-plus",
    priority: 72,
  },
  {
    providerID: "sambanova",
    envNames: ["SAMBANOVA_API_KEY"],
    signupUrl: "https://cloud.sambanova.ai",
    recommendedModelID: "sambanova-gpt-oss-120b",
    priority: 71,
  },
  {
    providerID: "huggingface",
    envNames: ["HF_TOKEN"],
    signupUrl: "https://huggingface.co/settings/tokens",
    recommendedModelID: "huggingface-qwen3-coder-next",
    priority: 70,
  },
  {
    providerID: "zai",
    envNames: ["ZAI_API_KEY"],
    signupUrl: "https://z.ai/manage-apikey/apikey-list",
    recommendedModelID: "zai-glm-47-flash",
    priority: 69,
  },
  {
    providerID: "inception",
    envNames: ["INCEPTION_API_KEY"],
    signupUrl: "https://platform.inceptionlabs.ai",
    recommendedModelID: "inception-mercury-2",
    priority: 68,
  },
  {
    providerID: "ai-gateway",
    envNames: ["AI_GATEWAY_API_KEY"],
    signupUrl: "https://vercel.com/ai-gateway",
    recommendedModelID: "vercel-claude-sonnet-46",
    priority: 67,
  },
  {
    providerID: "kilo",
    envNames: ["KILO_API_KEY"],
    signupUrl: "https://app.kilo.ai",
    recommendedModelID: "kilo-ling-26-1t-free",
    priority: 66,
  },
  {
    providerID: "cloudflare",
    envNames: ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"],
    signupUrl: "https://dash.cloudflare.com/profile/api-tokens",
    recommendedModelID: "cloudflare-gpt-oss-120b",
    priority: 65,
    companionEnvNames: ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"],
  },
  {
    providerID: "jekko",
    envNames: ["JEKKO_API_KEY"],
    signupUrl: "https://jekko.ai/zen",
    recommendedModelID: "big-pickle",
    priority: 95,
  },
  {
    providerID: "jnoccio",
    envNames: ["JNOCCIO_DEVELOPER_KEY"],
    recommendedModelID: "jnoccio-fusion",
    priority: 96,
  },
]
