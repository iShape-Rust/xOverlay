#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"
profile="${BENCH_PROFILE:-full}"
budget_ms="${BENCH_BUDGET_MS:-700}"
samples="${BENCH_SAMPLES:-9}"
solver_timeout_seconds="${BENCH_SOLVER_TIMEOUT_SECONDS:-30}"

"${script_dir}/build.sh"
"${crate_dir}/target/release/overlay-benchmark" \
  --profile "${profile}" \
  --budget-ms "${budget_ms}" \
  --samples "${samples}" \
  --solver-timeout-seconds "${solver_timeout_seconds}" \
  --machine "${BENCH_MACHINE:-}" \
  --boost-bin "${crate_dir}/target/release/boost-runner" \
  --output "${crate_dir}/results/latest.json" \
  "$@"
"${script_dir}/render.sh" "${crate_dir}/results/latest.json"

echo "Benchmark data and the report page are ready."
