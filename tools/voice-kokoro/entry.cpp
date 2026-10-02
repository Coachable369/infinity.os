#include "model/model.h"
#include "synthesis/phonemizer.h"
#include "synthesis/synth.h"
#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <string>

static std::unique_ptr<kokopop::Model> model;
// A modest conversational pace reduces both generated mel frames and perceived
// response latency without the clipped cadence produced by aggressive rates.
static constexpr float conversational_speed = 1.10f;
extern "C" int native_cancelled(void);
extern "C" unsigned int native_phase;
extern "C" void (*native_init_start[])(void);
extern "C" void (*native_init_end[])(void);

// ------------------------=
// FUNC: native_initialize
// DESC: Initializes the private C++ image once on the single owning speech worker, never the kernel global constructor list.
// ------------------=
extern "C" void native_initialize(void) {
    static bool initialized = false;
    if (initialized) return;
    for (auto constructor = native_init_start; constructor != native_init_end; ++constructor) (*constructor)();
    initialized = true;
}

// ------------------------=
// FUNC: prepare_model
// DESC: Loads and validates the resident synthesis model once inside the bounded native speech arena.
// ------------------=
static bool prepare_model() {
    if (model) return true;
    std::string error;
    const kokopop_model_options options{1, KOKOPOP_BACKEND_CPU};
    if (!kokopop::load_model_from_gguf("/kokoro.gguf", &options, model, error)) return false;
    if (model->is_mock || model->sample_rate("af_heart") != 24000) {
        model.reset();
        return false;
    }
    return true;
}

// ------------------------=
// FUNC: native_prepare_synthesis
// DESC: Warms the complete synthesis model before the UI advertises conversational readiness.
// ------------------=
extern "C" int native_prepare_synthesis(void) {
    native_initialize();
    native_phase = 1;
    return native_cancelled() ? 2 : prepare_model() ? 0 : 3;
}

// ------------------------=
// FUNC: synthesize_phrase
// DESC: Produces one bounded phrase using the persistent private CPU model and releases its transient audio.
// ------------------=
static int synthesize_phrase(const char *text, size_t length, int16_t *pcm, size_t capacity, size_t *frames) {
    native_initialize();
    std::string error;
    native_phase = 1;
    if (!prepare_model()) return 3;
    if (native_cancelled()) return 2;
    std::string phonemes;
    native_phase = 2;
    if (!kokopop::phonemize_text(std::string(text, length), "en-us", 'a', phonemes, error)) return 4;
    if (native_cancelled()) return 2;
    kokopop_audio audio{};
    native_phase = 3;
    if (!kokopop::synthesize_phonemes(*model, phonemes, "af_heart", conversational_speed, audio, error))
        return native_cancelled() ? 2 : 5;
    int result = 0;
    native_phase = 4;
    if (audio.sample_rate != 24000 || !audio.n_samples || audio.n_samples > capacity) result = 6;
    else if (native_cancelled()) result = 2;
    else {
        for (size_t i = 0; i < audio.n_samples; ++i) {
            if (!std::isfinite(audio.samples[i])) { result = 6; break; }
            float value = audio.samples[i] * 32767.0f;
            pcm[i] = static_cast<int16_t>(std::round(std::max(-32768.0f, std::min(32767.0f, value))));
        }
        if (!result) *frames = audio.n_samples;
    }
    std::free(audio.samples);
    return result;
}

// ------------------------=
// FUNC: join_phrase
// DESC: Crossfades independently inferred Kokoro segments so internal text bounds cannot create clicks or discontinuities.
// ------------------=
static size_t join_phrase(int16_t *pcm, size_t written, size_t produced) {
    if (!written || !produced) return written + produced;
    const size_t overlap = std::min(size_t(120), std::min(written, produced));
    for (size_t i = 0; i < overlap; ++i) {
        const int32_t left = pcm[written - overlap + i];
        const int32_t right = pcm[written + i];
        const int32_t mixed = left * int32_t(overlap - i) + right * int32_t(i + 1);
        pcm[written - overlap + i] = static_cast<int16_t>(mixed / int32_t(overlap + 1));
    }
    std::memmove(pcm + written, pcm + written + overlap, (produced - overlap) * sizeof(int16_t));
    return written + produced - overlap;
}

// ------------------------=
// FUNC: fade_edges
// DESC: Applies a short equal-gain ramp to a complete utterance to prevent device-start and cancellation clicks.
// ------------------=
static void fade_edges(int16_t *pcm, size_t frames) {
    const size_t ramp = std::min(size_t(96), frames / 2);
    for (size_t i = 0; i < ramp; ++i) {
        pcm[i] = static_cast<int16_t>(int32_t(pcm[i]) * int32_t(i + 1) / int32_t(ramp + 1));
        pcm[frames - 1 - i] = static_cast<int16_t>(int32_t(pcm[frames - 1 - i]) * int32_t(i + 1) / int32_t(ramp + 1));
    }
}

// ------------------------=
// FUNC: native_run
// DESC: Keeps paragraph inference inside the fixed arena by synthesizing word-boundary phrases into bounded PCM.
// ------------------=
extern "C" int native_run(const char *text, size_t length, int16_t *pcm, size_t capacity, size_t *frames) {
    size_t position = 0, written = 0;
    while (position < length) {
        if (native_cancelled()) return 2;
        size_t count = std::min(size_t(44), length - position);
        if (count < length - position) {
            size_t boundary = count;
            while (boundary && text[position + boundary - 1] != ' ') --boundary;
            // Do not split a single long word into unrelated spoken fragments.
            if (!boundary) return 1;
            count = boundary;
        }
        if (written >= capacity) return 6;
        size_t produced = 0;
        int result = synthesize_phrase(text + position, count, pcm + written, capacity - written, &produced);
        if (result) return result;
        written = join_phrase(pcm, written, produced);
        position += count;
    }
    fade_edges(pcm, written);
    *frames = written;
    return 0;
}
