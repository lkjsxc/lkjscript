#!/bin/bash
set -euo pipefail
witness_root=/home/coder/workspace/lkjscript-native-public-20260922
cd "$witness_root"
# Invoke only after root confirms official publication. No authentication or user curl config.
env -i PATH=/usr/bin:/bin HOME="$witness_root/home" curl -q --fail --location --silent --show-error --proto '=https' --proto-redir '=https' --connect-timeout 15 --max-time 180 --dump-header acquisition/install.headers --output acquisition/install.sh https://github.com/lkjsxc/lkjscript/releases/download/v0.1.40/install.sh
env -i PATH=/usr/bin:/bin HOME="$witness_root/home" TMPDIR="$witness_root/tmp" sh acquisition/install.sh --prefix "$witness_root/installation"
