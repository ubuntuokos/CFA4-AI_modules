#!/usr/bin/env bash
# CycloneDX SBOMs for the two shipped CFA4 binaries (cfa4, cfa4-cli).
#
# Source-level inventory from Cargo.lock: every crate, version, licence expression
# and checksum that the release build links for x86_64 Linux. It does not compile
# CFA4 and does not cover system libraries, Qt/PySide6 or the package formats
# themselves (see docs/audit/F1-01-SBOM-2026-10-25.md).
#
# Requires: cargo-cyclonedx (reviewed version below), jq.
# Output:   $DIST/sbom/{cfa4,cfa4-cli}.cdx.json and $DIST/sbom/SHA256SUMS
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/../../packaging/env.sh"

REQUIRED_CYCLONEDX="0.5.9"
TARGET="x86_64-unknown-linux-gnu"
OUT="$DIST/sbom"

have="$(cargo cyclonedx --version 2>/dev/null | awk '{print $2}')" || true
if [[ "$have" != "$REQUIRED_CYCLONEDX" ]]; then
  echo "error: cargo-cyclonedx $REQUIRED_CYCLONEDX is required (found: ${have:-none})." >&2
  echo "       cargo install --locked cargo-cyclonedx --version $REQUIRED_CYCLONEDX" >&2
  exit 2
fi
command -v jq >/dev/null || { echo "error: jq is required" >&2; exit 2; }

# cargo-cyclonedx writes one <crate>.cdx.json next to every workspace manifest.
# Work on a throwaway copy of the manifests so the checkout stays clean.
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
(cd "$ROOT" && git ls-files -z -- 'Cargo.toml' 'Cargo.lock' '*/Cargo.toml' '*.rs' 'rust-toolchain.toml' \
  | tar --null -T - -cf - | tar -xf - -C "$work")

(cd "$work" && cargo cyclonedx --format json --spec-version 1.5 --target "$TARGET" -qq)

mkdir -p "$OUT"
for crate in cfa4 cfa4-cli; do
  src="$work/apps/$crate/$crate.cdx.json"
  [[ -s "$src" ]] || { echo "error: no SBOM produced for $crate" >&2; exit 1; }
  # Every component must carry a licence: an empty licence is a release-gate gap.
  missing="$(jq '[.components[] | select((.licenses // []) | length == 0) | .name] | length' "$src")"
  count="$(jq '.components | length' "$src")"
  if [[ "$count" -lt 1 || "$missing" -ne 0 ]]; then
    echo "error: $crate SBOM has $count components, $missing without a licence" >&2
    exit 1
  fi
  cp "$src" "$OUT/$crate.cdx.json"
  echo "$crate: $count components (CycloneDX 1.5, $TARGET)"
done
(cd "$OUT" && sha256sum cfa4.cdx.json cfa4-cli.cdx.json > SHA256SUMS)
echo "CFA4 SBOMs written to $OUT"
