#!/usr/bin/env bash
# CFA4 AI – parancs futtatása a működési minimumon.
# Minimum: 32 GB RAM (swap nélkül), 8 CPU-mag, egyetlen 12 GB-os referencia-GPU.
# Referenciakártyák: NVIDIA RTX 3060 12 GB, valamint egy 12 GB-os AMD és egy 12 GB-os Intel kártya.
#
# Használat:  scripts/dev/run-minspec.sh <parancs> [argumentumok...]
#   pl.       scripts/dev/run-minspec.sh cargo test --release -p cfa4-ai-workers
#             CFA4_REF_VENDOR=amd CFA4_REF_GPU=0 scripts/dev/run-minspec.sh python -m pytest
#
# Felülírható környezeti változók:
#   CFA4_MIN_RAM     (alap: 32G)
#   CFA4_MIN_CPUS    (alap: 8)
#   CFA4_REF_VENDOR  nvidia | amd | intel (alap: nvidia)
#   CFA4_REF_GPU     a kártya indexe az adott gyártó saját számozásában.
#                    NVIDIA-n elhagyható: az első RTX 3060 legalább 12 GB-tal (nvidia-smi index).
#                    AMD-n és Intelen kötelező (AMD: rocm-smi, Intel: Level Zero szerinti fizikai index).
set -euo pipefail

MIN_RAM="${CFA4_MIN_RAM:-32G}"
MIN_CPUS="${CFA4_MIN_CPUS:-8}"
VENDOR="${CFA4_REF_VENDOR:-nvidia}"
REF_GPU="${CFA4_REF_GPU:-}"

if [[ $# -eq 0 ]]; then
  echo "Használat: $0 <parancs> [argumentumok...]" >&2
  exit 64
fi

case "$VENDOR" in
  nvidia)
    if [[ -z "$REF_GPU" ]]; then
      if ! command -v nvidia-smi >/dev/null 2>&1; then
        echo "Nincs nvidia-smi: add meg kézzel a kártyát (CFA4_REF_GPU=<index>)" >&2
        exit 2
      fi
      REF_GPU=$(nvidia-smi --query-gpu=index,name,memory.total --format=csv,noheader,nounits \
        | awk -F', *' '$2 ~ /3060/ && $3 >= 12000 { print $1; exit }')
    fi
    # PCI_BUS_ID: a CUDA indexe így megegyezik az nvidia-smi indexével.
    GPU_ENV=(CUDA_DEVICE_ORDER=PCI_BUS_ID "CUDA_VISIBLE_DEVICES=$REF_GPU")
    ;;
  amd)
    GPU_ENV=("ROCR_VISIBLE_DEVICES=$REF_GPU" "HIP_VISIBLE_DEVICES=0" CUDA_VISIBLE_DEVICES=)
    ;;
  intel)
    # ZE_AFFINITY_MASK keeps only the physical card and renumbers it to 0,
    # so the SYCL selector must name the remapped index.
    GPU_ENV=("ZE_AFFINITY_MASK=$REF_GPU" "ONEAPI_DEVICE_SELECTOR=level_zero:0" CUDA_VISIBLE_DEVICES=)
    ;;
  *)
    echo "Ismeretlen CFA4_REF_VENDOR: $VENDOR (nvidia | amd | intel)" >&2
    exit 64
    ;;
esac

if [[ -z "$REF_GPU" ]]; then
  echo "Nincs megadva vagy nem található a referencia-GPU. Add meg: CFA4_REF_GPU=<index>" >&2
  exit 2
fi

LAST_CPU=$((MIN_CPUS - 1))
echo "[minspec] RAM=${MIN_RAM} (swap nélkül), CPU=0-${LAST_CPU}, GPU=${VENDOR}:${REF_GPU}" >&2

# MemorySwapMax=0: a swap ne takarja el a memórianyomást.
# A cgroup memóriakorlátja a gyerekfolyamatokra (Python workerek) is vonatkozik.
# A VRAM-ot nem korlátozzuk: a 12 GB-os kártya maga a határ.
exec systemd-run --user --scope --quiet \
  -p MemoryMax="$MIN_RAM" -p MemorySwapMax=0 \
  -- taskset -c "0-${LAST_CPU}" \
  env "${GPU_ENV[@]}" \
  "$@"
