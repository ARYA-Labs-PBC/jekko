# JNoccio Fusion Key Inventory

Use this file as the checklist for setting up upstream access.

| Provider | Sign-up / API key URL | Env key name(s) |
| --- | --- | --- |
| OpenRouter | https://openrouter.ai/keys | `OPENROUTER_API_KEY` |
| Vercel AI Gateway | https://vercel.com/ai-gateway | `AI_GATEWAY_API_KEY` |
| Kilo Gateway | https://app.kilo.ai | `KILO_API_KEY` |
| Z.ai | https://z.ai/manage-apikey/apikey-list | `ZAI_API_KEY` |
| Inception Labs | https://platform.inceptionlabs.ai | `INCEPTION_API_KEY` |
| SambaNova Cloud | https://cloud.sambanova.ai | `SAMBANOVA_API_KEY` |
| Hugging Face Inference Providers | https://huggingface.co/settings/tokens | `HF_TOKEN` |
| Cerebras | https://cloud.cerebras.ai | `CEREBRAS_API_KEY` |
| Groq | https://console.groq.com/keys | `GROQ_API_KEY` |
| Google Gemini | https://aistudio.google.com/apikey | `GEMINI_API_KEY` |
| Cloudflare Workers AI | https://dash.cloudflare.com/profile/api-tokens | `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID` |
| Mistral | https://console.mistral.ai/api-keys | `MISTRAL_API_KEY` |
| GitHub Models | https://github.com/marketplace/models | `GITHUB_TOKEN` |
| NVIDIA NIM | https://build.nvidia.com | `NVIDIA_API_KEY` |
| Fireworks | https://fireworks.ai/pricing | `FIREWORKS_API_KEY` |
| Alibaba / DashScope | https://www.alibabacloud.com/help/en/model-studio/qwen-coder | `DASHSCOPE_API_KEY` |

## Env File

Put the values in `/Users/bentaylor/Code/opencode/jnoccio-fusion/.env.jnoccio`.

From the Jekko TUI, select the locked `Jnoccio Fusion` model and enter the
path to your local git-crypt key file. A successful unlock creates
`.env.jnoccio` from `.env.jnoccio.example` when the file does not already
exist. Existing `.env.jnoccio` files are never overwritten.

That file should contain the following keys:

```text
OPENROUTER_API_KEY=
AI_GATEWAY_API_KEY=
CEREBRAS_API_KEY=
KILO_API_KEY=
ZAI_API_KEY=
INCEPTION_API_KEY=
SAMBANOVA_API_KEY=
HF_TOKEN=
GROQ_API_KEY=
GEMINI_API_KEY=
CLOUDFLARE_API_TOKEN=
CLOUDFLARE_ACCOUNT_ID=
MISTRAL_API_KEY=
GITHUB_TOKEN=
NVIDIA_API_KEY=
FIREWORKS_API_KEY=
DASHSCOPE_API_KEY=
```

## Local Unlock Test

Authorized keyholders can run the local-only unlock proof with:

```bash
JNOCCIO_GIT_CRYPT_KEY_PATH=/path/to/jnoccio-fusion.key rtk bun test test/local/jnoccio-unlock.local.test.ts
```

The test skips in CI, when `JNOCCIO_GIT_CRYPT_KEY_PATH` is unset, or when
`git-crypt` is unavailable.
