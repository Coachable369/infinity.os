#include "llvm/TargetParser/Host.h"
#include "llvm/TargetParser/Triple.h"
#include "llvm/Config/llvm-config.h"
#include <cstring>
#include <cassert>
#define __INFINITY__ 1
#include "TargetParser/Unix/Host.inc"
// ------------------------=
// FUNC: main
// DESC: Exercises the actual native target-version path without consulting host OS identity.
// ------------------=
int main() {
  for (auto Target : {"x86_64-unknown-elf", "aarch64-unknown-elf",
                     "aarch64-apple-darwin", "powerpc-ibm-aix"}) {
    llvm::Triple Before(Target);
    llvm::Triple After(updateTripleOSVersion(Target));
    assert(Before == After);
  }
  return 0;
}
