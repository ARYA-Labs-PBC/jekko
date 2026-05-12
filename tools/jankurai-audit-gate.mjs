#!/usr/bin/env node

import { readFileSync } from 'node:fs';

const inputPath = process.argv[2] ?? 'target/jankurai/repo-score.json';

function fail(message) {
  console.error(`jankurai audit gate failed: ${message}`);
  process.exit(1);
}

let score;
try {
  score = JSON.parse(readFileSync(inputPath, 'utf8'));
} catch (error) {
  const detail = error instanceof Error ? error.message : String(error);
  fail(`unable to read ${inputPath}: ${detail}`);
}

const capsApplied = Array.isArray(score.caps_applied) ? score.caps_applied : null;
const findingCount = Number(score.finding_count);
const hardFindings = Number(score.hard_findings);
const softFindings = Number(score.soft_findings);

const problems = [];

if (!capsApplied) {
  problems.push('caps_applied is missing or not an array');
} else if (capsApplied.length > 0) {
  problems.push(`caps_applied must be empty, found: ${capsApplied.join(', ')}`);
}

if (!Number.isFinite(findingCount)) {
  problems.push('finding_count is missing or not numeric');
} else if (findingCount !== 0) {
  problems.push(`finding_count must be 0, found: ${findingCount}`);
}

if (!Number.isFinite(hardFindings)) {
  problems.push('hard_findings is missing or not numeric');
} else if (hardFindings !== 0) {
  problems.push(`hard_findings must be 0, found: ${hardFindings}`);
}

if (!Number.isFinite(softFindings)) {
  problems.push('soft_findings is missing or not numeric');
} else if (softFindings !== 0) {
  problems.push(`soft_findings must be 0, found: ${softFindings}`);
}

if (problems.length > 0) {
  fail(problems.join('; '));
}

console.log(`jankurai audit gate passed: ${inputPath} has 0 caps and 0 findings`);
