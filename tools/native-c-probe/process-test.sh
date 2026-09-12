#!/bin/sh
# Host-side native process ABI behavior; this is not installed context proof.
set -eu
cd "$(dirname "$0")/../.."
mkdir -p build/native-c
/opt/homebrew/opt/llvm/bin/clang -std=c17 -Wall -Wextra -Werror -Isdk/compiler/include \
  -Dgetpid=infinity_test_getpid -Dgetsid=infinity_test_getsid \
  -Dgethostname=infinity_test_gethostname -Disatty=infinity_test_isatty \
  -Ddup2=infinity_test_dup2 -Dkill=infinity_test_kill \
  -Dsigaction=infinity_test_sigaction -Dsigprocmask=infinity_test_sigprocmask \
  -D_exit=infinity_test_exit sdk/compiler/platform.c sdk/compiler/process.c \
  tools/native-c-probe/process-test.c -o build/native-c/process-test
build/native-c/process-test
