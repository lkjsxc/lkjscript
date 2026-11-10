#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
if (($# == 0)); then
  set -- selfhost_source_artifact_whole_maintained_raw_source_fixed_point --nocapture
fi
exec bash tools/check.sh slow "$@"
