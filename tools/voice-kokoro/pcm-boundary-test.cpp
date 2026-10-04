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
// FUNC: verify_join
// DESC: Compares an actual production phrase join with independent retained spans, including quiet edges and unchanged internal pauses.
// ------------------=
static void verify_join(const std::vector<int16_t> &left, const std::vector<int16_t> &right) {
    const int16_t sentinel = 12345;
    std::vector<int16_t> source(left);
    source.insert(source.end(), right.begin(), right.end());
    std::vector<int16_t> expected(source);
    auto last = std::find_if(left.rbegin(), left.rend(), [](int16_t value) { return value != 0; });
    auto first = std::find_if(right.begin(), right.end(), [](int16_t value) { return value != 0; });
    if (last != left.rend() && first != right.end()) {
        const size_t tail = size_t(last - left.rbegin());
        const size_t head = size_t(first - right.begin());
        const size_t gap = std::min(tail, size_t(480)) + std::min(head, size_t(480))
                         - std::min({tail, head, size_t(120)});
        expected.assign(left.begin(), left.end() - tail);
        expected.insert(expected.end(), gap, 0);
        expected.insert(expected.end(), right.begin() + head, right.end());
    }
    std::vector<int16_t> actual(source.size() + 2, sentinel);
    std::copy(source.begin(), source.end(), actual.begin() + 1);
    const size_t kept = native_pcm_join_phrases(actual.data() + 1, left.size(), right.size(), 480, 120);
    assert(kept == expected.size() && kept <= source.size());
    assert(actual.front() == sentinel && actual.back() == sentinel);
    assert(std::equal(expected.begin(), expected.end(), actual.begin() + 1));
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
    // The old energy-floor join reduced 4,000 quiet nonzero samples to 840.
    // Neither low amplitude nor absence of padding permits removing speech.
    verify_join(std::vector<int16_t>(2000, 1), std::vector<int16_t>(2000, -1));
    for (size_t left = 0; left < 32; ++left) {
        for (size_t right = 0; right < 32; ++right) {
            verify_join(std::vector<int16_t>(left, 0), std::vector<int16_t>(right, 0));
            verify_join(std::vector<int16_t>(left, 1), std::vector<int16_t>(right, -1));
            std::vector<int16_t> first(left, 0), second(right, 0);
            if (left) first[left / 2] = 1;
            if (right) second[right / 2] = -1;
            verify_join(first, second);
        }
    }
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
    verify_join(std::vector<int16_t>(phrase.begin(), phrase.begin() + frames),
                std::vector<int16_t>(phrase.begin(), phrase.begin() + frames));
    // Inference-boundary padding can be large; the complete internal 400 ms
    // pause and both one-LSB boundaries must survive regardless of amplitude.
    std::vector<int16_t> padded(6000, 0);
    padded.insert(padded.end(), phrase.begin(), phrase.begin() + frames);
    padded.insert(padded.end(), 6000, 0);
    verify_join(padded, padded);
    return 0;
}
