#include "model/model.h"
#include "synthesis/phonemizer.h"
#include "synthesis/synth.h"
#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <memory>
#include <string>

static std::unique_ptr<kokopop::Model> model;
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
// FUNC: synthesize_phrase
// DESC: Produces one bounded phrase using the persistent private CPU model and releases its transient audio.
// ------------------=
static int synthesize_phrase(const char *text, size_t length, int16_t *pcm, size_t capacity, size_t *frames) {
    native_initialize();
    std::string error;
    native_phase = 1;
    if (!model) {
        const kokopop_model_options options{1, KOKOPOP_BACKEND_CPU};
        if (!kokopop::load_model_from_gguf("/kokoro.gguf", &options, model, error)) return 3;
        if (model->is_mock || model->sample_rate("af_heart") != 24000) { model.reset(); return 3; }
    }
    if (native_cancelled()) return 2;
    std::string phonemes;
    native_phase = 2;
    if (!kokopop::phonemize_text(std::string(text, length), "en-us", 'a', phonemes, error)) return 4;
    if (native_cancelled()) return 2;
    kokopop_audio audio{};
    native_phase = 3;
    if (!kokopop::synthesize_phonemes(*model, phonemes, "af_heart", 1.0f, audio, error))
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
        written += produced;
        position += count;
    }
    *frames = written;
    return 0;
}
