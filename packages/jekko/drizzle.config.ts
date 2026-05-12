import { defineConfig } from "drizzle-kit"

export default defineConfig({
  dialect: "sqlite",
  schema: "./src/**/*.sql.ts",
  out: "../../db/migrations",
  dbCredentials: {
    url: "/home/thdxr/.local/share/jekko/jekko.db",
  },
})
