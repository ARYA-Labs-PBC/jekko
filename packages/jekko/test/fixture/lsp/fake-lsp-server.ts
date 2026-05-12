// Simple JSON-RPC 2.0 LSP-like fake server over stdio

type PullRegistration = {
  id?: string
  method?: string
  registerOptions?: any
  [key: string]: any
}

type PullConfig = {
  delayMs: number
  registerOn?: string
  registrations: PullRegistration[]
  documentDiagnostics: any[]
  documentDiagnosticsByIdentifier: Record<string, any[]>
  documentDelayMsByIdentifier: Record<string, number>
  workspaceDiagnostics: any[]
  workspaceDiagnosticsByIdentifier: Record<string, any[]>
  workspaceDelayMsByIdentifier: Record<string, number>
}

let nextId = 1
let readBuffer: any = Buffer.alloc(0)
let lastChange: any = null
let initializeParams: any = null
let diagnosticRequestCount = 0
let registeredCapability = false
const pendingClientRequests = new Map()
let pullConfig: PullConfig = {
  delayMs: 0,
  registerOn: undefined,
  registrations: [],
  documentDiagnostics: [],
  documentDiagnosticsByIdentifier: {},
  documentDelayMsByIdentifier: {},
  workspaceDiagnostics: [],
  workspaceDiagnosticsByIdentifier: {},
  workspaceDelayMsByIdentifier: {},
}

function encode(message: any) {
  const json = JSON.stringify(message)
  const header = `Content-Length: ${Buffer.byteLength(json, "utf8")}\r\n\r\n`
  return Buffer.concat([Buffer.from(header, "utf8"), Buffer.from(json, "utf8")])
}

function decodeFrames(buffer: any): { messages: string[]; rest: any } {
  const results: string[] = []
  let idx: number
  while ((idx = buffer.indexOf("\r\n\r\n")) !== -1) {
    const header: string = buffer.slice(0, idx).toString("utf8")
    const contentLengthLine = header
      .split("\r\n")
      .find((line: string) => line.toLowerCase().startsWith("content-length:"))
    const length = contentLengthLine
      ? Number.parseInt(contentLengthLine.slice(contentLengthLine.indexOf(":") + 1).trim(), 10)
      : 0
    const bodyStart = idx + 4
    const bodyEnd = bodyStart + length
    if (buffer.length < bodyEnd) break
    results.push(buffer.slice(bodyStart, bodyEnd).toString("utf8"))
    buffer = buffer.slice(bodyEnd)
  }
  return { messages: results, rest: buffer }
}

function send(message: any) {
  process.stdout.write(encode(message))
}

function sendRequest(method: string, params: any) {
  const id = nextId++
  send({ jsonrpc: "2.0", id, method, params })
  return id
}

function sendResponse(id: number, result: any) {
  send({ jsonrpc: "2.0", id, result })
}

function sendNotification(method: string, params: any) {
  send({ jsonrpc: "2.0", method, params })
}

function maybeRegister(method?: string) {
  if (pullConfig.registerOn !== method || registeredCapability) return
  registeredCapability = true
  sendRequest("client/registerCapability", {
    registrations: pullConfig.registrations.map((registration, index) => ({
      id: registration.id ?? `pull-${index}`,
      method: registration.method ?? "textDocument/diagnostic",
      registerOptions: registration.registerOptions ?? registration,
    })),
  })
}

function delayed(id: number, result: any, delayMs = pullConfig.delayMs) {
  if (!delayMs) {
    sendResponse(id, result)
    return
  }
  setTimeout(() => sendResponse(id, result), delayMs)
}

function diagnosticsForIdentifier(identifier: string) {
  return pullConfig.documentDiagnosticsByIdentifier[identifier] ?? pullConfig.documentDiagnostics
}

function workspaceDiagnosticsForIdentifier(identifier: string) {
  return pullConfig.workspaceDiagnosticsByIdentifier[identifier] ?? pullConfig.workspaceDiagnostics
}

function documentDelayForIdentifier(identifier: string) {
  return pullConfig.documentDelayMsByIdentifier[identifier] ?? pullConfig.delayMs
}

function workspaceDelayForIdentifier(identifier: string) {
  return pullConfig.workspaceDelayMsByIdentifier[identifier] ?? pullConfig.delayMs
}

function handle(raw: string) {
  let data: any
  try {
    data = JSON.parse(raw)
  } catch {
    return
  }

  if (typeof data.method === "undefined" && typeof data.id !== "undefined") {
    const pending = pendingClientRequests.get(data.id)
    if (!pending) return
    pendingClientRequests.delete(data.id)
    sendResponse(pending, data.result ?? null)
    return
  }

  if (data.method === "initialize") {
    initializeParams = data.params
    sendResponse(data.id, {
      capabilities: {
        textDocumentSync: {
          change: 2,
        },
      },
    })
    return
  }

  if (data.method === "test/get-initialize-params") {
    sendResponse(data.id, initializeParams)
    return
  }

  if (data.method === "test/request-configuration") {
    const id = sendRequest("workspace/configuration", data.params)
    pendingClientRequests.set(id, data.id)
    return
  }

  if (data.method === "initialized" || data.method === "workspace/didChangeConfiguration") {
    return
  }

  if (data.method === "textDocument/didOpen") {
    maybeRegister("didOpen")
    return
  }

  if (data.method === "textDocument/didChange") {
    lastChange = data.params
    maybeRegister("didChange")
    return
  }

  if (data.method === "test/trigger") {
    const method = data.params && data.params.method
    if (method === "client/registerCapability") {
      sendRequest(method, {
        registrations: [
          {
            id: "test-diagnostic-registration",
            method: "textDocument/diagnostic",
            registerOptions: { identifier: "syntax" },
          },
        ],
      })
      return
    }
    if (method === "client/unregisterCapability") {
      sendRequest(method, {
        unregisterations: [{ id: "test-diagnostic-registration", method: "textDocument/diagnostic" }],
      })
      return
    }
    if (method) sendRequest(method, {})
    return
  }

  if (data.method === "test/configure-pull-diagnostics") {
    pullConfig = {
      delayMs: data.params?.delayMs ?? 0,
      registerOn: data.params?.registerOn,
      registrations: data.params?.registrations ?? [],
      documentDiagnostics: data.params?.documentDiagnostics ?? [],
      documentDiagnosticsByIdentifier: data.params?.documentDiagnosticsByIdentifier ?? {},
      documentDelayMsByIdentifier: data.params?.documentDelayMsByIdentifier ?? {},
      workspaceDiagnostics: data.params?.workspaceDiagnostics ?? [],
      workspaceDiagnosticsByIdentifier: data.params?.workspaceDiagnosticsByIdentifier ?? {},
      workspaceDelayMsByIdentifier: data.params?.workspaceDelayMsByIdentifier ?? {},
    }
    registeredCapability = false
    sendResponse(data.id, null)
    return
  }

  if (data.method === "test/register-configured-pull-diagnostics") {
    maybeRegister(undefined)
    sendResponse(data.id, null)
    return
  }

  if (data.method === "test/publish-diagnostics") {
    sendNotification("textDocument/publishDiagnostics", data.params)
    return
  }

  if (data.method === "test/get-last-change") {
    sendResponse(data.id, lastChange)
    return
  }

  if (data.method === "test/get-diagnostic-request-count") {
    sendResponse(data.id, diagnosticRequestCount)
    return
  }

  if (data.method === "textDocument/diagnostic") {
    diagnosticRequestCount += 1
    delayed(
      data.id,
      {
        kind: "full",
        items: diagnosticsForIdentifier(data.params?.identifier ?? ""),
      },
      documentDelayForIdentifier(data.params?.identifier ?? ""),
    )
    return
  }

  if (data.method === "workspace/diagnostic") {
    diagnosticRequestCount += 1
    delayed(
      data.id,
      {
        items: workspaceDiagnosticsForIdentifier(data.params?.identifier ?? ""),
      },
      workspaceDelayForIdentifier(data.params?.identifier ?? ""),
    )
    return
  }

  if (typeof data.id !== "undefined") {
    sendResponse(data.id, null)
  }
}

process.stdin.on("data", (chunk) => {
  readBuffer = Buffer.concat([readBuffer, chunk])
  const { messages, rest } = decodeFrames(readBuffer)
  readBuffer = rest
  for (const message of messages) handle(message)
})
