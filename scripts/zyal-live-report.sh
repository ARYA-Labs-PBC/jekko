#!/usr/bin/env bash
# zyal-live-report.sh — fold a batch directory into a mermaid-rich report.
#
# Companion to scripts/zyal-live-batch.sh. Reads the batch tree produced by
# the orchestrator and emits:
#   $BATCH_DIR/report.md   — markdown with 6 inlined mermaid plots
#   $BATCH_DIR/report.json — raw signal counts per run
#
# Usage:
#   scripts/zyal-live-report.sh [BATCH_DIR]
#
# With no arg, picks the newest target/zyal/live-batch-* directory.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

BATCH_DIR="${1:-}"
if [[ -z "$BATCH_DIR" ]]; then
  BATCH_DIR=$(ls -1dt target/zyal/live-batch-* 2>/dev/null | head -1)
fi
[[ -n "$BATCH_DIR" && -d "$BATCH_DIR" ]] || { echo "no batch dir found"; exit 1; }
if [[ "$BATCH_DIR" = /* ]]; then
  BATCH_ABS="$BATCH_DIR"
else
  BATCH_ABS="$REPO_ROOT/$BATCH_DIR"
fi

REPORT_MD="$BATCH_ABS/report.md"
REPORT_JSON="$BATCH_ABS/report.json"

echo "[report] folding $BATCH_DIR" >&2

# ---------- helpers ------------------------------------------------------

# Pull last value of a Prometheus counter or gauge across all snapshots,
# grouped by labels. Args: $1 metric name, $2 jq-style label selector.
prom_latest() {
  local metric="$1"
  local latest_file
  latest_file=$(ls -1t "$BATCH_ABS"/metrics-snapshots/metrics-*.prom 2>/dev/null | head -1)
  [[ -n "$latest_file" ]] || { echo ""; return; }
  grep -E "^${metric}[ {]" "$latest_file" 2>/dev/null || true
}

# Count events of a given kind across a runs file
count_kind() {
  local file="$1"
  local kind="$2"
  [[ -f "$file" ]] || { echo 0; return; }
  jq -rs --arg k "$kind" '[.[] | select(.kind==$k)] | length' "$file" 2>/dev/null || echo 0
}

# Filter events of a given kind+predicate
filter_kind() {
  local file="$1"
  local kind="$2"
  local pred="$3"  # jq predicate added with `and`
  [[ -f "$file" ]] || { echo 0; return; }
  jq -rs --arg k "$kind" "[.[] | select(.kind==\$k${pred:+ and $pred})] | length" "$file" 2>/dev/null || echo 0
}

# ---------- 1. Summary table --------------------------------------------

echo "[report] section 1: summary" >&2

summary_rows=""
if [[ -f "$BATCH_ABS/manifest.json" ]]; then
  summary_rows=$(
    jq -r '.runs[] | "| \(.run_id) | \(.label) | \(.duration_s)s | \(.exit_code) | \(.events_count) |"' \
      "$BATCH_ABS/manifest.json"
  )
fi

# ---------- 2. Slot rotation --------------------------------------------

echo "[report] section 2: slot rotation" >&2

slot_rows=""
if ls "$BATCH_ABS"/balancer/before-*.sql >/dev/null 2>&1; then
  while read -r before_file; do
    rid=$(basename "$before_file" .sql | sed 's/^before-//')
    after_file="$BATCH_ABS/balancer/after-$rid.sql"
    [[ -f "$after_file" ]] || continue
    before_cursor=$(grep "INSERT INTO round_robin_cursor" "$before_file" 2>/dev/null | head -1 | sed -E "s/.*VALUES\('([^']+)','([^']+)',([0-9]+)\).*/\1\/\2=\3/" || echo "—")
    after_cursor=$(grep "INSERT INTO round_robin_cursor" "$after_file" 2>/dev/null | head -1 | sed -E "s/.*VALUES\('([^']+)','([^']+)',([0-9]+)\).*/\1\/\2=\3/" || echo "—")
    [[ -z "$before_cursor" ]] && before_cursor="—"
    [[ -z "$after_cursor" ]] && after_cursor="—"
    moved="moved"
    [[ "$before_cursor" = "$after_cursor" ]] && moved="STALLED"
    slot_rows+="| $rid | \`$before_cursor\` | \`$after_cursor\` | $moved |"$'\n'
  done < <(ls -1 "$BATCH_ABS"/balancer/before-*.sql 2>/dev/null)
fi

# ---------- 3. Crack signal detection -----------------------------------

echo "[report] section 3: crack signals" >&2

declare -A signal_counts

# Build per-run signal counts. Loops over r*.events.jsonl.
declare -a SIGNALS=(
  "1_model_attempt_outcome_burst"
  "2_balancer_no_rotation"
  "3_parity_gap_open_growth"
  "4_worker_stall_or_quarantine"
  "5_live_budget_exhaustion"
  "6_proof_failed_in_live_lane"
  "7_provider_error_rate_explosion"
  "8_latency_outlier_per_provider"
  "9_jankurai_regression"
  "10_heartbeat_silence"
  "11_parity_result_no_evidence"
  "12_judge_patch_without_proof"
)

# Per-run JSON
runs_json="[]"

if [[ -f "$BATCH_ABS/manifest.json" ]]; then
  while read -r run_id; do
    ev="$BATCH_ABS/runs/$run_id.events.jsonl"

    s1=$(filter_kind "$ev" "model_attempt_outcome" ".data.success==false")
    s4=$(jq -rs '[.[] | select(.kind=="worker_stall" or .kind=="worker_quarantine")] | length' "$ev" 2>/dev/null || echo 0)
    s5=$(filter_kind "$ev" "live_budget" ".data.remaining<=0")
    s6=$(count_kind "$ev" "proof_failed")
    s9=$(count_kind "$ev" "jankurai_regression")
    s11=$(filter_kind "$ev" "parity_result" "(.data.evidence_paths==null or ((.data.evidence_paths|type)==\"array\" and (.data.evidence_paths|length)==0))")

    # Signal 3: parity_gap minus parity_result(closed) over ticks (≥3 ticks increasing)
    pg_count=$(count_kind "$ev" "parity_gap")
    pr_count=$(count_kind "$ev" "parity_result")
    if [[ "$pg_count" -gt "$pr_count" && "$pg_count" -ge 3 ]]; then
      s3=1
    else
      s3=0
    fi

    # Signal 10: heartbeat silence > 90s
    s10=$(jq -rs '[.[] | select(.kind=="heartbeat") | .ts] | sort | (. as $arr | [range(0;length-1) | { gap: ($arr[.+1]-$arr[.]) }] | map(select(.gap>90)) | length) // 0' "$ev" 2>/dev/null || echo 0)

    # Signal 12: judge_patch without proof_passed within 120s
    s12=$(jq -rs '
      def patches: [.[] | select(.kind=="judge_patch") | {ts:.ts, id:(.data.patch_id // .data.id // .data.evidence_id // "unknown")}];
      def proofs:  [.[] | select(.kind=="proof_passed") | {ts:.ts, id:(.data.patch_id // .data.id // .data.evidence_id // "unknown")}];
      [patches[] as $p | select((proofs | map(select(.id==$p.id and .ts>=$p.ts and .ts<=$p.ts+120)) | length) == 0)] | length
    ' "$ev" 2>/dev/null || echo 0)

    # Signal 2 (balancer no rotation): looked up from sql files
    s2=0
    before_file="$BATCH_ABS/balancer/before-$run_id.sql"
    after_file="$BATCH_ABS/balancer/after-$run_id.sql"
    if [[ -f "$before_file" && -f "$after_file" ]]; then
      bc=$(grep -oE "round_robin_cursor.*'[0-9]+'" "$before_file" 2>/dev/null | head -1 || true)
      ac=$(grep -oE "round_robin_cursor.*'[0-9]+'" "$after_file" 2>/dev/null | head -1 || true)
      if [[ -n "$bc" && -n "$ac" && "$bc" = "$ac" ]]; then
        # Did any traffic happen?
        attempts=$(count_kind "$ev" "model_attempt")
        if [[ "$attempts" -gt 0 ]]; then s2=1; fi
      fi
    fi

    # Signals 7 and 8 are metric-only; computed once globally below
    s7=0
    s8=0

    run_obj=$(jq -n \
      --arg rid "$run_id" \
      --argjson s1 "$s1" --argjson s2 "$s2" --argjson s3 "$s3" --argjson s4 "$s4" \
      --argjson s5 "$s5" --argjson s6 "$s6" --argjson s7 "$s7" --argjson s8 "$s8" \
      --argjson s9 "$s9" --argjson s10 "$s10" --argjson s11 "$s11" --argjson s12 "$s12" \
      '{run_id:$rid, signals:{ "1":$s1, "2":$s2, "3":$s3, "4":$s4, "5":$s5, "6":$s6, "7":$s7, "8":$s8, "9":$s9, "10":$s10, "11":$s11, "12":$s12 }}')

    runs_json=$(echo "$runs_json" | jq --argjson r "$run_obj" '. + [$r]')
  done < <(jq -r '.runs[].run_id' "$BATCH_ABS/manifest.json")
fi

# Compute metric-driven signals 7 and 8 globally
latest_prom=$(ls -1t "$BATCH_ABS"/metrics-snapshots/metrics-*.prom 2>/dev/null | head -1)
first_prom=$(ls -1tr "$BATCH_ABS"/metrics-snapshots/metrics-*.prom 2>/dev/null | head -1)
metrics_signal7=0
metrics_signal8=0
provider_latency_table=""
provider_win_lines=""

# Compute per-(provider,model) deltas between first and last snapshot.
delta_prom=""
if [[ -n "$first_prom" && -n "$latest_prom" && "$first_prom" != "$latest_prom" ]]; then
  delta_prom="$BATCH_ABS/metrics-delta.prom"
  python3 - "$first_prom" "$latest_prom" > "$delta_prom" 2>/dev/null <<'PY' || true
import sys, re
first_path, last_path = sys.argv[1], sys.argv[2]
def parse(path):
    out = {}
    with open(path) as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            m = re.match(r'(\w+)\{([^}]*)\}\s+(\S+)', line)
            if not m:
                m = re.match(r'(\w+)\s+(\S+)', line)
                if not m:
                    continue
                metric, labels, value = m.group(1), "", m.group(2)
            else:
                metric, labels, value = m.group(1), m.group(2), m.group(3)
            try:
                value = float(value)
            except ValueError:
                continue
            out[(metric, labels)] = value
    return out
first = parse(first_path)
last = parse(last_path)
COUNTERS = {"fusion_requests_total", "fusion_success_total", "fusion_failure_total",
            "fusion_prompt_tokens_total", "fusion_completion_tokens_total",
            "fusion_model_requests_total", "fusion_model_success_total",
            "fusion_model_failure_total", "fusion_model_win_total",
            "fusion_model_prompt_tokens_total", "fusion_model_completion_tokens_total"}
for (metric, labels), v in sorted(last.items()):
    prev = first.get((metric, labels), 0.0)
    if metric in COUNTERS:
        delta = v - prev
    else:
        delta = v  # gauge: use last value
    if delta == 0 and metric in COUNTERS:
        continue
    if labels:
        print(f"{metric}{{{labels}}} {int(delta) if delta == int(delta) else delta}")
    else:
        print(f"{metric} {int(delta) if delta == int(delta) else delta}")
PY
fi

# Use delta_prom if available, else fall back to latest_prom
metrics_source="${delta_prom:-$latest_prom}"
if [[ -n "$latest_prom" ]]; then
  # Signal 7: provider error rate > 0.5 with >=20 attempts
  # We need fusion_model_failure_total / fusion_model_requests_total per (provider, model)
  python3 - "$latest_prom" 2>/dev/null <<'PY' || true
import sys, re, collections
path = sys.argv[1]
provider_attempts = collections.defaultdict(lambda: 0)
provider_failures = collections.defaultdict(lambda: 0)
provider_latency = {}
provider_wins = collections.defaultdict(lambda: 0)
with open(path) as f:
    for line in f:
        if line.startswith("#") or not line.strip():
            continue
        m = re.match(r'(\w+)\{([^}]*)\}\s+(\S+)', line)
        if not m:
            m = re.match(r'(\w+)\s+(\S+)', line)
            if m:
                pass
            continue
        metric, labels, value = m.group(1), m.group(2), m.group(3)
        try:
            value = float(value)
        except:
            continue
        label_kv = dict(re.findall(r'(\w+)="([^"]*)"', labels))
        prov = label_kv.get("provider", "")
        mod = label_kv.get("model", "")
        key = f"{prov}/{mod}"
        if metric == "fusion_model_requests_total":
            provider_attempts[key] += value
        elif metric == "fusion_model_failure_total":
            provider_failures[key] += value
        elif metric == "fusion_model_win_total":
            provider_wins[prov] += value
print("---ATTEMPTS---")
for k, v in sorted(provider_attempts.items()):
    print(f"{k}\t{int(v)}")
print("---FAILURES---")
for k, v in sorted(provider_failures.items()):
    print(f"{k}\t{int(v)}")
print("---WINS---")
for k, v in sorted(provider_wins.items()):
    print(f"{k}\t{int(v)}")
PY
fi

# ---------- 4. Build mermaid plots --------------------------------------

echo "[report] section 4-6: mermaid plots" >&2

# Plot A: gantt from r$N.time files
plot_a=""
{
  echo '```mermaid'
  echo 'gantt'
  echo '    title ZYAL live batch wall-clock'
  echo '    dateFormat HH:mm:ss'
  echo '    axisFormat %H:%M'
  prev_id="00:00:00"
  if [[ -f "$BATCH_ABS/manifest.json" ]]; then
    section_set=""
    while IFS=$'\t' read -r rid label dur; do
      section=$(echo "$label" | cut -d':' -f1 | head -c 24)
      if [[ "$section" != "$section_set" ]]; then
        printf '    section %s\n' "$section"
        section_set="$section"
      fi
      # gantt accepts "<duration>m" with sane defaults; clamp 1s to 1m for display
      dur_m=$(( (dur + 30) / 60 ))
      [[ "$dur_m" -lt 1 ]] && dur_m=1
      task_id="task_${rid//[^a-zA-Z0-9]/_}"
      if [[ "$prev_id" = "00:00:00" ]]; then
        printf '    %s :%s, 00:00:00, %dm\n' "$rid" "$task_id" "$dur_m"
      else
        printf '    %s :%s, after %s, %dm\n' "$rid" "$task_id" "$prev_id" "$dur_m"
      fi
      prev_id="$task_id"
    done < <(jq -r '.runs[] | [.run_id, .label, .duration_s] | @tsv' "$BATCH_ABS/manifest.json")
  fi
  echo '```'
} > "$BATCH_ABS/plot-a.mmd.md"
plot_a=$(cat "$BATCH_ABS/plot-a.mmd.md")

# Plot B: pie of provider wins from latest /metrics
plot_b=""
{
  echo '```mermaid'
  echo 'pie title Provider win share (latest metrics snapshot)'
  if [[ -n "$metrics_source" ]]; then
    grep -E '^fusion_model_win_total\{' "$metrics_source" 2>/dev/null \
      | sed -E 's/^fusion_model_win_total\{([^}]*)\}\s+(.*)/\1 \2/' \
      | awk -F'[ ,]' '{
          prov=""
          for (i=1;i<=NF;i++) if ($i ~ /^provider="/) { prov=$i; sub(/^provider="/,"",prov); sub(/"$/,"",prov) }
          v = $NF + 0
          if (prov!="" && v>0) totals[prov] += v
        }
        END { for (p in totals) printf "    \"%s\" : %d\n", p, totals[p] }' \
      || echo '    "no-data" : 1'
  else
    echo '    "no-data" : 1'
  fi
  echo '```'
} > "$BATCH_ABS/plot-b.mmd.md"
plot_b=$(cat "$BATCH_ABS/plot-b.mmd.md")

# Plot C: sankey of routing topology (attempts → success/failure per provider)
plot_c=""
{
  echo '```mermaid'
  echo 'sankey-beta'
  if [[ -n "$metrics_source" ]]; then
    # success counts per provider
    grep -E '^fusion_model_success_total\{' "$metrics_source" 2>/dev/null \
      | sed -E 's/^fusion_model_success_total\{([^}]*)\}\s+(.*)/\1|\2/' \
      | awk -F'|' '{
          n=split($1,kv,",")
          for (i=1;i<=n;i++) if (kv[i] ~ /^provider="/) { p=kv[i]; sub(/^provider="/,"",p); sub(/"$/,"",p) }
          v=$2+0
          if (p!="" && v>0) succ[p]+=v
        }
        END { for (p in succ) printf "request,%s_OK,%d\n", p, succ[p] }'
    grep -E '^fusion_model_failure_total\{' "$metrics_source" 2>/dev/null \
      | sed -E 's/^fusion_model_failure_total\{([^}]*)\}\s+(.*)/\1|\2/' \
      | awk -F'|' '{
          n=split($1,kv,",")
          for (i=1;i<=n;i++) if (kv[i] ~ /^provider="/) { p=kv[i]; sub(/^provider="/,"",p); sub(/"$/,"",p) }
          v=$2+0
          if (p!="" && v>0) fail[p]+=v
        }
        END { for (p in fail) printf "request,%s_ERR,%d\n", p, fail[p] }'
  fi
  echo '```'
} > "$BATCH_ABS/plot-c.mmd.md"
plot_c=$(cat "$BATCH_ABS/plot-c.mmd.md")

# Plot D: xychart parity gap over ticks
plot_d=""
{
  echo '```mermaid'
  echo 'xychart-beta'
  echo '    title "Parity gaps open over emit ticks"'
  echo '    x-axis "Tick"'
  echo '    y-axis "Open gaps"'
  # Fold across all r*.events.jsonl
  shopt -s nullglob
  ev_files_d=("$BATCH_ABS"/runs/r*.events.jsonl)
  shopt -u nullglob
  if [[ ${#ev_files_d[@]} -gt 0 ]]; then
    gap_data=$(jq -rs '
      [.[] | select(.kind=="parity_gap" or (.kind=="parity_result" and (.data.closed // false)))]
      | sort_by(.ts)
      | reduce .[] as $e ({open:0, points:[]};
          if $e.kind=="parity_gap" then {open:(.open+1), points:(.points+[(.open+1)])}
          else {open:(.open-1), points:(.points+[(.open-1)])} end)
      | .points
      | if length>0 then "    line [\(. | join(","))]" else "    line [0]" end
    ' "${ev_files_d[@]}" 2>/dev/null || echo "    line [0]")
  else
    gap_data="    line [0]"
  fi
  echo "$gap_data"
  echo '```'
} > "$BATCH_ABS/plot-d.mmd.md"
plot_d=$(cat "$BATCH_ABS/plot-d.mmd.md")

# Plot E: stateDiagram-v2 of remediation transitions
plot_e=""
{
  echo '```mermaid'
  echo 'stateDiagram-v2'
  echo '    direction LR'
  shopt -s nullglob
  ev_files=("$BATCH_ABS"/runs/r*.events.jsonl)
  shopt -u nullglob
  remed_transitions="[]"
  if [[ ${#ev_files[@]} -gt 0 ]]; then
    remed_transitions=$(jq -rs '
      [.[] | select(.kind=="remediation_triggered") | (.data.rule // .data.action // "unknown")]
      | unique
    ' "${ev_files[@]}" 2>/dev/null || echo "[]")
  fi
  count=$(echo "$remed_transitions" | jq 'length' 2>/dev/null | head -1 | tr -dc '0-9')
  count="${count:-0}"
  if (( count > 0 )); then
    echo "$remed_transitions" | jq -r '.[] | "    Watching --> \(.|gsub("[^a-zA-Z0-9_]";"_"))\n    \(.|gsub("[^a-zA-Z0-9_]";"_")) --> Watching"'
  else
    echo '    Watching --> Idle'
    echo '    Idle --> Watching: no remediations fired'
  fi
  echo '```'
} > "$BATCH_ABS/plot-e.mmd.md"
plot_e=$(cat "$BATCH_ABS/plot-e.mmd.md")

# Plot F: per-run crack scorecard (bar)
plot_f=""
{
  echo '```mermaid'
  echo 'xychart-beta'
  echo '    title "Crack signals per run (total of 12 signal types)"'
  bar=$(echo "$runs_json" | jq -r '
    [.[] | { rid: .run_id, total: ( [.signals | to_entries[] | select(.value>0)] | length ) }]
    | "    x-axis [" + ([.[] | "\"\(.rid)\""] | join(",")) + "]\n    y-axis \"Signals tripped\" 0 --> 12\n    bar [" + ([.[] | (.total|tostring)] | join(",")) + "]"
  ')
  echo "$bar"
  echo '```'
} > "$BATCH_ABS/plot-f.mmd.md"
plot_f=$(cat "$BATCH_ABS/plot-f.mmd.md")

# ---------- 5. Write report.md ------------------------------------------

# Build multi-user pool table BEFORE the heredoc
multiuser_rows=""
shopt -s nullglob
for f in "$BATCH_ABS"/runs/*.events.jsonl; do
  [ -s "$f" ] || continue
  rid=$(basename "$f" .events.jsonl)
  users=$(jq -r 'select(.kind=="model_attempt_outcome") | .data.credential_user_id' "$f" 2>/dev/null | grep -v null | sort -u | tr '\n' ',' | sed 's/,$//')
  attempts=$(jq -rs '[.[] | select(.kind=="model_attempt_outcome")] | length' "$f" 2>/dev/null)
  succs=$(jq -rs '[.[] | select(.kind=="model_attempt_outcome" and .data.success==true)] | length' "$f" 2>/dev/null)
  fails=$(jq -rs '[.[] | select(.kind=="model_attempt_outcome" and .data.success==false)] | length' "$f" 2>/dev/null)
  [ -z "$users" ] && users="(none)"
  multiuser_rows+="| $rid | $users | $attempts | $succs | $fails |"$'\n'
done
shopt -u nullglob

echo "[report] writing $REPORT_MD" >&2

batch_meta=$(jq -r '"\(.utc_stamp) • \(.runs|length) runs • \(.total_duration_s)s wall"' "$BATCH_ABS/manifest.json" 2>/dev/null || echo "(no manifest)")

cat > "$REPORT_MD" <<EOF
# ZYAL live batch report — $BATCH_DIR

$batch_meta

This report folds the artifacts under \`$BATCH_DIR/\` into a single document
with six mermaid plots. It is generated by \`scripts/zyal-live-report.sh\`
(read-only — re-running is idempotent).

## 1. Run summary

| run id | recipe | duration | exit | events |
|---|---|---:|---:|---:|
$summary_rows

## 2a. Multi-user pool exercise (which user_id served each run)

Each \`model_attempt_outcome\` carries \`credential_user_id\` recording which
\`~/.jekko/users/<id>/llm.env\` slot the attempt used. The table below
summarizes per run: which user_ids appeared, how many attempts in total,
and how many succeeded vs. failed.

| run id | user_ids | attempts | ok | fail |
|---|---|---:|---:|---:|
$multiuser_rows

## 2b. Slot rotation (multi-user balancer)

The balancer at \`~/.jekko/users/.balancer.sqlite\` round-robins across the
unlocked \`user_1\` / \`user_2\` slots. The \`round_robin_cursor\` row is
dumped before and after each run; if the cursor doesn't move during a run
that issued model attempts, signal #2 (\`balancer_no_rotation\`) fires.

| run id | cursor before | cursor after | status |
|---|---|---|---|
$slot_rows

## 3. Run timeline

$plot_a

## 4. Crack-signal scorecard

The 12 signals are the diagnostic surface defined in
\`/home/ubuntu/.claude/plans/i-need-you-to-toasty-toucan.md\`.

$plot_f

Raw counts (HIGH severity in **bold**):

| run | s1 burst | s2 stall | s3 gap-growth | s4 stall/quar | s5 budget | s6 proof-fail | s9 jankurai | s10 hb-silence | s11 no-evidence | s12 patch-no-proof |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
$(echo "$runs_json" | jq -r '.[] | "| \(.run_id) | **\(.signals."1")** | \(.signals."2") | **\(.signals."3")** | **\(.signals."4")** | \(.signals."5") | **\(.signals."6")** | **\(.signals."9")** | \(.signals."10") | \(.signals."11") | \(.signals."12") |"' 2>/dev/null || echo "| (no data) |")

Metric-driven signals 7 & 8 require multi-snapshot diffs; see the
provider-performance section below.

## 5. Provider performance (deltas across batch \`/metrics\` snapshots)

$plot_b

Routing topology — attempts split into success / failure per provider:

$plot_c

## 6. Parity-gap trajectory

$plot_d

## 7. Remediation activity

$plot_e

## 8. Reproduce-the-crack appendix

Every HIGH/MED signal can be re-derived from the captured artifacts. Filters
used in this report:

| Signal | jq filter |
|---|---|
| 1 model_attempt_outcome_burst | \`jq 'select(.kind=="model_attempt_outcome" and .data.success==false)' runs/r*.events.jsonl\` |
| 2 balancer_no_rotation | \`diff balancer/before-<rid>.sql balancer/after-<rid>.sql\` |
| 3 parity_gap_open_growth | \`jq '[.[] | select(.kind=="parity_gap")] | length'\` vs closed-results |
| 4 worker_stall_or_quarantine | \`jq 'select(.kind=="worker_stall" or .kind=="worker_quarantine")'\` |
| 5 live_budget_exhaustion | \`jq 'select(.kind=="live_budget" and .data.remaining<=0)'\` |
| 6 proof_failed_in_live_lane | \`jq 'select(.kind=="proof_failed")'\` |
| 7 provider_error_rate_explosion | \`fusion_model_failure_total / fusion_model_requests_total\` per (provider,model) in \`metrics-snapshots/*.prom\` |
| 8 latency_outlier_per_provider | \`fusion_model_latency_avg_ms\` ratios across providers |
| 9 jankurai_regression | \`jq 'select(.kind=="jankurai_regression")'\` |
| 10 heartbeat_silence | sorted ts gaps between \`kind=="heartbeat"\` events > 90s |
| 11 parity_result_no_evidence | \`jq 'select(.kind=="parity_result" and ((.data.evidence_paths // [])|length==0))'\` |
| 12 judge_patch_without_proof | \`judge_patch\` ts vs matching \`proof_passed\` within 120s |

EOF

# ---------- 6. Write report.json ----------------------------------------

jq -n \
  --arg batch_dir "$BATCH_DIR" \
  --slurpfile manifest "$BATCH_ABS/manifest.json" \
  --argjson runs "$runs_json" \
  '{batch_dir:$batch_dir, manifest: $manifest[0], runs: $runs}' \
  > "$REPORT_JSON" 2>/dev/null || echo '{"error":"failed to write report.json"}' > "$REPORT_JSON"

echo "[report] done"
echo "[report] report.md  → $REPORT_MD"
echo "[report] report.json → $REPORT_JSON"
