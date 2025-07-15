#!/bin/bash

# Inspired by the nix tool wrapProgram
if command -v rustup > /dev/null; then
    rustup install nightly-2025-07-13 1> /dev/null
    rustup +nightly-2025-07-13 target add wasm32-unknown-unknown 1> /dev/null
    exec rustup run nightly-2025-07-13 nextless_cli_raw "$@"
else
    exec nextless_cli_raw "$@"
fi
