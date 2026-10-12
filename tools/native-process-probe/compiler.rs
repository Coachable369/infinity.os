use super::{native_memory::*, native_c_image::Image, cpu::{self, Tables, Controller, Context, MACHINE}, emit, finish, IMAGE};
const COMPILE: bool = option_env!("INFINITY_COMPILER_SOURCE").is_some();
const SOURCE: &[u8] = b"#include <stdio.h>\nint answer(int x) { return x * 7 + 5; }\nint main(void) { if (puts(\"native C\") < 0) return 99; return answer(7); }\n";
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
    for _ in 0..1000 {
        let result = match child.poll(0,true, |_, tables, operation, args| {
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
        return result;
    }
    panic!("guest-linked app exceeded execution budget")
}

// ------------------------=
// FUNC: run
// DESC: Exercises the production bounded compiler task against real user-mode Clang and LLD, not a fixture-specific syscall loop.
// ------------------=
pub unsafe fn run(memory: &mut NativeMemory, kernel: KernelAccessMap, controller: Controller) -> ! {
    let initial = memory.available_pages();
    let compile=super::native_compiler_task::COMPILE_ARGS;
    let link=super::native_compiler_task::LINK_ARGS;
    let version: &[&[u8]] = &[b"clang", b"--version"];
    let mut task = super::native_compiler_task::Task::start(IMAGE, if LINK { link } else if COMPILE { compile } else { version },
        super::execution::SecurityIdentity([1;16]), 1, 1, memory, kernel, controller, u64::MAX).unwrap();
    let mut diagnostic = [0u8;65536];
    let mut output = [0u8;65536];
    let source: &[u8] = if option_env!("INFINITY_COMPILER_INVALID_SOURCE").is_some() {
        b"int main(void) { return ; this is not C; }\n"
    } else { SOURCE };
    let inputs = [super::native_job_files::File { path: b"/input.o", bytes: LINK_INPUT },
        super::native_job_files::File { path: b"/app.ld", bytes: LINK_SCRIPT },
        super::native_job_files::File { path: b"/crt.o", bytes: CRT },
        super::native_job_files::File { path: b"/sdk/stdio.h", bytes:super::native_compiler_task::STDIO },
        super::native_job_files::File { path: b"/sdk/stddef.h", bytes:super::native_compiler_task::STDDEF }];
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
