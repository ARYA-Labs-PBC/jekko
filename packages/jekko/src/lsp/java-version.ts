export function parseJavaMajorVersion(stderr: string): number | undefined {
  const firstLineEnd = stderr.indexOf("\n")
  const firstLine = (firstLineEnd === -1 ? stderr : stderr.slice(0, firstLineEnd)).trim()
  const versionStart = firstLine.indexOf('"')
  if (versionStart === -1) return
  const versionEnd = firstLine.indexOf('"', versionStart + 1)
  if (versionEnd === -1) return
  const version = firstLine.slice(versionStart + 1, versionEnd)
  const versionParts = version.split(".")
  const majorPart = versionParts[0] === "1" ? versionParts[1] : versionParts[0]
  if (!majorPart) return
  let major = 0
  for (const char of majorPart) {
    if (char < "0" || char > "9") return
    major = major * 10 + (char.charCodeAt(0) - 48)
  }
  return major
}
