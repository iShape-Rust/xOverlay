#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"

cargo run --manifest-path "${crate_dir}/Cargo.toml" --release --bin generate-svgs -- "${crate_dir}/site/assets"
