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

// ------------------------=
// FUNC: native_run
// DESC: Produces genuine Kokoro PCM from bounded English text using only the private native CPU model.
// ------------------=
extern "C" int native_run(const char *text, size_t length, int16_t *pcm, size_t capacity, size_t *frames) {
    std::string error;
    if (!model) {
        const kokopop_model_options options{1, KOKOPOP_BACKEND_CPU};
        if (!kokopop::load_model_from_gguf("/kokoro.gguf", &options, model, error)) return 3;
        if (model->is_mock || model->sample_rate("af_heart") != 24000) { model.reset(); return 3; }
    }
    if (native_cancelled()) return 2;
    std::string phonemes;
    if (!kokopop::phonemize_text(std::string(text, length), "en-us", 'a', phonemes, error)) return 4;
    if (native_cancelled()) return 2;
    kokopop_audio audio{};
    if (!kokopop::synthesize_phonemes(*model, phonemes, "af_heart", 1.0f, audio, error))
        return native_cancelled() ? 2 : 5;
    int result = 0;
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
