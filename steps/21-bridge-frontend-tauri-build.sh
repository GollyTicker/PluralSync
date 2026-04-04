#!/bin/bash

set -euo pipefail

export TAURI_APP_PATH="bridge-src-tauri"

cp bridge-src-tauri/{template.,}tauri.conf.json

cargo tauri build --debug --no-bundle
