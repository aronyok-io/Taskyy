#!/usr/bin/env bash
set -euo pipefail

project=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
destination=${1:-"$project/dist"}
mkdir -p -- "$destination"
destination=$(cd -- "$destination" && pwd)
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT

desktop="$project/packaging/io.github.taskyy.Taskyy.desktop"
desktop-file-validate "$desktop"
cargo build --release --locked --manifest-path "$project/Cargo.toml" --target-dir "${CARGO_TARGET_DIR:-"$project/target"}"
target=${CARGO_TARGET_DIR:-"$project/target"}
mkdir -p "$stage"/{SOURCES,RPMS,taskyy-payload}
install -m0755 "$target/release/taskyy" "$stage/taskyy-payload/taskyy"
install -m0644 "$desktop" "$project/LICENSE" "$stage/taskyy-payload/"
magick "$project/assets/taskyy.png" -resize 256x256 "$stage/taskyy-payload/icon.png"
tar -C "$stage" -czf "$stage/SOURCES/taskyy-payload.tar.gz" taskyy-payload
rpmbuild -bb --define "_topdir $stage" --define "_tmppath $stage" "$project/packaging/taskyy.spec"
find "$stage/RPMS" -type f -name '*.rpm' -exec cp -v -t "$destination" -- {} +
