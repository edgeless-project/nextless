#!/bin/bash

# Inspired by the nix tool wrapProgram
if command -v rustup > /dev/null; then
    # Nextless requires a nightly compiler for native builds (build_std).
    # We use a specific nightly here to prevent frequent updates.
    rustup install nightly-2026-01-01 1> /dev/null
    rustup +nightly-2026-01-01 target add wasm32-unknown-unknown 1> /dev/null
    # rustup run does not work for Cargo used as a crate.
    export RUSTC="$(rustup +nightly-2026-01-01 which rustc)"
    exec nextless_cli_raw "$@"
else
    exec nextless_cli_raw "$@"
fi
