#ifndef HERMES_BINDING_HPP
#define HERMES_BINDING_HPP

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct HermesRt HermesRt;

enum HermesValueKind {
  HermesValueKind_Undefined = 0,
  HermesValueKind_Null = 1,
  HermesValueKind_Boolean = 2,
  HermesValueKind_Number = 3,
  HermesValueKind_Symbol = 4,
  HermesValueKind_BigInt = 5,
  HermesValueKind_String = 6,
  HermesValueKind_Object = 7,
};

struct HermesValue {
  enum HermesValueKind kind;
  union {
    bool boolean;
    double number;
    void* pointer; 
  } data;
};

typedef struct HermesValue (*HermesHostFunctionCallback)(
    HermesRt* rt,
    const struct HermesValue* this_val,
    const struct HermesValue* args,
    size_t arg_count,
    void* user_data);

typedef void (*HermesHostFunctionFinalizer)(void* user_data);

typedef struct HermesValue (*HermesHostObjectGetCallback)(
    HermesRt* rt,
    const void* name,  
    void* user_data);

typedef void (*HermesHostObjectSetCallback)(
    HermesRt* rt,
    const void* name,  
    const struct HermesValue* value,
    void* user_data);

typedef void** (*HermesHostObjectGetPropertyNamesCallback)(
    HermesRt* rt,
    size_t* out_count,
    void* user_data);

typedef void (*HermesHostObjectFinalizer)(void* user_data);

typedef void (*HermesNativeStateFinalizer)(void* data);

typedef struct HermesPreparedJs HermesPreparedJs;

struct HermesRuntimeConfig {
  bool enable_eval;
  bool es6_proxy;
  bool intl;
  bool microtask_queue;
  bool enable_generator;
  bool enable_block_scoping;
  bool enable_hermes_internal;
  bool enable_hermes_internal_test_methods;
  unsigned max_num_registers;
  bool enable_jit;
  bool force_jit;
  unsigned jit_threshold;
  unsigned jit_memory_limit;
  bool enable_async_generators;
  unsigned bytecode_warmup_percent;
  bool randomize_memory_layout;
};

typedef void (*HermesFatalHandler)(const char* msg, size_t len);

HermesRt* hermes__Runtime__New(void);
HermesRt* hermes__Runtime__NewWithConfig(const struct HermesRuntimeConfig* cfg);
void hermes__Runtime__Delete(HermesRt* rt);

bool hermes__Runtime__HasPendingError(const HermesRt* rt);
struct HermesValue hermes__Runtime__GetAndClearError(HermesRt* rt);
const char* hermes__Runtime__GetAndClearErrorMessage(HermesRt* rt);

void* hermes__Runtime__Global(HermesRt* rt);

size_t hermes__Runtime__Description(HermesRt* rt, char* buf, size_t buf_len);
bool hermes__Runtime__IsInspectable(HermesRt* rt);

struct HermesValue hermes__Runtime__EvaluateJavaScript(
    HermesRt* rt,
    const uint8_t* data,
    size_t len,
    const char* source_url,
    size_t source_url_len);

int hermes__Runtime__DrainMicrotasks(HermesRt* rt, int max_hint);

bool hermes__Runtime__QueueMicrotask(HermesRt* rt, const void* func);

struct HermesValue hermes__Runtime__CreateValueFromJsonUtf8(
    HermesRt* rt,
    const uint8_t* json,
    size_t len);

struct HermesValue hermes__Runtime__EvaluateJavaScriptWithSourceMap(
    HermesRt* rt,
    const uint8_t* data,
    size_t len,
    const uint8_t* source_map,
    size_t source_map_len,
    const char* source_url,
    size_t source_url_len);

HermesPreparedJs* hermes__Runtime__PrepareJavaScript(
    HermesRt* rt,
    const uint8_t* data,
    size_t len,
    const char* source_url,
    size_t source_url_len);

struct HermesValue hermes__Runtime__EvaluatePreparedJavaScript(
    HermesRt* rt,
    const HermesPreparedJs* prepared);

void hermes__PreparedJavaScript__Delete(HermesPreparedJs* prepared);

void* hermes__Scope__New(HermesRt* rt);
void hermes__Scope__Delete(void* scope);

void* hermes__String__CreateFromUtf8(
    HermesRt* rt,
    const uint8_t* utf8,
    size_t len);

void* hermes__String__CreateFromAscii(
    HermesRt* rt,
    const char* ascii,
    size_t len);

size_t hermes__String__ToUtf8(
    HermesRt* rt,
    const void* str,
    char* buf,
    size_t buf_len);

bool hermes__String__StrictEquals(
    HermesRt* rt,
    const void* a,
    const void* b);

void hermes__String__Release(void* pv);

void* hermes__PropNameID__ForAscii(HermesRt* rt, const char* str, size_t len);
void* hermes__PropNameID__ForUtf8(
    HermesRt* rt,
    const uint8_t* utf8,
    size_t len);
void* hermes__PropNameID__ForString(HermesRt* rt, const void* str);
void* hermes__PropNameID__ForSymbol(HermesRt* rt, const void* sym);

size_t hermes__PropNameID__ToUtf8(
    HermesRt* rt,
    const void* pni,
    char* buf,
    size_t buf_len);

bool hermes__PropNameID__Equals(
    HermesRt* rt,
    const void* a,
    const void* b);

void hermes__PropNameID__Release(void* pv);

void* hermes__Object__New(HermesRt* rt);

struct HermesValue hermes__Object__GetProperty__String(
    HermesRt* rt,
    const void* obj,
    const void* name);

struct HermesValue hermes__Object__GetProperty__PropNameID(
    HermesRt* rt,
    const void* obj,
    const void* name);

bool hermes__Object__SetProperty__String(
    HermesRt* rt,
    const void* obj,
    const void* name,
    const struct HermesValue* val);

bool hermes__Object__SetProperty__PropNameID(
    HermesRt* rt,
    const void* obj,
    const void* name,
    const struct HermesValue* val);

bool hermes__Object__HasProperty__String(
    HermesRt* rt,
    const void* obj,
    const void* name);

bool hermes__Object__HasProperty__PropNameID(
    HermesRt* rt,
    const void* obj,
    const void* name);

void* hermes__Object__GetPropertyNames(HermesRt* rt, const void* obj);

bool hermes__Object__IsArray(HermesRt* rt, const void* obj);
bool hermes__Object__IsFunction(HermesRt* rt, const void* obj);
bool hermes__Object__IsArrayBuffer(HermesRt* rt, const void* obj);

bool hermes__Object__StrictEquals(
    HermesRt* rt,
    const void* a,
    const void* b);

bool hermes__Object__InstanceOf(
    HermesRt* rt,
    const void* obj,
    const void* func);

void hermes__Object__SetExternalMemoryPressure(
    HermesRt* rt,
    const void* obj,
    size_t amount);

bool hermes__Object__HasNativeState(HermesRt* rt, const void* obj);
void* hermes__Object__GetNativeState(HermesRt* rt, const void* obj);
void hermes__Object__SetNativeState(
    HermesRt* rt,
    const void* obj,
    void* data,
    HermesNativeStateFinalizer finalizer);

void* hermes__Object__CreateFromHostObject(
    HermesRt* rt,
    HermesHostObjectGetCallback get_cb,
    HermesHostObjectSetCallback set_cb,
    HermesHostObjectGetPropertyNamesCallback get_names_cb,
    void* user_data,
    HermesHostObjectFinalizer finalizer);
void* hermes__Object__GetHostObject(HermesRt* rt, const void* obj);
bool hermes__Object__IsHostObject(HermesRt* rt, const void* obj);

bool hermes__Object__DeleteProperty__String(
    HermesRt* rt,
    const void* obj,
    const void* name);

bool hermes__Object__DeleteProperty__PropNameID(
    HermesRt* rt,
    const void* obj,
    const void* name);

bool hermes__Object__DeleteProperty__Value(
    HermesRt* rt,
    const void* obj,
    const struct HermesValue* name);

struct HermesValue hermes__Object__GetProperty__Value(
    HermesRt* rt,
    const void* obj,
    const struct HermesValue* name);

bool hermes__Object__SetProperty__Value(
    HermesRt* rt,
    const void* obj,
    const struct HermesValue* name,
    const struct HermesValue* val);

bool hermes__Object__HasProperty__Value(
    HermesRt* rt,
    const void* obj,
    const struct HermesValue* name);

void* hermes__Object__CreateWithPrototype(
    HermesRt* rt,
    const struct HermesValue* prototype);

bool hermes__Object__SetPrototype(
    HermesRt* rt,
    const void* obj,
    const struct HermesValue* prototype);

struct HermesValue hermes__Object__GetPrototype(
    HermesRt* rt,
    const void* obj);

void hermes__Object__Release(void* pv);

void* hermes__Array__New(HermesRt* rt, size_t length);
size_t hermes__Array__Size(HermesRt* rt, const void* arr);

struct HermesValue hermes__Array__GetValueAtIndex(
    HermesRt* rt,
    const void* arr,
    size_t index);

bool hermes__Array__SetValueAtIndex(
    HermesRt* rt,
    const void* arr,
    size_t index,
    const struct HermesValue* val);

void hermes__Array__Release(void* pv);

void* hermes__ArrayBuffer__New(HermesRt* rt, size_t size);
size_t hermes__ArrayBuffer__Size(HermesRt* rt, const void* buf);
uint8_t* hermes__ArrayBuffer__Data(HermesRt* rt, const void* buf);

struct HermesValue hermes__Function__Call(
    HermesRt* rt,
    const void* func,
    const struct HermesValue* this_val,
    const struct HermesValue* args,
    size_t argc);

struct HermesValue hermes__Function__CallAsConstructor(
    HermesRt* rt,
    const void* func,
    const struct HermesValue* args,
    size_t argc);

void* hermes__Function__CreateFromHostFunction(
    HermesRt* rt,
    const void* name,
    unsigned int param_count,
    HermesHostFunctionCallback callback,
    void* user_data,
    HermesHostFunctionFinalizer finalizer);

bool hermes__Function__IsHostFunction(HermesRt* rt, const void* func);

void hermes__Function__Release(void* pv);

void hermes__Value__Release(struct HermesValue* val);
bool hermes__Value__StrictEquals(
    HermesRt* rt,
    const struct HermesValue* a,
    const struct HermesValue* b);

void* hermes__Value__ToString(HermesRt* rt, const struct HermesValue* val);

struct HermesValue hermes__Value__Clone(
    HermesRt* rt,
    const struct HermesValue* val);

void* hermes__Symbol__ToString(HermesRt* rt, const void* sym);
bool hermes__Symbol__StrictEquals(
    HermesRt* rt,
    const void* a,
    const void* b);
void hermes__Symbol__Release(void* pv);

void* hermes__BigInt__FromInt64(HermesRt* rt, int64_t val);
void* hermes__BigInt__FromUint64(HermesRt* rt, uint64_t val);
bool hermes__BigInt__IsInt64(HermesRt* rt, const void* bi);
bool hermes__BigInt__IsUint64(HermesRt* rt, const void* bi);
uint64_t hermes__BigInt__Truncate(HermesRt* rt, const void* bi);
int64_t hermes__BigInt__GetInt64(HermesRt* rt, const void* bi);
void* hermes__BigInt__ToString(HermesRt* rt, const void* bi, int radix);
bool hermes__BigInt__StrictEquals(
    HermesRt* rt,
    const void* a,
    const void* b);
void hermes__BigInt__Release(void* pv);

void* hermes__WeakObject__Create(HermesRt* rt, const void* obj);
struct HermesValue hermes__WeakObject__Lock(HermesRt* rt, const void* wo);
void hermes__WeakObject__Release(void* pv);

bool hermes__IsHermesBytecode(const uint8_t* data, size_t len);
uint32_t hermes__GetBytecodeVersion(void);
void hermes__PrefetchHermesBytecode(const uint8_t* data, size_t len);
bool hermes__HermesBytecodeSanityCheck(const uint8_t* data, size_t len);

void hermes__Runtime__WatchTimeLimit(HermesRt* rt, uint32_t timeout_ms);
void hermes__Runtime__UnwatchTimeLimit(HermesRt* rt);
void hermes__Runtime__AsyncTriggerTimeout(HermesRt* rt);

void hermes__EnableSamplingProfiler(void);
void hermes__DisableSamplingProfiler(void);
void hermes__DumpSampledTraceToFile(const char* filename);

void hermes__SetFatalHandler(HermesFatalHandler handler);

const uint8_t* hermes__GetBytecodeEpilogue(
    const uint8_t* data,
    size_t len,
    size_t* out_epilogue_len);

bool hermes__IsCodeCoverageProfilerEnabled(void);
void hermes__EnableCodeCoverageProfiler(void);
void hermes__DisableCodeCoverageProfiler(void);

void hermes__Runtime__RegisterForProfiling(HermesRt* rt);
void hermes__Runtime__UnregisterForProfiling(HermesRt* rt);

bool hermes__Runtime__LoadSegment(
    HermesRt* rt,
    const uint8_t* data,
    size_t len,
    const struct HermesValue* context);

uint64_t hermes__Object__GetUniqueID(HermesRt* rt, const void* obj);
uint64_t hermes__String__GetUniqueID(HermesRt* rt, const void* str);
uint64_t hermes__Symbol__GetUniqueID(HermesRt* rt, const void* sym);
uint64_t hermes__BigInt__GetUniqueID(HermesRt* rt, const void* bi);
uint64_t hermes__PropNameID__GetUniqueID(HermesRt* rt, const void* pni);
uint64_t hermes__Value__GetUniqueID(
    HermesRt* rt,
    const struct HermesValue* val);

void hermes__Runtime__ResetTimezoneCache(HermesRt* rt);

#ifdef __cplusplus
} 
#endif

#endif 
