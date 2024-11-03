#ifndef RUSTC_LLVM_LLVMWRAPPER_H
#define RUSTC_LLVM_LLVMWRAPPER_H

#include "SuppressLLVMWarnings.h"

#include "llvm/Support/raw_ostream.h"
#include <cstddef>
#include <cstdint>

#define LLVM_VERSION_GE(major, minor)                                          \
  (LLVM_VERSION_MAJOR > (major) ||                                             \
   LLVM_VERSION_MAJOR == (major) && LLVM_VERSION_MINOR >= (minor))

#define LLVM_VERSION_LT(major, minor) (!LLVM_VERSION_GE((major), (minor)))

extern "C" void LLVMRustSetLastError(const char *);

enum class LLVMRustResult { Success, Failure };

enum LLVMRustAttribute {
  AlwaysInline = 0,
  ByVal = 1,
  Cold = 2,
  InlineHint = 3,
  MinSize = 4,
  Naked = 5,
  NoAlias = 6,
  NoCapture = 7,
  NoInline = 8,
  NonNull = 9,
  NoRedZone = 10,
  NoReturn = 11,
  NoUnwind = 12,
  OptimizeForSize = 13,
  ReadOnly = 14,
  SExt = 15,
  StructRet = 16,
  UWTable = 17,
  ZExt = 18,
  InReg = 19,
  SanitizeThread = 20,
  SanitizeAddress = 21,
  SanitizeMemory = 22,
  NonLazyBind = 23,
  OptimizeNone = 24,
  ReadNone = 26,
  SanitizeHWAddress = 28,
  WillReturn = 29,
  StackProtectReq = 30,
  StackProtectStrong = 31,
  StackProtect = 32,
  NoUndef = 33,
  SanitizeMemTag = 34,
  NoCfCheck = 35,
  ShadowCallStack = 36,
  AllocSize = 37,
  AllocatedPointer = 38,
  AllocAlign = 39,
  SanitizeSafeStack = 40,
  FnRetThunkExtern = 41,
  Writable = 42,
  DeadOnUnwind = 43,
};

typedef struct OpaqueRustString *RustStringRef;
typedef struct LLVMOpaqueTwine *LLVMTwineRef;
typedef struct LLVMOpaqueSMDiagnostic *LLVMSMDiagnosticRef;

extern "C" void LLVMRustStringWriteImpl(RustStringRef Str, const char *Ptr,
                                        size_t Size);

class RawRustStringOstream : public llvm::raw_ostream {
  RustStringRef Str;
  uint64_t Pos;

  void write_impl(const char *Ptr, size_t Size) override {
    LLVMRustStringWriteImpl(Str, Ptr, Size);
    Pos += Size;
  }

  uint64_t current_pos() const override { return Pos; }

public:
  explicit RawRustStringOstream(RustStringRef Str) : Str(Str), Pos(0) {}

  ~RawRustStringOstream() {
    // LLVM requires this.
    flush();
  }
};

#endif // RUSTC_LLVM_LLVMWRAPPER_H
