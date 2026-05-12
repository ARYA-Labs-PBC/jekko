// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
export type YamlScope =
  | "property"
  | "string"
  | "boolean"
  | "number"
  | "comment"
  | "sentinel"
  | "literal"
  | "punctuation"
  | "sequence"
  | "block"
  | "operator"

export type YamlToken = {
  readonly start: number
  readonly end: number
  readonly scope: YamlScope
}

const SENTINEL_OPEN_RE = /^\s*<<<ZYAL\b[^]*?>>>$/
const SENTINEL_CLOSE_RE = /^\s*<<<END_ZYAL\b[^]*?>>>$/
const SENTINEL_ARM_RE = /^\s*ZYAL_ARM\b/
const BARE_BOOLEAN_RE = /^(true|false|yes|no|null|~)\b/i
const BARE_NUMBER_RE = /^-?\d+(?:\.\d+)?(?:[A-Za-z%]+)?\b/
const WORD_RE = /^[A-Za-z_$][A-Za-z0-9_$./*-]*/
const UPPER_WORD_RE = /^[A-Z][A-Z0-9_/-]{1,}\b/
const PUNCTUATION = new Set(["{", "}", "[", "]", ":", ","])
const OPERATOR = new Set(["|", ">"])

/**
 * Tokenize a YAML-ish text buffer for highlighting in the TUI prompt. Tolerant
 * of partial input — the user may be in the middle of pasting a ZYAL block,
 * so the function never throws and never assumes the document parses cleanly.
 *
 * Offsets are byte-equivalent character offsets into the input string, so they
 * can feed directly into opentui extmark `start`/`end` fields.
 */
export function tokenizeYaml(text: string): YamlToken[] {
  const tokens: YamlToken[] = []
  const lines = text.split("\n")
  let offset = 0
  let blockParentIndent: number | undefined

  for (const line of lines) {
    const lineEnd = offset + line.length
    const indent = indentation(line)
    const nonBlank = line.trim().length > 0

    if (blockParentIndent !== undefined && nonBlank && indent <= blockParentIndent) {
      blockParentIndent = undefined
    }

    if (SENTINEL_OPEN_RE.test(line) || SENTINEL_CLOSE_RE.test(line) || SENTINEL_ARM_RE.test(line)) {
      tokens.push({ start: offset, end: lineEnd, scope: "sentinel" })
      offset = lineEnd + 1
      continue
    }

    if (blockParentIndent !== undefined) {
      tokenizeBlockLine(line, offset, tokens)
      offset = lineEnd + 1
      continue
    }

    const hashIdx = findCommentStart(line)
    const codeEnd = hashIdx >= 0 ? hashIdx : line.length
    const codePart = line.slice(0, codeEnd)

    tokenizeYamlCode(codePart, offset, tokens)
    if (lineStartsBlockScalar(codePart)) blockParentIndent = indent

    if (hashIdx >= 0) {
      tokens.push({ start: offset + hashIdx, end: lineEnd, scope: "comment" })
    }

    offset = lineEnd + 1
  }

  return tokens
}

function tokenizeYamlCode(code: string, baseOffset: number, tokens: YamlToken[]) {
  let index = 0
  while (index < code.length) {
    const char = code[index]

    if (!char || isWhitespace(char)) {
      index++
      continue
    }

    if (char === "-" && isSequenceMarker(code, index)) {
      push(tokens, baseOffset + index, baseOffset + index + 1, "sequence")
      index++
      continue
    }

    if (char === '"' || char === "'" || char === "`") {
      const end = scanQuoted(code, index, char)
      push(tokens, baseOffset + index, baseOffset + end, "string")
      index = end
      continue
    }

    if (PUNCTUATION.has(char)) {
      push(tokens, baseOffset + index, baseOffset + index + 1, "punctuation")
      index++
      continue
    }

    if (OPERATOR.has(char)) {
      push(tokens, baseOffset + index, baseOffset + index + 1, "operator")
      index++
      continue
    }

    const valueToken = matchScalarToken(code.slice(index), baseOffset + index)
    if (valueToken) {
      tokens.push(valueToken)
      index += valueToken.end - valueToken.start
      continue
    }

    const word = code.slice(index).match(WORD_RE)
    if (word) {
      const start = index
      const end = index + word[0].length
      const afterSpaces = skipSpaces(code, end)
      if (code[afterSpaces] === ":") {
        push(tokens, baseOffset + start, baseOffset + end, "property")
        push(tokens, baseOffset + afterSpaces, baseOffset + afterSpaces + 1, "punctuation")
        index = afterSpaces + 1
      } else {
        push(tokens, baseOffset + start, baseOffset + end, "literal")
        index = end
      }
      continue
    }

    push(tokens, baseOffset + index, baseOffset + index + 1, "literal")
    index++
  }
}

function tokenizeBlockLine(line: string, baseOffset: number, tokens: YamlToken[]) {
  let index = 0
  while (index < line.length) {
    const char = line[index]
    if (!char || isWhitespace(char)) {
      index++
      continue
    }

    if (char === "-" && isSequenceMarker(line, index)) {
      push(tokens, baseOffset + index, baseOffset + index + 1, "sequence")
      index++
      continue
    }

    if (char === '"' || char === "'" || char === "`") {
      const end = scanQuoted(line, index, char)
      push(tokens, baseOffset + index, baseOffset + end, "string")
      index = end
      continue
    }

    if (PUNCTUATION.has(char)) {
      push(tokens, baseOffset + index, baseOffset + index + 1, "punctuation")
      index++
      continue
    }

    const valueToken = matchScalarToken(line.slice(index), baseOffset + index)
    if (valueToken) {
      tokens.push(valueToken)
      index += valueToken.end - valueToken.start
      continue
    }

    const upper = line.slice(index).match(UPPER_WORD_RE)
    if (upper) {
      push(tokens, baseOffset + index, baseOffset + index + upper[0].length, "operator")
      index += upper[0].length
      continue
    }

    const next = nextBlockBoundary(line, index + 1)
    push(tokens, baseOffset + index, baseOffset + next, "block")
    index = next
  }
}

function matchScalarToken(text: string, baseOffset: number): YamlToken | null {
  const boolMatch = text.match(BARE_BOOLEAN_RE)
  if (boolMatch) {
    return { start: baseOffset, end: baseOffset + boolMatch[0].length, scope: "boolean" }
  }

  const numberMatch = text.match(BARE_NUMBER_RE)
  if (numberMatch) {
    return { start: baseOffset, end: baseOffset + numberMatch[0].length, scope: "number" }
  }

  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return null
}

function lineStartsBlockScalar(codePart: string) {
  return /:\s*[|>][+-]?\s*$/.test(codePart)
}

function indentation(line: string) {
  let count = 0
  while (count < line.length && (line[count] === " " || line[count] === "\t")) count++
  return count
}

function isWhitespace(char: string) {
  return char === " " || char === "\t"
}

function skipSpaces(text: string, index: number) {
  while (index < text.length && isWhitespace(text[index] ?? "")) index++
  return index
}

function isSequenceMarker(text: string, index: number) {
  const before = index === 0 ? "" : text[index - 1]
  const after = text[index + 1]
  return (!before || isWhitespace(before)) && (after === " " || after === "\t")
}

function scanQuoted(text: string, start: number, quote: string) {
  let index = start + 1
  while (index < text.length) {
    const char = text[index]
    if (char === "\\" && quote !== "'") {
      index += 2
      continue
    }
    if (char === quote) {
      if (quote === "'" && text[index + 1] === "'") {
        index += 2
        continue
      }
      return index + 1
    }
    index++
  }
  return text.length
}

function nextBlockBoundary(line: string, start: number) {
  let index = start
  while (index < line.length) {
    const char = line[index]
    if (
      char === '"' ||
      char === "'" ||
      char === "`" ||
      PUNCTUATION.has(char) ||
      BARE_BOOLEAN_RE.test(line.slice(index)) ||
      BARE_NUMBER_RE.test(line.slice(index)) ||
      UPPER_WORD_RE.test(line.slice(index))
    ) {
      return index
    }
    index++
  }
  return line.length
}

function push(tokens: YamlToken[], start: number, end: number, scope: YamlScope) {
  if (end <= start) return
  tokens.push({ start, end, scope })
}

/**
 * Locate the start column of a YAML line comment, ignoring `#` characters
 * that fall inside a quoted string. Returns -1 when no comment is present.
 */
function findCommentStart(line: string): number {
  let inSingle = false
  let inDouble = false
  for (let i = 0; i < line.length; i++) {
    const ch = line[i]
    if (ch === "\\" && (inSingle || inDouble)) {
      i += 1
      continue
    }
    if (ch === '"' && !inSingle) inDouble = !inDouble
    else if (ch === "'" && !inDouble) inSingle = !inSingle
    else if (ch === "#" && !inSingle && !inDouble) {
      if (i === 0 || line[i - 1] === " " || line[i - 1] === "\t") return i
    }
  }
  return -1
}
