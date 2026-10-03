#ifndef INFINITY_KOKORO_PCM_BOUNDARY_H
#define INFINITY_KOKORO_PCM_BOUNDARY_H

#include <cstddef>
#include <cstdint>
#include <cstring>

// ------------------------=
// FUNC: native_pcm_trim_zero_edges
// DESC: Removes model-added exact silence around one job, retaining padding and every nonzero sample or internal pause.
// ------------------=
static size_t native_pcm_trim_zero_edges(int16_t *pcm, size_t frames, size_t padding) {
    size_t first = 0;
    while (first < frames && pcm[first] == 0) ++first;
    // Preserve the existing all-silent result contract; no speech may be
    // inferred from an energy threshold or manufactured for an empty result.
    if (first == frames) return frames;
    size_t last = frames;
    while (last > first && pcm[last - 1] == 0) --last;
    const size_t start = first > padding ? first - padding : 0;
    const size_t end = frames - last > padding ? last + padding : frames;
    const size_t kept = end - start;
    if (start) std::memmove(pcm, pcm + start, kept * sizeof(*pcm));
    if (kept < frames) std::memset(pcm + kept, 0, (frames - kept) * sizeof(*pcm));
    return kept;
}

#endif
