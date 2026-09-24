#!/usr/bin/env bash
set -euo pipefail

# Generate CycloneDX SBOMs from the committed lockfiles.
# Tooling reads package-lock.json and src-tauri/Cargo.lock; the SBOMs are never
# written by hand. Output defaults to ./sbom.

out_dir="${1:-sbom}"
cyclonedx_npm_version="${CYCLONEDX_NPM_VERSION:-6.0.1}"

mkdir -p "$out_dir"

echo "npm: generating CycloneDX SBOM from package-lock.json"
npx --yes "@cyclonedx/cyclonedx-npm@${cyclonedx_npm_version}" \
    --package-lock-only \
    --spec-version 1.5 \
    --output-format JSON \
    --output-file "${out_dir}/npm.cdx.json" \
    --output-reproducible \
    ./package.json

echo "cargo: generating CycloneDX SBOM from src-tauri/Cargo.lock"
cargo cyclonedx \
    --manifest-path src-tauri/Cargo.toml \
    --format json \
    --spec-version 1.5 \
    --override-filename cargo.cdx
mv src-tauri/cargo.cdx.json "${out_dir}/cargo.cdx.json"

echo "SBOMs written to ${out_dir}:"
ls -l "$out_dir"
