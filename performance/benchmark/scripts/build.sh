#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"
boost_prefix="${BOOST_ROOT:-}"

if [[ -z "${boost_prefix}" ]] && command -v brew >/dev/null 2>&1; then
  boost_prefix="$(brew --prefix boost)"
fi

if [[ -z "${boost_prefix}" ]]; then
  echo "Boost was not found. Set BOOST_ROOT to its installation prefix." >&2
  exit 1
fi

cargo build --manifest-path "${crate_dir}/Cargo.toml" --release --bins
clang++ \
  -I"${boost_prefix}/include" \
  -O3 -DNDEBUG -std=c++20 -march=native \
  "${crate_dir}/boost/runner.cpp" \
  -o "${crate_dir}/target/release/boost-runner"

echo "Built Rust benchmark tools and Boost Polygon 90 runner."
