#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

just jekko-build-fast

host_target="jekko-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/x64/;s/aarch64/arm64/')"
export JEKKO_BIN="$root/packages/jekko/dist/$host_target/bin/jekko"

UPDATE_README_DEMO=1 cargo test --manifest-path crates/tuiwright-jekko-unlock/Cargo.toml --test readme_demo -- --ignored --nocapture

artifact_dir="$root/target/tuiwright-jekko/readme-demo"
fixture_dir="$root/target/jk-target-bk/tuiwright-jekko"
source_dir="$artifact_dir"
if [[ ! -f "$source_dir/01-boot.png" || ! -f "$source_dir/03-running.png" ]]; then
  source_dir="$fixture_dir"
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
font_path="$(fc-match -f '%{file}\n' Menlo)"

crop_frame() {
  local src="$1"
  local dst="$2"
  local max_height="$3"
  local width height crop_height
  width="$(identify -format '%w' "$src")"
  height="$(identify -format '%h' "$src")"
  if (( height < max_height )); then
    crop_height="$height"
  else
    crop_height="$max_height"
  fi
  magick "$src" -crop "${width}x${crop_height}+0+0" +repage "$dst"
}

boot_src="$source_dir/01-boot.png"
middle_src="$source_dir/02-run-card.png"
final_src="$source_dir/03-running.png"

if [[ ! -f "$middle_src" ]]; then
  middle_src="$source_dir/zyal-03-panel.png"
fi
if [[ ! -f "$final_src" ]]; then
  final_src="$source_dir/zyal-04-full-docs-fast.png"
fi

boot_crop="$tmpdir/boot.png"
middle_crop="$tmpdir/middle.png"
final_crop="$tmpdir/final.png"
middle_overlay="$tmpdir/middle-overlay.png"
snippet_file="$tmpdir/zyal-snippet.txt"

crop_frame "$boot_src" "$boot_crop" 900
crop_frame "$middle_src" "$middle_crop" 1020
crop_frame "$final_src" "$final_crop" 1020

sed -n '2,11p' "$root/docs/ZYAL/examples/13-advanced-research-loop.zyal" > "$snippet_file"
magick \
  -background '#0b1220' \
  -fill '#f8fafc' \
  -font "$font_path" \
  -pointsize 20 \
  -interline-spacing 6 \
  -size 920x280 \
  caption:@"$snippet_file" \
  "$middle_overlay"

magick \
  "$boot_crop" \
  -delay 220 \
  "$middle_crop" "$middle_overlay" -geometry +72+72 -composite \
  -delay 320 \
  "$final_crop" \
  -delay 480 \
  -loop 0 \
  docs/assets/jekko-tui-zyal-demo.gif

magick docs/assets/jekko-tui-zyal-demo.gif -layers Optimize docs/assets/jekko-tui-zyal-demo.optimized.gif
mv docs/assets/jekko-tui-zyal-demo.optimized.gif docs/assets/jekko-tui-zyal-demo.gif
