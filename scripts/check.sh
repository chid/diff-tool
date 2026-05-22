#!/usr/bin/env bash
set -euo pipefail

npm run check
CARGO_HOME="${CARGO_HOME:-/tmp/cargo-home}" cargo test --workspace
