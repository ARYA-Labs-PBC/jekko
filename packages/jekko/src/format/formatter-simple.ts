import { commandFormatter, type Info } from "./formatter-shared"
import { which } from "../util/which"

export const gofmt: Info = commandFormatter("gofmt", [".go"], () => which("gofmt"), ["-w", "$FILE"])

export const mix: Info = commandFormatter(
  "mix",
  [".ex", ".exs", ".eex", ".heex", ".leex", ".neex", ".sface"],
  () => which("mix"),
  ["format", "$FILE"],
)

export const zig: Info = commandFormatter("zig", [".zig", ".zon"], () => which("zig"), ["fmt", "$FILE"])

export const ktlint: Info = commandFormatter("ktlint", [".kt", ".kts"], () => which("ktlint"), ["-F", "$FILE"])

export const rubocop: Info = commandFormatter(
  "rubocop",
  [".rb", ".rake", ".gemspec", ".ru"],
  () => which("rubocop"),
  ["--autocorrect", "$FILE"],
)

export const standardrb: Info = commandFormatter(
  "standardrb",
  [".rb", ".rake", ".gemspec", ".ru"],
  () => which("standardrb"),
  ["--fix", "$FILE"],
)

export const htmlbeautifier: Info = commandFormatter(
  "htmlbeautifier",
  [".erb", ".html.erb"],
  () => which("htmlbeautifier"),
  ["$FILE"],
)

export const dart: Info = commandFormatter("dart", [".dart"], () => which("dart"), ["format", "$FILE"])

export const terraform: Info = commandFormatter(
  "terraform",
  [".tf", ".tfvars"],
  () => which("terraform"),
  ["fmt", "$FILE"],
)

export const latexindent: Info = commandFormatter(
  "latexindent",
  [".tex"],
  () => which("latexindent"),
  ["-w", "-s", "$FILE"],
)

export const gleam: Info = commandFormatter("gleam", [".gleam"], () => which("gleam"), ["format", "$FILE"])

export const shfmt: Info = commandFormatter("shfmt", [".sh", ".bash"], () => which("shfmt"), ["-w", "$FILE"])

export const nixfmt: Info = commandFormatter("nixfmt", [".nix"], () => which("nixfmt"), ["$FILE"])

export const rustfmt: Info = commandFormatter("rustfmt", [".rs"], () => which("rustfmt"), ["$FILE"])

export const ormolu: Info = commandFormatter("ormolu", [".hs"], () => which("ormolu"), ["-i", "$FILE"])

export const cljfmt: Info = commandFormatter(
  "cljfmt",
  [".clj", ".cljs", ".cljc", ".edn"],
  () => which("cljfmt"),
  ["fix", "--quiet", "$FILE"],
)

export const dfmt: Info = commandFormatter("dfmt", [".d"], () => which("dfmt"), ["-i", "$FILE"])
