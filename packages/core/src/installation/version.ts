declare global {
  const JEKKO_VERSION: string
  const JEKKO_CHANNEL: string
}

export const InstallationVersion = typeof JEKKO_VERSION === "string" ? JEKKO_VERSION : "local"
export const InstallationChannel = typeof JEKKO_CHANNEL === "string" ? JEKKO_CHANNEL : "local"
export const InstallationLocal = InstallationChannel === "local"
