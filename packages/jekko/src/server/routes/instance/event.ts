import z from "zod"
import { Hono } from "hono"
import { describeRoute, resolver } from "hono-openapi"
import { BusEvent } from "@/bus/bus-event"
import { Bus } from "@/bus"
import { streamQueuedSSE } from "../sse-stream"

export const EventRoutes = () =>
  new Hono().get(
    "/event",
    describeRoute({
      summary: "Subscribe to events",
      description: "Get events",
      operationId: "event.subscribe",
      responses: {
        200: {
          description: "Event stream",
          content: {
            "text/event-stream": {
              schema: resolver(
                z.union(BusEvent.payloads()).meta({
                  ref: "Event",
                }),
              ),
            },
          },
        },
      },
    }),
    async (c) => {
      c.header("Cache-Control", "no-cache, no-transform")
      c.header("X-Accel-Buffering", "no")
      c.header("X-Content-Type-Options", "nosniff")
      return streamQueuedSSE(c, {
        connectedMessage: "event connected",
        disconnectedMessage: "event disconnected",
        initialEvent: () =>
          JSON.stringify({
            id: Bus.createID(),
            type: "server.connected",
            properties: {},
          }),
        heartbeatEvent: () =>
          JSON.stringify({
            id: Bus.createID(),
            type: "server.heartbeat",
            properties: {},
          }),
        subscribe(q, stop) {
          return Bus.subscribeAll((event) => {
            q.push(JSON.stringify(event))
            if (event.type === Bus.InstanceDisposed.type) {
              stop()
            }
          })
        },
      })
    },
  )
