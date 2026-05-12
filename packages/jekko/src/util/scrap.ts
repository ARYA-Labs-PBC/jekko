export function hasMeaningfulText(value: string | null | undefined): value is string {
  return typeof value === "string" && value.trim().length > 0
}

export function normalizeLabel(value: string): string {
  return value.trim().replace(/\s+/g, " ")
}

export function formatLabel(prefix: string, value: string): string {
  const normalizedPrefix = normalizeLabel(prefix)
  const normalizedValue = normalizeLabel(value)
  return normalizedPrefix ? `${normalizedPrefix}: ${normalizedValue}` : normalizedValue
}
