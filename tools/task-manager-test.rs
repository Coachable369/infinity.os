#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "../kernel/ui/mod.rs"]
mod ui;

mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey {
        Character(u8),
        Backspace,
        Delete,
        Left,
        Right,
        Home,
        End,
        Enter,
    }
}

// ------------------------=
// FUNC: output_text
// DESC: Provides the kernel diagnostic sink required by the host runtime harness.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::execution::{ContextState, PriorityClass, ResourceBudget};
use runtime::service::SERVICE_RUNTIME;
use runtime::task_manager::{
    installed_image_bytes, TaskManagerError, FILE_NAVIGATOR_INSTALLED_BYTES,
    IMAGE_FILE_NAVIGATOR, IMAGE_TASK_MANAGER, IMAGE_TEXT_EDITOR,
};

// ------------------------=
// FUNC: main
// DESC: Verifies observable task monitoring, lifecycle, protection, and resource-control transitions.
// ------------------=
fn main() {
    let mut system = runtime::InfinityRuntime::new(false);
    system.define_bootstrap().unwrap();
    system.start_all(0);
    let services = system.task_manager.task_count(&system.execution);
    assert!(services > 0);
    let protected = system
        .services
        .inspect(SERVICE_RUNTIME)
        .unwrap()
        .context
        .unwrap();
    assert_eq!(
        system.task_manager.end(&mut system.execution, protected),
        Err(TaskManagerError::ProtectedSystemTask)
    );

    let files = system
        .task_manager
        .launch(&mut system.execution, IMAGE_FILE_NAVIGATOR)
        .unwrap();
    let editor = system
        .task_manager
        .launch(&mut system.execution, IMAGE_TEXT_EDITOR)
        .unwrap();
    let manager = system
        .task_manager
        .launch(&mut system.execution, IMAGE_TASK_MANAGER)
        .unwrap();
    assert_ne!(files, editor);
    assert_ne!(editor, manager);
    assert_eq!(
        system
            .task_manager
            .inspect(&system.execution, files)
            .unwrap()
            .state,
        ContextState::Runnable
    );

    system
        .task_manager
        .pause(&mut system.execution, files)
        .unwrap();
    assert_eq!(
        system
            .task_manager
            .inspect(&system.execution, files)
            .unwrap()
            .state,
        ContextState::Waiting
    );
    system
        .task_manager
        .resume(&mut system.execution, files)
        .unwrap();
    system.execution.account_memory(files, 4096).unwrap();
    system.execution.account_cpu_tick(files).unwrap();
    system.execution.account_cpu_tick(files).unwrap();
    system.task_manager.sample_cpu(&system.execution);
    let telemetry = system
        .task_manager
        .inspect(&system.execution, files)
        .unwrap();
    assert_eq!(telemetry.usage.memory_bytes, 4096);
    assert_eq!(telemetry.usage.cpu_ticks, 2);
    assert_eq!(telemetry.cpu_share_percent, 100);
    assert_eq!(telemetry.installed_bytes, FILE_NAVIGATOR_INSTALLED_BYTES);
    assert_eq!(installed_image_bytes(IMAGE_FILE_NAVIGATOR), telemetry.installed_bytes);
    system.execution.account_cpu_tick(files).unwrap();
    system.execution.account_cpu_tick(editor).unwrap();
    system.task_manager.sample_cpu(&system.execution);
    assert_eq!(
        system
            .task_manager
            .inspect(&system.execution, files)
            .unwrap()
            .cpu_share_percent,
        50
    );
    assert_eq!(
        system
            .task_manager
            .inspect(&system.execution, editor)
            .unwrap()
            .cpu_share_percent,
        50
    );
    let budget = ResourceBudget {
        memory_limit: 2 * 1024 * 1024,
        cpu_weight: 25,
        message_queue_limit: 8,
        io_priority: 1,
    };
    system
        .task_manager
        .throttle(
            &mut system.execution,
            files,
            budget,
            PriorityClass::Background,
        )
        .unwrap();
    let throttled = system
        .task_manager
        .inspect(&system.execution, files)
        .unwrap();
    assert_eq!(throttled.budget, budget);
    assert_eq!(throttled.priority, PriorityClass::Background);
    let old_identity = system.execution.get(files).unwrap().security_identity;
    system
        .task_manager
        .end(&mut system.execution, files)
        .unwrap();
    assert_eq!(
        system.execution.get(files).unwrap().state,
        ContextState::Stopped
    );
    system
        .task_manager
        .relaunch(&mut system.execution, files)
        .unwrap();
    assert_eq!(
        system.execution.get(files).unwrap().state,
        ContextState::Runnable
    );
    assert_ne!(
        system.execution.get(files).unwrap().security_identity,
        old_identity
    );
    assert_eq!(system.execution.get(files).unwrap().usage.memory_bytes, 0);
    assert_eq!(
        system.task_manager.task_count(&system.execution),
        services + 3
    );
    println!("PASS Task Manager: authoritative monitoring, app launch/end/relaunch, pause/resume, throttling, priority, and system-task protection");
}
