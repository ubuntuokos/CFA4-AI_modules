#!/usr/bin/env bash
# Regression test for scripts/dev/run-minspec.sh without a GPU or systemd:
# fake systemd-run, taskset and nvidia-smi on PATH record what the wrapper
# would execute.
set -euo pipefail
cd "$(dirname "$0")/../.."

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
BIN="$WORK/bin"
mkdir -p "$BIN"

# systemd-run: record the cgroup properties, then run what follows "--".
cat >"$BIN/systemd-run" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$CFA4_TEST_LOG.systemd"
while [[ $# -gt 0 && "$1" != "--" ]]; do shift; done
shift
exec "$@"
SH
# taskset: record the CPU list, then run the command.
cat >"$BIN/taskset" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$2" >"$CFA4_TEST_LOG.taskset"
shift 2
exec "$@"
SH
# nvidia-smi: a 24 GB card at index 0, the 12 GB RTX 3060 at index 1.
cat >"$BIN/nvidia-smi" <<'SH'
#!/usr/bin/env bash
echo "0, NVIDIA GeForce RTX 4090, 24564"
echo "1, NVIDIA GeForce RTX 3060, 12288"
SH
chmod +x "$BIN"/*

fail() { echo "FAIL: $*" >&2; exit 1; }

run() {
  local log="$WORK/log"
  rm -f "$log".*
  CFA4_TEST_LOG="$log" PATH="$BIN:$PATH" "$@" \
    bash scripts/dev/run-minspec.sh env >"$log.env" 2>"$log.err"
}

# Default NVIDIA: picks the 3060, caps RAM at 32G without swap, 8 CPUs.
run env
grep -qx 'MemoryMax=32G' "$WORK/log.systemd" || fail "MemoryMax=32G missing"
grep -qx 'MemorySwapMax=0' "$WORK/log.systemd" || fail "MemorySwapMax=0 missing"
grep -qx '0-7' "$WORK/log.taskset" || fail "CPU list is not 0-7"
grep -qx 'CUDA_VISIBLE_DEVICES=1' "$WORK/log.env" || fail "3060 (index 1) not selected"
grep -qx 'CUDA_DEVICE_ORDER=PCI_BUS_ID' "$WORK/log.env" || fail "PCI_BUS_ID order missing"

# Overrides: RAM, CPUs and an explicit card.
run env CFA4_MIN_RAM=16G CFA4_MIN_CPUS=4 CFA4_REF_GPU=0
grep -qx 'MemoryMax=16G' "$WORK/log.systemd" || fail "CFA4_MIN_RAM ignored"
grep -qx '0-3' "$WORK/log.taskset" || fail "CFA4_MIN_CPUS ignored"
grep -qx 'CUDA_VISIBLE_DEVICES=0' "$WORK/log.env" || fail "CFA4_REF_GPU ignored"

# AMD and Intel hide CUDA devices and need an explicit index.
run env CFA4_REF_VENDOR=amd CFA4_REF_GPU=2
grep -qx 'ROCR_VISIBLE_DEVICES=2' "$WORK/log.env" || fail "AMD card not selected"
grep -qx 'CUDA_VISIBLE_DEVICES=' "$WORK/log.env" || fail "AMD run must hide CUDA devices"
run env CFA4_REF_VENDOR=intel CFA4_REF_GPU=3
grep -qx 'ONEAPI_DEVICE_SELECTOR=level_zero:3' "$WORK/log.env" || fail "Intel card not selected"
if run env CFA4_REF_VENDOR=amd; then fail "AMD without CFA4_REF_GPU must fail"; fi
if run env CFA4_REF_VENDOR=other CFA4_REF_GPU=0; then fail "unknown vendor must fail"; fi

# No command: usage error, nothing runs.
if CFA4_TEST_LOG="$WORK/log" PATH="$BIN:$PATH" bash scripts/dev/run-minspec.sh 2>/dev/null; then
  fail "missing command must fail"
fi

echo "PASS: run-minspec wrapper limits (RAM, swap, CPU, GPU selection)"
