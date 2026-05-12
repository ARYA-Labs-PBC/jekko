import type { InstanceContext } from "../project/instance"

export interface Context extends Pick<InstanceContext, "directory" | "worktree"> {}

export interface Info {
  name: string
  environment?: Record<string, string>
  extensions: string[]
  enabled(context: Context): Promise<string[] | false>
}

export function commandFormatter(
  name: string,
  extensions: string[],
  resolve: () => string | false | undefined,
  args: string[],
  environment?: Record<string, string>,
): Info {
  return {
    name,
    extensions,
    environment,
    async enabled() {
      const match = resolve()
      if (!match) return false
      return [match, ...args]
    },
  }
}
