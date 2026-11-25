#!/usr/bin/env bash
# Fetches the pinned real-world TOML corpus into tests/conformance/corpus/toml-real/<repo> (gitignored): the
# .toml files (Cargo manifests, rustfmt and clippy configs, deny.toml) of rust-analyzer, Bevy, Tokio, clap, serde, uv and
# Cargo, all MIT or Apache-2.0.
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs toml-real <<'EOF'
rust-lang/rust-analyzer 9c74e78e01d7cd31663f4ecf71e65b1cc4b9ccb8 *.toml
bevyengine/bevy 1c4460b102a6a4380319455051c0e4c6051708a6 *.toml
tokio-rs/tokio 854013a8cb7d1397e9d037315592dc1d2d55187d *.toml
clap-rs/clap 487474f013d3e0cbffc911934575f7ebcc72a6d4 *.toml
serde-rs/serde 5be4690236e16d8a16d80f76703e59091ac3be2e *.toml
astral-sh/uv d99121a2e246065c28661dc58da9233b0abbd479 *.toml
rust-lang/cargo 08aa485f6fb55e8265856645334269e1262f111f *.toml
EOF
