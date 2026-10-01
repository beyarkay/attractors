#!/usr/bin/env zsh
# Micro-benchmarks (divan) plus an end-to-end headless render (hyperfine).
# Usage: scripts/bench.sh [paper size, default A5]
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
paper=${1:-A5}

cargo bench --bench attractor
cargo build --release

params=$(mktemp)
echo 'a=-1.4,b=1.6,c=1.0,d=0.7' > $params
hyperfine --warmup 1 --runs 5 \
    --prepare "rm -f cache/clifford/a=-1.4,b=1.6,c=1,d=0.7,size=*.png" \
    "./target/release/attractors $params $paper"
rm -f cache/clifford/a=-1.4,b=1.6,c=1,d=0.7,size=*.png
rm -f $params
