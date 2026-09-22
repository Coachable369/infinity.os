#!/bin/sh
# Primary Hermes payload with Ministral as the secondary installed local model.
set -eu
cd "$(dirname "$0")/.."
export INFINITY_HERMES_MODEL=model-cache/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf
export QWEN_ISO_OUTPUT=build/hermes/InfinityOS-Hermes-Qwen-aarch64.iso
if ! test -f "$INFINITY_HERMES_MODEL"; then
    mkdir -p model-cache
    curl -fL --retry 3 -o "$INFINITY_HERMES_MODEL.part" 'https://huggingface.co/NousResearch/Hermes-3-Llama-3.2-3B-GGUF/resolve/3cd927095d8cbab12c743f932aa63b6f7bbfa141/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf'
    mv "$INFINITY_HERMES_MODEL.part" "$INFINITY_HERMES_MODEL"
fi
exec sh tools/build-qwen.sh
