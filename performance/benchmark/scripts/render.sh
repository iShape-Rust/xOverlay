#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"
repo_dir="$(cd "${crate_dir}/../.." && pwd)"
results_file="${1:-${crate_dir}/results/latest.json}"

"${script_dir}/generate-svgs.sh"
cargo run --manifest-path "${crate_dir}/Cargo.toml" --release --bin render-report -- \
  "${results_file}" "${repo_dir}/docs/performance/index.html"
