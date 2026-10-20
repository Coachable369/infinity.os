use super::{native_memory::*, native_c_image::Image, cpu::{self, Tables, Controller, Context, MACHINE}, emit, finish, IMAGE};
#[cfg(feature = "python-pip-probe")]
#[path = "pip.rs"] mod pip;
const COMPILE: bool = option_env!("INFINITY_COMPILER_SOURCE").is_some();
const PYTHON: bool = option_env!("INFINITY_PYTHON_PROBE").is_some();
const PYTHON_EXTENSION: bool = option_env!("INFINITY_PYTHON_EXTENSION_PROBE").is_some();
const PYTHON_VALIDATION: bool = option_env!("INFINITY_PYTHON_VALIDATION_PROBE").is_some();
const PYTHON_INVALID: bool = option_env!("INFINITY_PYTHON_INVALID_PROBE").is_some();
#[path = "../../kernel/runtime/native_python_workflow.rs"] mod python_workflow;
static EXTENSION: &[u8] = include_bytes!(env!("INFINITY_PYTHON_EXTENSION_BYTES"));
const EXTENSION_SOURCE: &[u8] = b"import sys\nimport infinity_native_test as native\nassert native.add(-5, 8) == 3\ntry:\n    native.add(9223372036854775807, 1)\nexcept OverflowError:\n    pass\nelse:\n    raise AssertionError('overflow was not rejected')\nprint(native.add(19, 35))\nsys.exit(native.add(19, 35))\n";
const PYTHON_SOURCE: &[u8] = b"import sys\nfrom collections import Counter\nclass Calculator:\n    def total(self, values):\n        return sum(x * x for x in values)\ncounts = Counter(['a', 'b', 'a'])\nassert counts['a'] == 2\nassert 'Infinity'.upper() == 'INFINITY'\nprint('Hello from native Python')\nsys.exit(Calculator().total([1, 2, 7]))\n";
const INTERACTIVE:bool=option_env!("INFINITY_INTERACTIVE_APP").is_some();
const SOURCE: &[u8] = b"#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <ctype.h>\nstatic int compare(const void *a,const void *b){return *(const int*)a-*(const int*)b;}\nint main(void){int *v=calloc(4,sizeof *v);if(!v)return 90;v[0]=7;v[1]=1;v[2]=5;v[3]=3;qsort(v,4,sizeof *v,compare);int key=5;int *found=bsearch(&key,v,4,sizeof *v,compare);char out[32];int n=snprintf(out,sizeof out,\"%s:%d\",\"native\",found?*found:0);int ok=n==8&&!strcmp(out,\"native:5\")&&toupper('c')=='C'&&puts(out)>=0;free(v);return ok?54:91;}\n";
const LINK: bool = option_env!("INFINITY_LINK_INPUT").is_some();
static LINK_INPUT: &[u8] = include_bytes!(env!("INFINITY_LINK_INPUT_BYTES"));
static LINK_SCRIPT: &[u8] = include_bytes!("../../sdk/c/application.ld");
static CRT: &[u8] = include_bytes!(env!("INFINITY_APPLICATION_CRT"));

// ------------------------=
// FUNC: execute_linked
// DESC: Loads the guest-linked artifact into a new private process and checks its real exit value.
// ------------------=
unsafe fn execute_linked(bytes: &[u8], memory: &mut NativeMemory, kernel: KernelAccessMap, controller: Controller) -> u64 {
    let mut child = super::native_app_task::Task::start(bytes,super::execution::SecurityIdentity([2;16]),
        2,memory,kernel,controller,u64::MAX).unwrap();
    let mut offset=0;
    let input=b"2 + 3\n9 * 6\nquit\n";
    for _ in 0..1000 {
        let result = match child.poll(0,true, |_, tables, operation, args| {
            if operation==0x1003 && INTERACTIVE {
                let count=(args[1] as usize).min(input.len()-offset);
                if copy_to_user(tables,args[0],&input[offset..offset+count]).is_err(){return -14;}
                offset+=count;return count as i64;
            }
            if operation != 0x1002 || args[1] > 4096 { return -88; }
            let mut bytes = [0;4096];
            if copy_from_user(tables,args[0],&mut bytes[..args[1] as usize]).is_err() {return -14;}
            args[1] as i64
        }) {
            super::native_process::State::Runnable => continue,
            super::native_process::State::Exited(status) => status as u64,
            _ => panic!("unexpected linked-app trap"),
        };
        child.release(memory).unwrap();
        if INTERACTIVE {assert_eq!(offset,input.len());}
        return result;
    }
    panic!("guest-linked app exceeded execution budget")
}

// ------------------------=
// FUNC: run
// DESC: Exercises the production bounded compiler task against real user-mode Clang and LLD, not a fixture-specific syscall loop.
// ------------------=
pub unsafe fn run(memory: &mut NativeMemory, kernel: KernelAccessMap, controller: Controller) -> ! {
    // A declared test clock fixture, never an installed-kernel fallback date.
    super::system_clock::SYSTEM.observe(1_800_000_000, cpu::clock_ns().unwrap_or(0));
    #[cfg(feature = "python-pip-probe")]
    pip::run(memory, kernel, controller);
    let initial = memory.available_pages();
    let compile=super::native_compiler_task::COMPILE_ARGS;
    let link=super::native_compiler_task::LINK_ARGS;
    let version: &[&[u8]] = &[b"clang", b"--version"];
    let python: &[&[u8]] = &[b"/system/python", b"/script.py"];
    let validation: &[&[u8]] = &[b"/system/python", b"-c", python_workflow::VALIDATE_SOURCE, b"/script.py"];
    let mut task = super::native_compiler_task::Task::start(IMAGE, if PYTHON_VALIDATION { validation } else if PYTHON { python } else if LINK { link } else if COMPILE { compile } else { version },
        super::execution::SecurityIdentity([1;16]), 1, 1, memory, kernel, controller, u64::MAX).unwrap();
    let mut diagnostic = [0u8;65536];
    if PYTHON { task.set_launch_secret([0x5a;32]); }
    let mut output = [0u8;65536];
    let source: &[u8] = if option_env!("INFINITY_COMPILER_INVALID_SOURCE").is_some() {
        b"int main(void) { return ; this is not C; }\n"
    } else if INTERACTIVE {include_bytes!("../native-c-probe/interactive.c")} else { SOURCE };
    let inputs = [super::native_job_files::File { path: b"/script.py", bytes: if PYTHON_INVALID { b"if True\n    pass\n" } else if PYTHON_EXTENSION { EXTENSION_SOURCE } else { PYTHON_SOURCE } },
        super::native_job_files::File { path: b"/infinity_native_test.so", bytes: EXTENSION },
        super::native_job_files::File { path: b"/input.o", bytes: LINK_INPUT },
        super::native_job_files::File { path: b"/app.ld", bytes: LINK_SCRIPT },
        super::native_job_files::File { path: b"/crt.o", bytes: CRT },
        super::native_job_files::File { path: b"/sdk/stdio.h", bytes:super::native_compiler_task::STDIO },
        super::native_job_files::File { path: b"/sdk/stddef.h", bytes:super::native_compiler_task::STDDEF },
        super::native_job_files::File { path: b"/sdk/string.h", bytes:super::native_compiler_task::STRING },
        super::native_job_files::File { path: b"/sdk/stdlib.h", bytes:super::native_compiler_task::STDLIB },
        super::native_job_files::File { path: b"/sdk/stdint.h", bytes:super::native_compiler_task::STDINT },
        super::native_job_files::File { path: b"/sdk/stdbool.h", bytes:super::native_compiler_task::STDBOOL },
        super::native_job_files::File { path: b"/sdk/stdarg.h", bytes:super::native_compiler_task::STDARG },
        super::native_job_files::File { path: b"/sdk/limits.h", bytes:super::native_compiler_task::LIMITS },
        super::native_job_files::File { path: b"/sdk/errno.h", bytes:super::native_compiler_task::ERRNO },
        super::native_job_files::File { path: b"/sdk/ctype.h", bytes:super::native_compiler_task::CTYPE },
        super::native_job_files::File { path: b"/sdk/assert.h", bytes:super::native_compiler_task::ASSERT },
        super::native_job_files::File { path:b"/sdk/infinity/console.h",bytes:super::native_compiler_task::CONSOLE },
        super::native_job_files::File { path:b"/sdk/infinity/syscall.h",bytes:super::native_compiler_task::SYSCALL }];
    for visit in 0..1_000_000u64 {
        let result = task.poll(cpu::clock_ns().unwrap_or(0), true, memory, source,
            &inputs, &mut output, &mut diagnostic);
        let status = match result {
            super::native_process::State::Runnable => continue,
            super::native_process::State::Exited(status) => status as u64,
            super::native_process::State::Faulted(reason,address,pc) => {
                for value in [0xfa017u64,reason,address,pc] { emit(value); }
                finish();
            }
            _ => panic!("compiler task terminated unexpectedly"),
        };
        let (length, errors, truncated) = task.result_lengths(source,&mut output,&mut diagnostic).unwrap();
        task.release(memory).unwrap();
        let app_status = if LINK && status == 0 { execute_linked(&output[..length],memory,kernel,controller) } else { u64::MAX };
        for value in [0x434c414e47454c30,status,(length+errors) as u64,visit+1,memory.available_pages() as u64,initial as u64] { emit(value); }
        assert!(!truncated);
        for bytes in [&diagnostic[..errors], &output[..length]] {
            emit(bytes.len() as u64);
            for chunk in bytes.chunks(8) {
                let mut packed=[0;8]; packed[..chunk.len()].copy_from_slice(chunk);
                emit(u64::from_le_bytes(packed));
            }
        }
        emit(app_status);
        finish();
    }
    panic!("compiler exceeded disposable probe slice budget")
}
