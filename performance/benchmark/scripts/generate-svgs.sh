#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"
repo_dir="$(cd "${crate_dir}/../.." && pwd)"

cargo run --manifest-path "${crate_dir}/Cargo.toml" --release --bin generate-svgs -- "${repo_dir}/docs/performance/assets"
