#include <whisper.h>
#include <ggml-backend.h>
#include <ggml-cpu.h>
#include <algorithm>
#include <cctype>
#include <cstdint>
#include <cstring>
#include <vector>

extern "C" const unsigned char whisper_model[];
extern "C" const size_t whisper_model_length;
extern "C" int native_cancelled(void);
extern "C" void native_initialize(void);
static whisper_context *context;

// ------------------------=
// FUNC: prepare_context
// DESC: Builds the resident offline recognizer before microphone admission so user speech never pays model startup latency.
// ------------------=
static bool prepare_context() {
    native_initialize();
    if (context) return true;
    whisper_context_params model = whisper_context_default_params();
    model.use_gpu = false;
    model.flash_attn = false;
    context = whisper_init_from_buffer_with_params(
        const_cast<unsigned char *>(whisper_model), whisper_model_length, model);
    return context != nullptr;
}

// ------------------------=
// FUNC: ggml_backend_reg_count
// DESC: Exposes the single statically linked CPU backend without dynamic discovery.
// ------------------=
extern "C" size_t ggml_backend_reg_count() {
    return 1;
}

// ------------------------=
// FUNC: ggml_backend_dev_count
// DESC: Reports devices from the one statically linked CPU registry.
// ------------------=
extern "C" size_t ggml_backend_dev_count() {
    return ggml_backend_reg_dev_count(ggml_backend_cpu_reg());
}

// ------------------------=
// FUNC: ggml_backend_dev_get
// DESC: Resolves a bounded CPU device index without filesystem or plugin loading.
// ------------------=
extern "C" ggml_backend_dev_t ggml_backend_dev_get(size_t index) {
    return index < ggml_backend_dev_count() ? ggml_backend_reg_dev_get(ggml_backend_cpu_reg(), index) : nullptr;
}

// ------------------------=
// FUNC: ggml_backend_dev_by_type
// DESC: Finds one statically linked device matching the requested native backend type.
// ------------------=
extern "C" ggml_backend_dev_t ggml_backend_dev_by_type(enum ggml_backend_dev_type type) {
    for (size_t index = 0; index < ggml_backend_dev_count(); ++index) {
        ggml_backend_dev_t device = ggml_backend_dev_get(index);
        if (ggml_backend_dev_type(device) == type) return device;
    }
    return nullptr;
}

// ------------------------=
// FUNC: ggml_backend_init_by_type
// DESC: Initializes only a statically linked device and never a host-loaded backend.
// ------------------=
extern "C" ggml_backend_t ggml_backend_init_by_type(enum ggml_backend_dev_type type, const char *params) {
    ggml_backend_dev_t device = ggml_backend_dev_by_type(type);
    return device ? ggml_backend_dev_init(device, params) : nullptr;
}

// ------------------------=
// FUNC: native_prepare
// DESC: Warms the complete embedded Whisper context without accepting or retaining microphone content.
// ------------------=
extern "C" int native_prepare() {
    if (native_cancelled()) return 2;
    return prepare_context() ? 0 : 3;
}

// ------------------------=
// FUNC: should_continue
// DESC: Aborts encoder or graph work as soon as the owning native request is cancelled.
// ------------------=
static bool should_continue(whisper_context *, whisper_state *, void *) {
    return native_cancelled() == 0;
}

// ------------------------=
// FUNC: should_abort
// DESC: Adapts the native cancellation boundary to GGML computation.
// ------------------=
static bool should_abort(void *) {
    return native_cancelled() != 0;
}

// ------------------------=
// FUNC: append_segment
// DESC: Copies one decoded UTF-8 segment into a bounded normalized transcript.
// ------------------=
static bool append_segment(char *output, size_t capacity, size_t &written, const char *segment) {
    if (!segment) return true;
    while (*segment && std::isspace(static_cast<unsigned char>(*segment))) ++segment;
    size_t length = std::strlen(segment);
    while (length && std::isspace(static_cast<unsigned char>(segment[length - 1]))) --length;
    if (!length) return true;
    size_t separator = written ? 1 : 0;
    if (separator + length >= capacity - written) return false;
    if (separator) output[written++] = ' ';
    std::memcpy(output + written, segment, length);
    written += length;
    output[written] = 0;
    return true;
}

// ------------------------=
// FUNC: native_transcribe
// DESC: Runs pinned English Whisper inference over one bounded 16 kHz utterance without host services.
// ------------------=
extern "C" int native_transcribe(const int16_t *pcm, size_t samples, char *output,
                                  size_t capacity, size_t *length) {
    if (!pcm || !samples || samples > 160000 || !output || capacity < 2 || !length) return 1;
    *length = 0;
    output[0] = 0;
    if (native_cancelled()) return 2;
    if (!prepare_context()) return 3;
    std::vector<float> input(samples);
    for (size_t index = 0; index < samples; ++index) input[index] = pcm[index] / 32768.0f;
    whisper_full_params params = whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    params.n_threads = 1;
    params.language = "en";
    params.translate = false;
    params.initial_prompt = "Infinity. Computer.";
    params.carry_initial_prompt = false;
    params.no_context = true;
    params.no_timestamps = true;
    params.single_segment = true;
    params.print_special = false;
    params.print_progress = false;
    params.print_realtime = false;
    params.print_timestamps = false;
    params.suppress_blank = true;
    params.suppress_nst = true;
    params.temperature = 0.0f;
    params.temperature_inc = 0.0f;
    params.greedy.best_of = 1;
    params.encoder_begin_callback = should_continue;
    params.abort_callback = should_abort;
    if (whisper_full(context, params, input.data(), static_cast<int>(input.size())) != 0) {
        int result = native_cancelled() ? 2 : 4;
        return result;
    }
    size_t written = 0;
    int count = whisper_full_n_segments(context);
    for (int index = 0; index < count; ++index) {
        if (!append_segment(output, capacity, written, whisper_full_get_segment_text(context, index))) {
            output[0] = 0;
            return 6;
        }
    }
    if (!written) return 5;
    *length = written;
    return 0;
}
