#include "pcm-boundary.h"
#include <algorithm>
#include <cassert>
#include <cstdio>
#include <limits>
#include <vector>

// ------------------------=
// FUNC: verify_case
// DESC: Checks exact retained samples, bounds sentinels, internal silence and erasure against an independently selected span.
// ------------------=
static void verify_case(const std::vector<int16_t> &source, size_t padding) {
    const int16_t sentinel = 12345;
    std::vector<int16_t> actual(source.size() + 2, sentinel);
    std::copy(source.begin(), source.end(), actual.begin() + 1);
    auto first = std::find_if(source.begin(), source.end(), [](int16_t value) { return value != 0; });
    auto last = std::find_if(source.rbegin(), source.rend(), [](int16_t value) { return value != 0; });
    size_t begin = 0, end = source.size();
    if (first != source.end()) {
        size_t prefix = size_t(first - source.begin());
        size_t suffix = size_t(last - source.rbegin());
        begin = prefix > padding ? prefix - padding : 0;
        end -= suffix > padding ? suffix - padding : 0;
    }
    size_t kept = native_pcm_trim_zero_edges(actual.data() + 1, source.size(), padding);
    assert(kept == end - begin);
    assert(actual.front() == sentinel && actual.back() == sentinel);
    assert(std::equal(source.begin() + begin, source.begin() + end, actual.begin() + 1));
    assert(std::all_of(actual.begin() + 1 + kept, actual.end() - 1, [](int16_t value) { return value == 0; }));
}

// ------------------------=
// FUNC: main
// DESC: Exercises tiny, silent, quiet, maximal, pause-bearing and consecutive native PCM spans; optionally transforms raw guest PCM.
// ------------------=
int main(int argc, char **argv) {
    if (argc == 3) {
        FILE *input = std::fopen(argv[1], "rb");
        assert(input);
        std::vector<int16_t> samples;
        int16_t sample;
        while (std::fread(&sample, sizeof(sample), 1, input) == 1) samples.push_back(sample);
        assert(!std::ferror(input));
        std::fclose(input);
        verify_case(samples, 480);
        size_t kept = native_pcm_trim_zero_edges(samples.data(), samples.size(), 480);
        FILE *output = std::fopen(argv[2], "wb");
        assert(output);
        assert(std::fwrite(samples.data(), sizeof(sample), kept, output) == kept);
        assert(std::fclose(output) == 0);
        return 0;
    }
    assert(argc == 1);
    for (size_t frames = 0; frames < 32; ++frames) {
        for (size_t padding : {size_t(0), size_t(1), size_t(480), std::numeric_limits<size_t>::max()}) {
            verify_case(std::vector<int16_t>(frames, 0), padding);
            verify_case(std::vector<int16_t>(frames, 1), padding);
            for (size_t first = 0; first < frames; ++first) {
                for (size_t last = first; last < frames; ++last) {
                    std::vector<int16_t> pcm(frames, 0);
                    pcm[first] = 1;
                    pcm[last] = -1;
                    verify_case(pcm, padding);
                }
            }
        }
    }
    std::vector<int16_t> phrase(24000, 0);
    // Preserve even one-LSB consonants and 400 ms of true internal silence.
    phrase[6000] = 1;
    phrase[6001] = std::numeric_limits<int16_t>::min();
    phrase[6002] = std::numeric_limits<int16_t>::max();
    phrase[15603] = -1;
    verify_case(phrase, 480);
    size_t frames = native_pcm_trim_zero_edges(phrase.data(), phrase.size(), 480);
    assert(frames == 10564);
    assert(phrase[480] == 1 && phrase[10083] == -1);
    std::vector<int16_t> response(phrase.begin(), phrase.begin() + frames);
    response.insert(response.end(), phrase.begin(), phrase.begin() + frames);
    // Concatenated jobs have just the 20 ms retained guard on either side.
    assert(response[frames - 481] == -1 && response[frames + 480] == 1);
    assert(std::all_of(response.begin() + frames - 480, response.begin() + frames + 480,
                       [](int16_t value) { return value == 0; }));
    return 0;
}
