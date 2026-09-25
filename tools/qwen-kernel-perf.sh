#!/bin/sh
# Behavioral baseline comparison; run on an otherwise idle ARM64 host.
# Optional argument selects the pre-optimization Git revision.
set -eu
project_root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$project_root"
baseline=${1:-8ead345164081a67b7ea84319ac7e698b9a46a3b}
temp_base=${TMPDIR:-$project_root/build/tmp}
mkdir -p "$temp_base"
bench_dir=$(mktemp -d "${temp_base%/}/infinity-qwen-perf.XXXXXX")
git show "$baseline:kernel/runtime/ai/qwen/cpu_math.c" > "$bench_dir/baseline.c"
clang -O3 -ffp-contract=off -Dinfinity_qwen_dot=baseline_dot \
    -Dinfinity_qwen_dot_rows=baseline_dot_rows \
    -Dinfinity_qwen_dot_rows_cached=baseline_dot_rows_cached \
    -Dinfinity_attention_scores=baseline_attention_scores \
    -Dinfinity_attention_values=baseline_attention_values \
    -c "$bench_dir/baseline.c" -o "$bench_dir/baseline.o"
clang -O3 -ffp-contract=off tools/qwen-kernel-perf.c \
    kernel/runtime/ai/qwen/cpu_math.c "$bench_dir/baseline.o" -o "$bench_dir/compare"
"$bench_dir/compare"
# Keep the baseline source and binary for reproducibility and inspection.
printf 'Benchmark artifacts: %s\n' "$bench_dir" >&2
