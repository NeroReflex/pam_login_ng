#!/bin/sh
new_version=${1}
version=$(grep -oP 'version\s*=\s*"\K[0-9]+\.[0-9]+\.[0-9]+' Cargo.toml)
find . -name "*.toml"  -type f -exec sed -i "s/\"$version\"/\"$new_version\"/g" {} +
#sed -i "s/\"$version\"/\"$new_version\"/g" Cargo.lock
cargo build
git add .
git commit -m "Bump version to $new_version"
git tag $new_version