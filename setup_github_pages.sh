#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

workflow_file=".github/workflows/pages.yml"
script_file="setup_github_pages.sh"

if [[ ! -f "$workflow_file" ]]; then
  echo "Error: $workflow_file does not exist." >&2
  exit 1
fi

branch="$(git branch --show-current)"
if [[ -z "$branch" ]]; then
  echo "Error: Git is in detached HEAD state." >&2
  exit 1
fi

git add "$workflow_file" "$script_file"

if git diff --cached --quiet -- "$workflow_file" "$script_file"; then
  echo "GitHub Pages setup is already committed."
else
  git commit --only "$workflow_file" "$script_file" \
    -m "Deploy benchmark report to GitHub Pages"
fi

git push --set-upstream origin "$branch"

echo "GitHub Pages workflow pushed from branch: $branch"
