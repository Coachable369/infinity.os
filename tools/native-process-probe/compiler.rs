use super::{native_memory::*, native_c_image::Image, cpu::{self, Tables, Controller, Context, MACHINE}, emit, finish, IMAGE};
#[cfg(feature = "python-pip-probe")]
#[path = "pip.rs"] mod pip;
const COMPILE: bool = option_env!("INFINITY_COMPILER_SOURCE").is_some();
const PYTHON: bool = option_env!("INFINITY_PYTHON_PROBE").is_some();
const PYTHON_CLOCK_PROBE: bool = option_env!("INFINITY_NATIVE_CLOCK_PROBE").is_some();
const PYTHON_EXTENSION: bool = option_env!("INFINITY_PYTHON_EXTENSION_PROBE").is_some();
const PYTHON_VALIDATION: bool = option_env!("INFINITY_PYTHON_VALIDATION_PROBE").is_some();
const PYTHON_GENERATED_VALIDATION: bool = option_env!("INFINITY_PYTHON_GENERATED_VALIDATION_PROBE").is_some();
const PYTHON_INVALID: bool = option_env!("INFINITY_PYTHON_INVALID_PROBE").is_some();
const PYTHON_RUNTIME_EXCEPTION: bool = option_env!("INFINITY_PYTHON_RUNTIME_EXCEPTION_PROBE").is_some();
#[path = "../../kernel/runtime/native_python_workflow.rs"] mod python_workflow;
#[path = "../../kernel/runtime/native_python_exception.rs"] mod python_exception;
static EXTENSION: &[u8] = include_bytes!(env!("INFINITY_PYTHON_EXTENSION_BYTES"));
const EXTENSION_SOURCE: &[u8] = b"import sys\nimport infinity_native_test as native\nassert native.add(-5, 8) == 3\ntry:\n    native.add(9223372036854775807, 1)\nexcept OverflowError:\n    pass\nelse:\n    raise AssertionError('overflow was not rejected')\nprint(native.add(19, 35))\nsys.exit(native.add(19, 35))\n";
const PYTHON_SOURCE: &[u8] = include_bytes!(env!("INFINITY_PYTHON_SOURCE_BYTES"));
const INTERACTIVE:bool=option_env!("INFINITY_INTERACTIVE_APP").is_some();
const SOURCE: &[u8] = b"#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <ctype.h>\nstatic int compare(const void *a,const void *b){return *(const int*)a-*(const int*)b;}\nint main(void){int *v=calloc(4,sizeof *v);if(!v)return 90;v[0]=7;v[1]=1;v[2]=5;v[3]=3;qsort(v,4,sizeof *v,compare);int key=5;int *found=bsearch(&key,v,4,sizeof *v,compare);char out[32];int n=snprintf(out,sizeof out,\"%s:%d\",\"native\",found?*found:0);int ok=n==8&&!strcmp(out,\"native:5\")&&toupper('c')=='C'&&puts(out)>=0&&fprintf(stderr,\"%d\\n\",54)==3;free(v);return ok?54:91;}\n";
const LINK: bool = option_env!("INFINITY_LINK_INPUT").is_some();
static LINK_INPUT: &[u8] = include_bytes!(env!("INFINITY_LINK_INPUT_BYTES"));
static LINK_SCRIPT: &[u8] = include_bytes!("../../sdk/c/application.ld");
static CRT: &[u8] = include_bytes!(env!("INFINITY_APPLICATION_CRT"));

#[cfg(feature = "compiler-clock-probe")]
unsafe extern "C" { static INFINITY_PROBE_CLOCK_IDS: [u64; 2]; }

#[cfg(feature = "compiler-clock-probe")]
// ------------------------=
// FUNC: clock_contract
// DESC: Executes clock selection with real C-header IDs, verifying corrections affect UTC but never monotonic time.
// ------------------=
unsafe fn clock_contract() {
    use super::system_clock::{Settings, SystemClock};
    let [realtime, monotonic] = INFINITY_PROBE_CLOCK_IDS;
    let clock = SystemClock::new();
    assert_eq!(clock.native_nanoseconds(realtime, 500), Err(-38));
    assert_eq!(clock.native_nanoseconds(monotonic, 500), Ok(500));
    clock.observe(1_800_000_000, 100);
    assert_eq!(clock.native_nanoseconds(realtime, 500), Ok(1_800_000_000_000_000_400));
    assert!(clock.configure(Settings { offset_minutes: -360, correction_seconds: 60 }));
    assert_eq!(clock.native_nanoseconds(realtime, 500), Ok(1_800_000_060_000_000_400));
    assert_eq!(clock.native_nanoseconds(monotonic, 500), Ok(500));
    assert_eq!(clock.native_nanoseconds(u64::MAX, 500), Err(-22));
}

// ------------------------=
// FUNC: execute_linked
// DESC: Loads the guest-linked artifact into a new private process and checks its real exit value.
// ------------------=
unsafe fn execute_linked(bytes: &[u8], memory: &mut NativeMemory, kernel: KernelAccessMap, controller: Controller) -> u64 {
    // The real SDK contains thread-local errno even when its initialized TLS is
    // empty. A non-TLS orphan must not collapse that zero-filled template.
    assert!(Image::parse(bytes,cpu::MACHINE).unwrap().tls.is_some_and(|tls|tls.memory_size>=4));
    let mut child = super::native_app_task::Task::start(bytes,super::execution::SecurityIdentity([2;16]),
        2,memory,kernel,controller,u64::MAX).unwrap();
    let mut offset=0;
    let mut written=[0usize;2];
    let input=b"2 + 3\n9 * 6\nquit\n";
    for _ in 0..1000 {
        let result = match child.poll(0,true, |_, tables, operation, args| {
            if operation==0x1003 && INTERACTIVE {
                let count=(args[1] as usize).min(input.len()-offset);
                if copy_to_user(tables,args[0],&input[offset..offset+count]).is_err(){return -14;}
                offset+=count;return count as i64;
            }
            if operation != 0x1002 || args[1] > 4096 || args[2] > 2 { return -88; }
            let mut bytes = [0;4096];
            if copy_from_user(tables,args[0],&mut bytes[..args[1] as usize]).is_err() {return -14;}
            written[usize::from(args[2]==2)]+=args[1] as usize;
            args[1] as i64
        }) {
            super::native_process::State::Runnable => continue,
            super::native_process::State::Exited(status) => status as u64,
            unexpected => {
                let (state, reason, address, pc) = match unexpected {
                    super::native_process::State::Faulted(reason, address, pc) => (1, reason, address, pc),
                    super::native_process::State::Cancelled => (2, 0, 0, 0),
                    super::native_process::State::Deadline => (3, 0, 0, 0),
                    super::native_process::State::Budget => (4, 0, 0, 0),
                    super::native_process::State::PlatformFailure => (5, 0, 0, 0),
                    _ => (0, 0, 0, 0),
                };
                for value in [0x4c494e4b4641494cu64, state, reason, address, pc,
                    written[0] as u64, written[1] as u64, bytes.len() as u64] { emit(value); }
                for chunk in bytes.chunks(8) {
                    let mut packed = [0; 8]; packed[..chunk.len()].copy_from_slice(chunk);
                    emit(u64::from_le_bytes(packed));
                }
                finish();
            }
        };
        child.release(memory).unwrap();
        if INTERACTIVE {assert_eq!(offset,input.len());}
        else {assert_eq!(written,[9,3]);}
        return result;
    }
    panic!("guest-linked app exceeded execution budget")
}

// ------------------------=
// FUNC: run
// DESC: Exercises the production bounded compiler task against real user-mode Clang and LLD, not a fixture-specific syscall loop.
// ------------------=
pub unsafe fn run(memory: &mut NativeMemory, kernel: KernelAccessMap, controller: Controller) -> ! {
    #[cfg(feature = "compiler-clock-probe")]
    clock_contract();
    // A declared test clock fixture, never an installed-kernel fallback date.
    super::system_clock::SYSTEM.observe(1_800_000_000, cpu::clock_ns().unwrap_or(0));
    #[cfg(feature = "python-pip-probe")]
    pip::run(memory, kernel, controller);
    let initial = memory.available_pages();
    let compile=super::native_compiler_task::COMPILE_ARGS;
    let link=super::native_compiler_task::LINK_ARGS;
    let version: &[&[u8]] = &[b"clang", b"--version"];
    let python: &[&[u8]] = &[b"/system/python", b"/script.py"];
    let generated_python: &[&[u8]] = &[b"/system/python", b"-m", b"_infinity_generated_run", b"/script.py"];
    let validation: &[&[u8]] = &[b"/system/python", b"-c",
        if PYTHON_GENERATED_VALIDATION { python_workflow::VALIDATE_GENERATED_APPLICATION }
        else { python_workflow::VALIDATE_SOURCE }, b"/script.py"];
    let mut task = super::native_compiler_task::Task::start(IMAGE, if PYTHON_VALIDATION { validation } else if PYTHON_RUNTIME_EXCEPTION { generated_python } else if PYTHON { python } else if LINK { link } else if COMPILE { compile } else { version },
        super::execution::SecurityIdentity([1;16]), 1, 1, memory, kernel, controller, u64::MAX).unwrap();
    let mut diagnostic = [0u8;65536];
    if PYTHON { task.set_launch_secret([0x5a;32]); }
    let mut output = [0u8;65536];
    let source: &[u8] = if option_env!("INFINITY_COMPILER_INVALID_SOURCE").is_some() {
        b"int main(void) { return ; this is not C; }\n"
    } else if INTERACTIVE {include_bytes!("../native-c-probe/interactive.c")} else { SOURCE };
    let inputs = [super::native_job_files::File { path: b"/script.py", bytes: if PYTHON_CLOCK_PROBE { include_bytes!("clock.py") } else if PYTHON_INVALID { b"if True\n    pass\n" } else if PYTHON_EXTENSION && option_env!("INFINITY_PYTHON_CUSTOM_SOURCE").is_none() { EXTENSION_SOURCE } else { PYTHON_SOURCE } },
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
    let mut exception: Option<python_exception::Evidence> = None;
    for visit in 0..1_000_000u64 {
        let result = task.poll_with_services(cpu::clock_ns().unwrap_or(0), true, memory, source,
            &inputs, &mut output, &mut diagnostic, |_, tables, operation, arguments| {
                if operation != python_exception::OPERATION { return None; }
                if !PYTHON_RUNTIME_EXCEPTION { return Some(-13); }
                if arguments[1] != python_exception::RECORD_BYTES as u64
                    || arguments[2..].iter().any(|value| *value != 0) { return Some(-22); }
                if exception.is_some() { return Some(-16); }
                let mut record = [0; python_exception::RECORD_BYTES];
                if copy_from_user(tables, arguments[0], &mut record).is_err() { return Some(-14); }
                let Some(evidence) = python_exception::decode(&record, b"/script.py", inputs[0].bytes) else { return Some(-22); };
                exception = Some(evidence);
                Some(0)
            });
        let status = match result {
            super::native_process::State::Runnable => continue,
            super::native_process::State::Exited(status) => status as u64,
            super::native_process::State::Faulted(reason,address,pc) => {
                for value in [0xfa017u64,reason,address,pc] { emit(value); }
                finish();
            }
            unexpected => {
                let state = match unexpected {
                    super::native_process::State::Cancelled => 1,
                    super::native_process::State::Deadline => 2,
                    super::native_process::State::Budget => 3,
                    super::native_process::State::PlatformFailure => 4,
                    _ => 0,
                };
                for value in [0xbad57a7eu64, state, visit] { emit(value); }
                finish();
            }
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
        if PYTHON_RUNTIME_EXCEPTION {
            for value in [0x5059455843455031, u64::from(exception.is_some()),
                exception.map_or(0, |e| e.category as u64),
                exception.map_or(0, |e| e.class as u64),
                exception.map_or(0, |e| e.line as u64)] { emit(value); }
        }
        finish();
    }
    panic!("compiler exceeded disposable probe slice budget")
}
