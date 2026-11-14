#!/usr/bin/env bash
# ACS-7 stabilitási próbák a referenciagép korlátai között (docs/ai/acs-adaptiv-eroforras.md 9. pont).
# Mindkét profilban (minimum: 16 GB, ajánlott: 32 GB RAM, swap nélkül, 8 mag, a referencia-GPU)
# lefuttatja:
#   1. a feladatsor véletlenszerű próbáját hosszú üzemben (CFA4_ACS7_STEPS lépés kiinduló értékenként);
#   2. a valódi gépen a memóriaterhelési próbát (a keret a cgroup-korlátot követi, a teli memória
#      várakozást ad, nem indítást);
#   3. a PostgreSQL-tár tesztjeit (ha van PostgreSQL), és a motor háttérfeladat-tesztjeit.
# A napló: target/acs7/<profil>-<idő>.log. Kilépési kód: 0, ha minden lépés zöld.
#
# Használat:  scripts/dev/acs7-stability.sh            # mindkét profil
#             CFA4_ACS7_PROFILES=minimum scripts/dev/acs7-stability.sh
# Felülírható: CFA4_ACS7_STEPS (alap: 200000), valamint a run-minspec.sh változói
# (CFA4_REF_VENDOR, CFA4_REF_GPU, ...).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
STEPS="${CFA4_ACS7_STEPS:-200000}"
PROFILES="${CFA4_ACS7_PROFILES:-minimum ajanlott}"
OUT="$ROOT/target/acs7"
mkdir -p "$OUT"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"

# előbb fordít, a korlátokon kívül, hogy a mérés csak a futást korlátozza
cargo test --release -p cfa4-tasks -p cfa4-store --no-run >/dev/null
cargo test --release -p cfa4-engine --lib --no-run >/dev/null

status=0
for profile in $PROFILES; do
  case "$profile" in
    minimum) limit_mib=16384 ;;
    ajanlott) limit_mib=32768 ;;
    *) echo "Ismeretlen profil: $profile" >&2; exit 64 ;;
  esac
  log="$OUT/$profile-$STAMP.log"
  echo "[acs7] profil=$profile, lépések=$STEPS, napló=$log" >&2
  run() {
    echo "== $*" >>"$log"
    if ! CFA4_PROFILE="$profile" CFA4_ACS7_STEPS="$STEPS" CFA4_EXPECT_RAM_LIMIT_MIB="$limit_mib" \
      scripts/dev/run-minspec.sh "$@" >>"$log" 2>&1; then
      echo "[acs7] HIBA ($profile): $*" >&2
      status=1
    fi
  }
  run cargo test --release -p cfa4-tasks --test stability
  run cargo test --release -p cfa4-tasks --test machine -- --ignored --nocapture
  run cargo test --release -p cfa4-store
  run cargo test --release -p cfa4-engine --lib -- task_host background a_queued_export
  grep -E '^test result|acs7:|skipped' "$log" | sed "s/^/[acs7 $profile] /" >&2 || true
done
exit "$status"
