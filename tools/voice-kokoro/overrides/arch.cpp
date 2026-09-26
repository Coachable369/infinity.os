#include "model/arch.h"
#include "model/gguf_util.h"
#include "arch/kokoro/kokoro_arch.h"
namespace kokopop {
// ------------------------=
// FUNC: arch_name
// DESC: Names only the native provider's supported model architecture.
// ------------------=
const char *arch_name(Arch value) { return value == Arch::Kokoro ? "kokoro-82m" : "unknown"; }
// ------------------------=
// FUNC: peek_arch
// DESC: Validates Kokoro metadata without accepting an unrelated model as speech synthesis.
// ------------------=
Arch peek_arch(gguf_context *meta) {
    if (!meta) return Arch::Unknown;
    std::string name;
    if (gguf_get_str(meta, "kokopop.arch", name))
        return name == "kokoro-82m" || name == "kokoro" ? Arch::Kokoro : Arch::Unknown;
    uint32_t version = 0;
    return gguf_get_u32(meta, "kokopop.kokoro.version", version) ? Arch::Kokoro : Arch::Unknown;
}
// ------------------------=
// FUNC: create_arch
// DESC: Instantiates only the packaged Kokoro engine; unsupported architectures fail closed.
// ------------------=
std::unique_ptr<ModelArch> create_arch(gguf_context *meta, std::string &error) {
    if (peek_arch(meta) == Arch::Kokoro) return std::make_unique<KokoroArch>();
    error = "Unsupported native speech architecture";
    return nullptr;
}
}
