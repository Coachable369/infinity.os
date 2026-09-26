#include <ggml-backend.h>
#include <ggml-cpu.h>
#include <cstring>

// ------------------------=
// FUNC: ggml_backend_reg_count
// DESC: Exposes exactly the statically linked CPU provider.
// ------------------=
size_t ggml_backend_reg_count() { return 1; }
// ------------------------=
// FUNC: ggml_backend_reg_get
// DESC: Rejects indices outside the single native backend.
// ------------------=
ggml_backend_reg_t ggml_backend_reg_get(size_t i) { return i == 0 ? ggml_backend_cpu_reg() : nullptr; }
// ------------------------=
// FUNC: ggml_backend_reg_by_name
// DESC: Resolves a static backend without loading a shared library.
// ------------------=
ggml_backend_reg_t ggml_backend_reg_by_name(const char *name) {
    auto reg = ggml_backend_cpu_reg();
    return name && !std::strcmp(name, ggml_backend_reg_name(reg)) ? reg : nullptr;
}
// ------------------------=
// FUNC: ggml_backend_dev_count
// DESC: Enumerates only the native CPU device.
// ------------------=
size_t ggml_backend_dev_count() { return 1; }
// ------------------------=
// FUNC: ggml_backend_dev_get
// DESC: Returns the statically linked CPU device for a valid index.
// ------------------=
ggml_backend_dev_t ggml_backend_dev_get(size_t i) { return i == 0 ? ggml_backend_reg_dev_get(ggml_backend_cpu_reg(), 0) : nullptr; }
// ------------------------=
// FUNC: ggml_backend_dev_by_name
// DESC: Resolves an exact CPU device name.
// ------------------=
ggml_backend_dev_t ggml_backend_dev_by_name(const char *name) {
    auto dev = ggml_backend_dev_get(0);
    return name && !std::strcmp(name, ggml_backend_dev_name(dev)) ? dev : nullptr;
}
// ------------------------=
// FUNC: ggml_backend_dev_by_type
// DESC: Rejects unsupported GPU and accelerator devices.
// ------------------=
ggml_backend_dev_t ggml_backend_dev_by_type(enum ggml_backend_dev_type type) { return type == GGML_BACKEND_DEVICE_TYPE_CPU ? ggml_backend_dev_get(0) : nullptr; }
// ------------------------=
// FUNC: ggml_backend_init_by_name
// DESC: Initializes only an explicitly supported static device.
// ------------------=
ggml_backend_t ggml_backend_init_by_name(const char *name, const char *params) {
    auto dev = ggml_backend_dev_by_name(name); return dev ? ggml_backend_dev_init(dev, params) : nullptr;
}
// ------------------------=
// FUNC: ggml_backend_init_by_type
// DESC: Initializes the CPU without dynamic backend discovery.
// ------------------=
ggml_backend_t ggml_backend_init_by_type(enum ggml_backend_dev_type type, const char *params) {
    auto dev = ggml_backend_dev_by_type(type); return dev ? ggml_backend_dev_init(dev, params) : nullptr;
}
// ------------------------=
// FUNC: ggml_backend_init_best
// DESC: Selects the sole packaged native CPU implementation.
// ------------------=
ggml_backend_t ggml_backend_init_best() { return ggml_backend_cpu_init(); }
// ------------------------=
// FUNC: ggml_backend_load
// DESC: Denies all dynamic library loading.
// ------------------=
ggml_backend_reg_t ggml_backend_load(const char *) { return nullptr; }
// ------------------------=
// FUNC: ggml_backend_unload
// DESC: Static backend lifetime is the native image lifetime.
// ------------------=
void ggml_backend_unload(ggml_backend_reg_t) {}
// ------------------------=
// FUNC: ggml_backend_load_all
// DESC: Performs no host library discovery.
// ------------------=
void ggml_backend_load_all() {}
// ------------------------=
// FUNC: ggml_backend_load_all_from_path
// DESC: Ignores host search paths because all providers are statically linked.
// ------------------=
void ggml_backend_load_all_from_path(const char *) {}
