#!/usr/bin/env bash
set -e

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
cargo install trunk wasm-bindgen-cli cargo-leptos leptosfmt just
