# Jekko v2 API example

This example shows the intended runtime flow for the v2 surface without
placing executable JavaScript in the spec tree.

```ts
import { Jekko } from "@jekko-ai/core"
import { ReadTool } from "@jekko-ai/core/tools"

const jekko = Jekko.make({})

jekko.tool.add(ReadTool)

jekko.tool.add({
  name: "bash",
  schema: {
    type: "object",
    properties: {
      command: {
        type: "string",
        description: "The command to run.",
      },
    },
    required: ["command"],
  },
  execute(_input, _ctx) {},
})

jekko.auth.add({
  provider: "openai",
  type: "api",
  value: process.env.OPENAI_API_KEY,
})

jekko.agent.add({
  name: "build",
  permissions: [],
  model: {
    id: "gpt-5-5",
    provider: "openai",
    variant: "xhigh",
  },
})

const sessionID = await jekko.session.create({
  agent: "build",
})

jekko.subscribe((event) => {
  console.log(event)
})

await jekko.session.prompt({
  sessionID,
  text: "hey what is up",
})

await jekko.session.prompt({
  sessionID,
  text: "what is up with this",
  files: [
    {
      mime: "image/png",
      uri: "data:image/png;base64,xxxx",
    },
  ],
})

await jekko.session.wait()

console.log(await jekko.session.messages(sessionID))
```
