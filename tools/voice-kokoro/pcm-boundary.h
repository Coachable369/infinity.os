#ifndef INFINITY_KOKORO_PCM_BOUNDARY_H
#define INFINITY_KOKORO_PCM_BOUNDARY_H

#include <cstddef>
#include <cstdint>
#include <cstring>

// ------------------------=
// FUNC: native_pcm_join_phrases
// DESC: Joins adjacent bounded inference results, removing only exact-zero edge padding and preserving every nonzero sample and internal pause.
// ------------------=
static size_t native_pcm_join_phrases(int16_t *pcm, size_t written, size_t produced,
                                     size_t padding, size_t max_overlap) {
    if (!written || !produced) return written + produced;
    size_t left = written;
    while (left && pcm[left - 1] == 0) --left;
    size_t right = 0;
    while (right < produced && pcm[written + right] == 0) ++right;
    // A silent model result is not evidence of an acoustic boundary. Preserve
    // its duration, just as the outer job-edge helper does.
    if (!left || right == produced) return written + produced;
    const size_t left_zeros = written - left < padding ? written - left : padding;
    const size_t right_zeros = right < padding ? right : padding;
    const size_t retained_left = left + left_zeros;
    const size_t removed_right = right - right_zeros;
    // Overlap only silence; an unconditional crossfade can attenuate or remove
    // quiet consonants even when neither input has any zero padding.
    size_t overlap = left_zeros < right_zeros ? left_zeros : right_zeros;
    if (overlap > max_overlap) overlap = max_overlap;
    const size_t destination = retained_left - overlap;
    const size_t remaining = produced - removed_right;
    std::memmove(pcm + destination, pcm + written + removed_right,
                 remaining * sizeof(*pcm));
    const size_t kept = destination + remaining;
    if (kept < written + produced)
        std::memset(pcm + kept, 0, (written + produced - kept) * sizeof(*pcm));
    return kept;
}

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
