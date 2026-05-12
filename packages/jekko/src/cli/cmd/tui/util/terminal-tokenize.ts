export type TerminalScope =
  | "success"
  | "error"
  | "warning"
  | "time"
  | "command"
  | "string"
  | "punctuation"
  | "number"
  | "keyword"
  | "prompt"

export type TerminalToken = {
  readonly start: number
  readonly end: number
  readonly scope: TerminalScope
}

const REGEX_RULES: { scope: TerminalScope; re: RegExp }[] = [
  // Shell prompts / commands: line starting with $ or # and the first command word
  { scope: "command", re: /^(?:[$#]|>)\s+[a-zA-Z0-9_-]+/gm },
  { scope: "prompt", re: /^(?:[$#]|>)/gm },
  
  // Strings
  { scope: "string", re: /"[^"]*"/g },
  { scope: "string", re: /'[^']*'/g },
  
  // Time formats like [ 0.012s], 12ms, 1.5s
  { scope: "time", re: /\[?\s*\d+(?:\.\d+)?(?:ms|s)\s*\]?/g },
  
  // Status badges
  { scope: "success", re: /\b(?:PASS|OK|SUCCESS)\b|✓/g },
  { scope: "error", re: /\b(?:FAIL|ERROR|FAILED|ERR)\b|✗/g },
  { scope: "warning", re: /\b(?:WARN|WARNING)\b/g },
  
  // Numbers (avoiding matching inside other words)
  { scope: "number", re: /\b\d+(?:\.\d+)?\b/g },
  
  // Common CLI keywords (just for some extra flavor)
  { scope: "keyword", re: /\b(?:true|false|null|undefined|yes|no)\b/g },
  
  // Punctuation
  { scope: "punctuation", re: /[\[\]{}()]/g },
]

const MAX_TOKENIZE_LENGTH = 10_000

export function tokenizeTerminal(text: string): TerminalToken[] {
  // This is a presentation-only tokenizer. Bounding the scan keeps terminal
  // rendering from doing unbounded regex work on arbitrary log output.
  const source = text.slice(0, MAX_TOKENIZE_LENGTH)
  const tokens: TerminalToken[] = []

  for (const rule of REGEX_RULES) {
    for (const match of source.matchAll(rule.re)) {
      tokens.push({
        start: match.index,
        end: match.index + match[0].length,
        scope: rule.scope,
      })
    }
  }

  // Sort tokens by start position. If they start at the same position, longer tokens win.
  tokens.sort((a, b) => {
    if (a.start !== b.start) return a.start - b.start
    return b.end - a.end
  })

  // Remove overlapping tokens
  const resolved: TerminalToken[] = []
  let cursor = 0
  for (const token of tokens) {
    if (token.start >= cursor) {
      resolved.push(token)
      cursor = token.end
    }
  }

  return resolved
}
