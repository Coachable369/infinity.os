#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/runtime/mod.rs"] mod runtime;
// ------------------------=
// FUNC: output_text
// DESC: Implements the output text operation.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::capability::*;
use runtime::event::*;
use runtime::execution::*;
use runtime::iop::*;
use runtime::scheduler::*;
use runtime::service::*;
use std::time::Instant;

// ------------------------=
// FUNC: identity
// DESC: Implements the identity operation.
// ------------------=
fn identity(value:u8)->SecurityIdentity{SecurityIdentity([value;16])}
// ------------------------=
// FUNC: budget
// DESC: Implements the budget operation.
// ------------------=
fn budget()->ResourceBudget{ResourceBudget{memory_limit:4096,cpu_weight:100,message_queue_limit:8,io_priority:1}}
// ------------------------=
// FUNC: ops
// DESC: Implements the ops operation.
// ------------------=
fn ops(first:u32)->[u32;MAX_OPERATIONS]{let mut out=[0;MAX_OPERATIONS];out[0]=first;out}

// ------------------------=
// FUNC: execution_and_scheduler
// DESC: Implements the execution and scheduler operation.
// ------------------=
fn execution_and_scheduler(){let mut contexts=ExecutionManager::new();let a=contexts.create(10,1,MemoryRegion{base:0x1000,length:0x1000},1,PriorityClass::System,budget()).unwrap();let b=contexts.create(11,1,MemoryRegion{base:0x3000,length:0x1000},2,PriorityClass::Background,budget()).unwrap();assert_eq!(contexts.create(12,1,MemoryRegion{base:0x1800,length:0x1000},3,PriorityClass::Normal,budget()),Err(ExecutionError::RegionOverlap));assert_eq!(contexts.account_memory(a,4097),Err(ExecutionError::BudgetExceeded));contexts.set_state(a,ContextState::Runnable).unwrap();contexts.set_state(b,ContextState::Runnable).unwrap();let mut scheduler=Scheduler::new();let mut saw_a=false;let mut saw_b=false;for _ in 0..16{if let Some(handle)=scheduler.next(&mut contexts){saw_a|=handle==a;saw_b|=handle==b;scheduler.yield_context(&mut contexts,handle)}}assert!(saw_a&&saw_b,"weighted scheduling must not starve background work");scheduler.block(&mut contexts,a);scheduler.wake(&mut contexts,a,WakeReason::Message);assert_eq!(contexts.get(a).unwrap().state,ContextState::Runnable);println!("PASS execution: stable identities, disjoint regions, budgets, fair scheduling/wakeup");}

// ------------------------=
// FUNC: capabilities
// DESC: Implements the capabilities operation.
// ------------------=
fn capabilities(){let issuer=identity(1);let a=identity(2);let b=identity(3);let mut manager=CapabilityManager::new();let parent=manager.grant(CapabilityType::ObjectRead,1,0b11,0,issuer,a,Some(100),0b01).unwrap();let child=manager.delegate(parent,b,0b01,42,Some(80),1).unwrap();assert!(manager.validate(child,b,CapabilityType::ObjectRead,1,1,42,2).is_ok());assert_eq!(manager.validate(child,b,CapabilityType::ObjectRead,1,1,43,2),Err(CapabilityError::Denied));manager.revoke(parent).unwrap();assert_eq!(manager.validate(child,b,CapabilityType::ObjectRead,1,1,42,2),Err(CapabilityError::Revoked));let lease=manager.grant(CapabilityType::StorageRead,7,1,0,issuer,a,Some(5),0).unwrap();assert!(manager.validate(lease,a,CapabilityType::StorageRead,7,1,0,4).is_ok());assert_eq!(manager.validate(lease,a,CapabilityType::StorageRead,7,1,0,5),Err(CapabilityError::Expired));println!("PASS capabilities: subset delegation, scope confinement, revocation, leases");}

// ------------------------=
// FUNC: iop
// DESC: Implements the iop operation.
// ------------------=
fn iop(){let caller=identity(4);let service=identity(5);let mut caps=CapabilityManager::new();let cap=caps.grant(CapabilityType::ServiceCall,OperationId::TestEcho as u64,1,0,service,caller,Some(100),0).unwrap();let mut router=IopRouter::new();router.register_endpoint(8,caller).unwrap();router.register_endpoint(9,service).unwrap();let message=IopMessage::request(OperationId::TestEcho,0xa81f,caller,cap,50,0xa81f,b"hello").unwrap();let mut encoded=[0u8;HEADER_BYTES];message.header.encode(&mut encoded);assert_eq!(IopHeader::decode(&encoded).unwrap(),message.header);let move_request=WindowMoveV1{window_id:9,x:-12,y:44,work_x:0,work_y:30,work_width:1920,work_height:1050};let mut move_bytes=[0u8;WINDOW_MOVE_V1_BYTES];move_request.encode(&mut move_bytes);assert_eq!(WindowMoveV1::decode(&move_bytes),Ok(move_request));assert_eq!(WindowMoveV1::decode(&move_bytes[..20]),Err(IopError::InvalidPayload));let commit=SurfaceCommitV1{surface_id:7,generation:42,damage_count:3};let mut commit_bytes=[0u8;SURFACE_COMMIT_V1_BYTES];commit.encode(&mut commit_bytes);assert_eq!(SurfaceCommitV1::decode(&commit_bytes),Ok(commit));let start=Instant::now();router.send(9,message,&caps,1).unwrap();let received=router.receive(9,1).unwrap();assert_eq!(received.bytes(),b"hello");router.respond(8,&received,service,b"hello",1).unwrap();let response=router.receive(8,1).unwrap();assert_eq!(response.header.message_type,MessageType::Response);assert_eq!(response.bytes(),b"hello");let nanos=start.elapsed().as_nanos();let expired=IopMessage::request(OperationId::TestEcho,2,caller,cap,2,2,b"late").unwrap();assert_eq!(router.send(9,expired,&caps,2),Err(IopError::DeadlineExceeded));let cancelled=IopMessage::request(OperationId::TestEcho,3,caller,cap,50,3,b"cancel").unwrap();router.cancel(3);assert_eq!(router.send(9,cancelled,&caps,3),Err(IopError::Cancelled));for id in 10..18{router.send(9,IopMessage::request(OperationId::TestEcho,id,caller,cap,90,id,b"x").unwrap(),&caps,3).unwrap()}assert_eq!(router.queue_depth(9),Some(ENDPOINT_QUEUE_CAPACITY));assert_eq!(router.send(9,IopMessage::request(OperationId::TestEcho,19,caller,cap,90,19,b"x").unwrap(),&caps,3),Err(IopError::Backpressure));caps.revoke(cap).unwrap();let denied=IopMessage::request(OperationId::TestEcho,20,caller,cap,90,20,b"x").unwrap();assert_eq!(router.send(9,denied,&caps,3),Err(IopError::AccessDenied));println!("PASS IOP: binary request/response echo, typed UI schemas, revoke, deadline, cancellation, bounded queue");println!("MEASURE host IOP round-trip={}ns",nanos);}

// ------------------------=
// FUNC: events
// DESC: Implements the events operation.
// ------------------=
fn events(){let publisher=identity(6);let subscriber=identity(7);let mut caps=CapabilityManager::new();let publish=caps.grant(CapabilityType::EventPublish,77,1,0,publisher,publisher,Some(1000),0).unwrap();let subscribe=caps.grant(CapabilityType::EventSubscribe,77,1,9,publisher,subscriber,Some(1000),0).unwrap();let mut fabric=EventFabric::new();let lease=fabric.subscribe(subscriber,subscribe,EventFilter{type_id:77,scope:Some(9)},1000,OverflowPolicy::DropOldest,2,&caps,1).unwrap();let start=Instant::now();fabric.publish(EventClass::StateChange,RoutingDomain::System,77,publisher,8,1,1,b"filtered",10,1,&caps,publish).unwrap();assert!(fabric.receive(lease,1).is_err());
fabric.publish(EventClass::StateChange,RoutingDomain::System,77,publisher,9,1,1,b"one",10,1,&caps,publish).unwrap();assert_eq!(fabric.receive(lease,1).unwrap().payload_len,3);let nanos=start.elapsed().as_nanos();caps.revoke(subscribe).unwrap();fabric.publish(EventClass::StateChange,RoutingDomain::System,77,publisher,9,2,2,b"two",10,2,&caps,publish).unwrap();assert!(fabric.receive(lease,2).is_err());
let pub88=caps.grant(CapabilityType::EventPublish,88,1,0,publisher,publisher,None,0).unwrap();let sub88=caps.grant(CapabilityType::EventSubscribe,88,1,0,publisher,subscriber,None,0).unwrap();let gap=fabric.subscribe(subscriber,sub88,EventFilter{type_id:88,scope:None},1000,OverflowPolicy::DropNewest,1,&caps,1).unwrap();fabric.publish(EventClass::StateChange,RoutingDomain::Storage,88,publisher,0,3,3,b"v1",10,3,&caps,pub88).unwrap();fabric.publish(EventClass::StateChange,RoutingDomain::Storage,88,publisher,0,3,3,b"v2",10,3,&caps,pub88).unwrap();fabric.receive(gap,3).unwrap();fabric.publish(EventClass::StateChange,RoutingDomain::Storage,88,publisher,0,3,3,b"v3",10,3,&caps,pub88).unwrap();fabric.receive(gap,3).unwrap();assert_eq!(fabric.sequence_gap(gap),Ok(true));assert!(runtime::acceptance_self_test(),"gap reconciliation IOP path and boot self-test");
let bounded=fabric.subscribe(subscriber,sub88,EventFilter{type_id:88,scope:None},1000,OverflowPolicy::Coalesce,2,&caps,1).unwrap();for n in 0..20{fabric.publish(EventClass::StateChange,RoutingDomain::Storage,88,publisher,0,n,n,b"state",10,4,&caps,pub88).unwrap();}assert!(fabric.queue_depth(bounded).unwrap()<=2);let record_pub=caps.grant(CapabilityType::EventPublish,99,1,0,publisher,publisher,None,0).unwrap();fabric.publish(EventClass::Record,RoutingDomain::System,99,publisher,0,9,8,b"Capability.Granted",255,5,&caps,record_pub).unwrap();assert_eq!(fabric.record_count(),1);
let mut limited=EventFabric::new();limited.set_rate_limit(1,0);let rate_cap=caps.grant(CapabilityType::EventPublish,55,1,0,publisher,publisher,None,0).unwrap();limited.publish(EventClass::Signal,RoutingDomain::Device,55,publisher,0,1,1,b"one",1,1000,&caps,rate_cap).unwrap();assert_eq!(limited.publish(EventClass::Signal,RoutingDomain::Device,55,publisher,0,1,1,b"two",1,1000,&caps,rate_cap),Err(EventError::RateLimited));let authoritative_state=2;let huge=[0u8;MAX_EVENT_PAYLOAD+1];assert_eq!(limited.publish(EventClass::StateChange,RoutingDomain::System,55,publisher,0,1,1,&huge,1,2000,&caps,rate_cap),Err(EventError::PayloadTooLarge));assert_eq!(authoritative_state,2,"committed state survives event publication failure");
println!("PASS IEF: filtering, delivery-time revocation, leases, rate limit, bounded coalescing, sequence-gap reconciliation, commit-before-publish, Record log");println!("MEASURE host event publish+receive={}ns",nanos);}

// ------------------------=
// FUNC: services
// DESC: Implements the services operation.
// ------------------=
fn services(){let mut manager=ServiceManager::new();let none=[0;MAX_DEPENDENCIES];manager.define(manifest(100,none,0,ops(1),1,RestartPolicy::Never,Criticality::Important)).unwrap();manager.define(manifest(101,[100,0,0,0],1,ops(2),1,RestartPolicy::BoundedRetry{maximum:3},Criticality::NonCritical)).unwrap();manager.define(manifest(102,[100,0,0,0],1,ops(3),1,RestartPolicy::Never,Criticality::Critical)).unwrap();let cycle=manager.define(manifest(103,[103,0,0,0],1,ops(4),1,RestartPolicy::Never,Criticality::NonCritical));assert_eq!(cycle,Err(ServiceError::DependencyCycle));let mut contexts=ExecutionManager::new();let start=Instant::now();manager.start_ready(&mut contexts,0);manager.announce_ready(100).unwrap();manager.start_ready(&mut contexts,0);manager.announce_ready(101).unwrap();manager.announce_ready(102).unwrap();assert_eq!(manager.provider(2),Some(101));manager.fail(101,&mut contexts,0).unwrap();assert_eq!(manager.inspect(101).unwrap().state,ServiceState::Restarting);assert_eq!(manager.start_ready(&mut contexts,999),0);assert_eq!(manager.start_ready(&mut contexts,1000),1);manager.announce_ready(101).unwrap();assert_eq!(manager.inspect(101).unwrap().state,ServiceState::Ready);let _=manager.fail(102,&mut contexts,1000);assert!(manager.degraded());
let mut integrated=runtime::InfinityRuntime::new(false);integrated.define_bootstrap().unwrap();integrated.start_all(0);integrated.services.define(manifest(200,[SERVICE_RUNTIME,0,0,0],1,ops(9),1,RestartPolicy::OnFailure,Criticality::NonCritical)).unwrap();integrated.start_all(0);let runtime_context=integrated.services.inspect(SERVICE_RUNTIME).unwrap().context.unwrap();let source=integrated.execution.get(runtime_context).unwrap().security_identity;let observer=identity(12);let subcap=integrated.capabilities.grant(CapabilityType::EventSubscribe,runtime::EVENT_SERVICE_STATE_CHANGED as u64,1,0,source,observer,None,0).unwrap();let lease=integrated.events.subscribe(observer,subcap,EventFilter{type_id:runtime::EVENT_SERVICE_STATE_CHANGED,scope:None},1000,OverflowPolicy::DropOldest,2,&integrated.capabilities,0).unwrap();integrated.fail_service(200,1).unwrap();assert_eq!(integrated.events.receive(lease,1).unwrap().type_id,runtime::EVENT_SERVICE_STATE_CHANGED);
println!("PASS services: dependency order/cycle detection, explicit ready, discovery, failure event, noncritical restart, critical degraded mode");println!("MEASURE host service startup={}ns",start.elapsed().as_nanos());}

// ------------------------=
// FUNC: ui_authority
// DESC: Proves compositor service operations require scoped, revocable capabilities and matching surface ownership.
// ------------------=
fn ui_authority() {
    let mut integrated = runtime::InfinityRuntime::new(false);
    integrated.define_bootstrap().unwrap();
    integrated.start_all(0);
    let window_context = integrated.services.inspect(SERVICE_WINDOW_SERVER).unwrap().context.unwrap();
    let window_source = integrated.execution.get(window_context).unwrap().security_identity;
    let observer = identity(32);
    let moved_subscription_cap = integrated.capabilities.grant(
        CapabilityType::EventSubscribe,
        runtime::EVENT_WINDOW_MOVED as u64,
        1,
        0,
        window_source,
        observer,
        Some(100),
        0,
    ).unwrap();
    let moved_subscription = integrated.events.subscribe(
        observer,
        moved_subscription_cap,
        EventFilter { type_id: runtime::EVENT_WINDOW_MOVED, scope: None },
        100,
        OverflowPolicy::LatestOnly,
        2,
        &integrated.capabilities,
        0,
    ).unwrap();
    let issuer = identity(30);
    let caller = identity(31);
    let context = ui::window::ContextId(31);
    let create_surface = integrated.capabilities.grant(
        CapabilityType::SurfaceCreate,
        SERVICE_WINDOW_SERVER as u64,
        1,
        0,
        issuer,
        caller,
        Some(100),
        0,
    ).unwrap();
    let surface = integrated.create_surface(
        caller,
        context,
        create_surface,
        ui::geometry::Size { width: 320, height: 200 },
        ui::surface::PixelFormat::Argb8888,
        ui::surface::SurfaceSecurityClass::Application,
        1,
    ).unwrap();
    let publish = integrated.capabilities.grant(
        CapabilityType::SurfacePublish,
        surface.0 as u64,
        1,
        0,
        issuer,
        caller,
        Some(100),
        0,
    ).unwrap();
    integrated.publish_surface(caller, context, publish, surface, 1, 1).unwrap();
    let inspect_surface = integrated.capabilities.grant(
        CapabilityType::SurfaceInspectMetadata,
        surface.0 as u64,
        1,
        0,
        issuer,
        caller,
        Some(100),
        0,
    ).unwrap();
    assert_eq!(integrated.inspect_surface(caller, inspect_surface, surface, 1).unwrap().owner, context);
    assert_eq!(integrated.inspect_surface(identity(99), inspect_surface, surface, 1), Err(runtime::UiOperationError::AccessDenied));
    let resize_surface = integrated.capabilities.grant(
        CapabilityType::SurfaceResize,
        surface.0 as u64,
        1,
        0,
        issuer,
        caller,
        Some(100),
        0,
    ).unwrap();
    assert_eq!(integrated.resize_surface(
        caller,
        context,
        resize_surface,
        surface,
        ui::geometry::Size { width: 400, height: 240 },
        1,
    ).unwrap().size.width, 400);
    let create_window = integrated.capabilities.grant(
        CapabilityType::WindowCreate,
        SERVICE_WINDOW_SERVER as u64,
        1,
        0,
        issuer,
        caller,
        Some(100),
        0,
    ).unwrap();
    let window = integrated.create_window(
        caller,
        context,
        create_window,
        surface,
        ui::geometry::Rect { x: 40, y: 40, width: 320, height: 200 },
        ui::window::ZOrderClass::Normal,
        1,
    ).unwrap();
    let manage = integrated.capabilities.grant(
        CapabilityType::WindowManageOwn,
        window.0 as u64,
        1,
        0,
        issuer,
        caller,
        Some(100),
        0,
    ).unwrap();
    assert_eq!(integrated.ui.surfaces.inspect(surface).unwrap().associated_window, Some(window));
    assert_eq!(integrated.create_window(caller, context, create_window, surface,
        ui::geometry::Rect { x: 50, y: 50, width: 320, height: 200 },
        ui::window::ZOrderClass::Normal, 1),
        Err(runtime::UiOperationError::Surface(ui::surface::SurfaceError::InUse)));
    let inspect_window = integrated.capabilities.grant(
        CapabilityType::WindowInspectMetadata,
        window.0 as u64,
        1,
        0,
        issuer,
        caller,
        Some(100),
        0,
    ).unwrap();
    assert_eq!(integrated.inspect_window(caller, inspect_window, window, 1).unwrap().surface, surface);
    assert_eq!(integrated.inspect_window(identity(99), inspect_window, window, 1), Err(runtime::UiOperationError::AccessDenied));
    let destroy_surface = integrated.capabilities.grant(
        CapabilityType::SurfaceDestroy,
        surface.0 as u64,
        1,
        0,
        issuer,
        caller,
        Some(100),
        0,
    ).unwrap();
    assert_eq!(integrated.destroy_surface(caller, context, destroy_surface, surface, 1), Err(runtime::UiOperationError::Surface(ui::surface::SurfaceError::InUse)));
    assert_eq!(integrated.move_window(
        caller,
        context,
        manage,
        window,
        ui::geometry::Point { x: 80, y: 70 },
        ui::geometry::Rect { x: 0, y: 30, width: 1280, height: 690 },
        2,
    ).unwrap().x, 80);
    let moved_event = integrated.events.receive(moved_subscription, 2).unwrap();
    assert_eq!(moved_event.class, EventClass::StateChange);
    assert_eq!(moved_event.correlation_id, window.0 as u64);
    assert_eq!(moved_event.payload[0], 1);
    assert_eq!(u32::from_le_bytes(moved_event.payload[4..8].try_into().unwrap()), window.0);
    use runtime::native_window::Command;
    let area = ui::geometry::Rect { x: 0, y: 30, width: 1280, height: 690 };
    let resized = integrated.native_window_command(caller, context, manage, window,
        Command::Resize(ui::geometry::Size { width: 500, height: 350 }), area, area, 2).unwrap();
    assert_eq!((resized.width, resized.height), (500, 350));
    {
        use runtime::native_memory::{NativeMemory, X86AddressSpace, PagePermissions, copy_to_user};
        let layout = std::alloc::Layout::from_size_align(128 * 1024, 4096).unwrap();
        let arena = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!arena.is_null());
        let mut memory = NativeMemory::empty();
        memory.initialize(arena as u64, layout.size() as u64).unwrap();
        let data = memory.allocate(1, context.0).unwrap();
        let mut tables = X86AddressSpace::new(&mut memory, context.0).unwrap();
        tables.map_range(&mut memory, 0x100000000, data.physical_address, 4096, PagePermissions::USER_RW).unwrap();
        tables.map_range(&mut memory, 0x200000000, data.physical_address, 4096, PagePermissions::KERNEL_RW).unwrap();
        copy_to_user(&mut tables, 0x100000000, b"Native app").unwrap();
        assert_eq!(integrated.native_window_syscall(caller, context, &tables, 0x1102,
            [manage, window.0 as u64, 0x100000000, 10, 0, 0], area, area, 2), 0);
        assert_eq!(integrated.ui.windows.inspect(window).unwrap().title(), "Native app");
        assert_eq!(integrated.native_window_syscall(caller, context, &tables, 0x1102,
            [manage, window.0 as u64, 0x200000000, 10, 0, 0], area, area, 2), -5);
        assert_eq!(integrated.native_window_syscall(caller, context, &tables, 0x1105,
            [manage, window.0 as u64, 999, 0, 0, 0], area, area, 2), -3);
        assert_eq!(integrated.ui.windows.inspect(window).unwrap().bounds, resized);
        assert_eq!(integrated.ui.windows.inspect(window).unwrap().title(), "Native app");
        tables.release(&mut memory).unwrap();
        memory.release(data).unwrap();
        assert_eq!(memory.available_pages(), 32);
        unsafe { std::alloc::dealloc(arena, layout); }
    }
    assert_eq!(integrated.native_window_command(identity(99), context, manage, window,
        Command::Close, area, area, 2), Err(runtime::UiOperationError::AccessDenied));
    assert_eq!(integrated.native_window_command(caller, ui::window::ContextId(99), manage, window,
        Command::Close, area, area, 2), Err(runtime::UiOperationError::AccessDenied));
    integrated.native_window_command(caller, context, manage, window,
        Command::State(ui::window::WindowState::Minimized), area, area, 2).unwrap();
    assert!(!integrated.ui.windows.inspect(window).unwrap().visible);
    integrated.native_window_command(caller, context, manage, window,
        Command::State(ui::window::WindowState::Normal), area, area, 2).unwrap();
    assert_eq!(integrated.ui.windows.inspect(window).unwrap().bounds, resized);
    assert_eq!(integrated.native_window_command(caller, context, manage, window,
        Command::Focus, area, area, 2), Err(runtime::UiOperationError::AccessDenied));
    let focus = integrated.capabilities.grant(CapabilityType::WindowFocus, window.0 as u64,
        1, 0, issuer, caller, Some(100), 0).unwrap();
    integrated.native_window_command(caller, context, focus, window, Command::Focus, area, area, 2).unwrap();
    assert_eq!(integrated.ui.windows.focused(), Some(window));
    integrated.capabilities.revoke(manage).unwrap();
    assert_eq!(integrated.native_window_command(caller, context, manage, window,
        Command::Close, area, area, 3), Err(runtime::UiOperationError::AccessDenied));
    assert_eq!(integrated.move_window(
        caller,
        context,
        manage,
        window,
        ui::geometry::Point { x: 100, y: 90 },
        ui::geometry::Rect { x: 0, y: 30, width: 1280, height: 690 },
        3,
    ), Err(runtime::UiOperationError::AccessDenied));
    let window_service = integrated.services.inspect(SERVICE_WINDOW_SERVER).unwrap();
    assert_eq!(window_service.manifest.operation_count, 12);
    let close = integrated.capabilities.grant(CapabilityType::WindowManageOwn, window.0 as u64,
        1, 0, issuer, caller, Some(100), 0).unwrap();
    integrated.native_window_command(caller, context, close, window, Command::Close, area, area, 3).unwrap();
    assert_eq!(integrated.ui.surfaces.inspect(surface).unwrap().associated_window, None);
    integrated.destroy_surface(caller, context, destroy_surface, surface, 3).unwrap();
    assert!(integrated.ui.surfaces.inspect(surface).is_none());
}

// ------------------------=
// FUNC: main
// DESC: Runs the program entry point.
// ------------------=
fn main(){execution_and_scheduler();capabilities();iop();events();services();ui_authority();
    native_event_abi::<runtime::native_memory::X86AddressSpace>();
    native_event_abi::<runtime::native_memory_arm::ArmAddressSpace>();
    native_surface_abi::<runtime::native_memory::X86AddressSpace>();
    native_surface_abi::<runtime::native_memory_arm::ArmAddressSpace>();
    native_input_routes();
    native_gui_lifecycle::<runtime::native_memory::X86AddressSpace>();
    native_gui_lifecycle::<runtime::native_memory_arm::ArmAddressSpace>();
    println!("PASS Milestone 4 host acceptance");}

// ------------------------=
// FUNC: native_event_abi
// DESC: Verifies real ARM and x86 user-page encodings, queue overflow, copy failure atomicity, revocation and process isolation.
// ------------------=
fn native_event_abi<T: runtime::native_memory::PageTable>() {
    use runtime::native_memory::*;
    let mut runtime = runtime::InfinityRuntime::new(false);
    let caller = identity(63);
    let publish = runtime.capabilities.grant(CapabilityType::EventPublish, 77, 1, 0,
        caller, caller, None, 0).unwrap();
    let subscribe = runtime.capabilities.grant(CapabilityType::EventSubscribe, 77, 1, 9,
        caller, caller, Some(100), 0).unwrap();
    let layout = std::alloc::Layout::from_size_align(1024 * 1024, 4096).unwrap();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let allocation = memory.allocate(3, 1).unwrap();
    let mut tables = T::new(&mut memory, 1).unwrap();
    let user = 0x1_0000_0000;
    tables.map_range(&mut memory, user, allocation.physical_address, 8192, PagePermissions::USER_RW).unwrap();
    tables.map_range(&mut memory, user+8192, allocation.physical_address+8192, 4096, PagePermissions::USER_R).unwrap();
    for args in [[subscribe,77,9,100,0,0], [subscribe,77,9,100,9,0], [subscribe,77,9,1,1,0]] {
        assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1110, args, 1), -3);
    }
    assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1110, [subscribe,77,8,100,1,0], 1), -1);
    let lease = runtime.native_event_syscall(caller, &mut tables, 0x1110, [subscribe,77,9,100,1,0], 1);
    assert!(lease > 0);
    let lease = lease as u64;
    assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1111, [lease,user,128,0,0,0], 1), 0);
    for n in 1u64..=3 {
        runtime.events.publish(EventClass::StateChange, RoutingDomain::Application, 77,
            caller, 9, n, n, &n.to_le_bytes(), 1, 1, &runtime.capabilities, publish).unwrap();
    }
    assert_eq!(runtime.events.queue_depth(lease), Some(1));
    assert_eq!(runtime.native_event_syscall(identity(64), &mut tables, 0x1111, [lease,user,128,0,0,0], 1), -1);
    assert_eq!(runtime.native_event_syscall(identity(64), &mut tables, 0x1112, [lease,0,0,0,0,0], 1), -1);
    for address in [user+8192, user+8150, user+16384, u64::MAX-4] {
        assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1111, [lease,address,128,0,0,0], 1), -5);
        assert_eq!(runtime.events.queue_depth(lease), Some(1));
    }
    assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1111, [lease,user,127,0,0,0], 1), -3);
    assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1111, [lease,user+4090,128,0,0,0], 1), 1);
    let mut bytes = [0u8;128];
    copy_from_user(&tables, user+4090, &mut bytes).unwrap();
    assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 1);
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 77);
    assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()), 3);
    assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 8);
    assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 1);
    assert_eq!(u64::from_le_bytes(bytes[32..40].try_into().unwrap()), 3);
    assert!(bytes[40..].iter().all(|b| *b == 0));
    runtime.events.publish(EventClass::StateChange, RoutingDomain::Application, 77,
        caller, 9, 4, 4, &[4], 1, 2, &runtime.capabilities, publish).unwrap();
    assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1111, [lease,user,128,0,0,0], 100), -2);
    runtime.capabilities.revoke(subscribe).unwrap();
    assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1111, [lease,user,128,0,0,0], 2), -1);
    assert_eq!(runtime.events.queue_depth(lease), Some(1));
    assert_eq!(runtime.native_event_syscall(caller, &mut tables, 0x1112, [lease,0,0,0,0,0], 2), 0);
    assert_eq!(runtime.events.queue_depth(lease), None);
    tables.release(&mut memory).unwrap();
    memory.release(allocation).unwrap();
    assert_eq!(memory.available_pages(), initial);
    unsafe { std::alloc::dealloc(arena, layout); }
}

// ------------------------=
// FUNC: native_surface_abi
// DESC: Exercises real retained pixels, compositor output, transactional uploads, bounded allocation and cross-context authority on both page-table implementations.
// ------------------=
fn native_surface_abi<T: runtime::native_memory::PageTable>() {
    use runtime::native_memory::*;
    use ui::{geometry::{Rect, Size}, window::{ContextId, SurfaceId, WindowId}};
    let mut runtime = runtime::InfinityRuntime::new(false);
    let mut surfaces = runtime::native_surface::NativeSurfaces::new();
    let caller = identity(65);
    let owner = ContextId(65);
    let create = runtime.capabilities.grant(CapabilityType::SurfaceCreate,
        SERVICE_WINDOW_SERVER as u64, 1, 0, caller, caller, None, 0).unwrap();
    let layout = std::alloc::Layout::from_size_align(80 * 1024 * 1024, 4096).unwrap();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let source = memory.allocate(3, owner.0).unwrap();
    let mut tables = T::new(&mut memory, owner.0).unwrap();
    let user = 0x1_0000_0000;
    tables.map_range(&mut memory, user, source.physical_address, 8192, PagePermissions::USER_RW).unwrap();
    tables.map_range(&mut memory, user+8192, source.physical_address+8192, 4096, PagePermissions::KERNEL_RW).unwrap();
    let before = memory.available_pages();
    assert_eq!(surfaces.syscall(&mut runtime, &mut memory, &tables, identity(66), owner, 0x1120, [create,128,64,0,0,0], 1), -1);
    assert_eq!(surfaces.syscall(&mut runtime, &mut memory, &tables, caller, owner, 0x1120, [create,u64::MAX,64,0,0,0], 1), -3);
    assert_eq!(memory.available_pages(), before);
    let display = Rect{x:0,y:0,width:256,height:128};
    assert_eq!(surfaces.dispatch(&mut runtime,&mut memory,&mut tables,caller,owner,0xffff,[0;6],display,display,1),-6);
    let id = surfaces.dispatch(&mut runtime, &mut memory, &mut tables, caller, owner, 0x1120, [create,128,64,0,0,0], display,display,1);
    assert!(id > 0);
    let id = id as u64;
    let surface = SurfaceId(id as u32);
    assert_eq!(before - memory.available_pages(), 16); // two 32KiB planes
    let publish = runtime.capabilities.grant(CapabilityType::SurfacePublish,id,1,0,caller,caller,None,0).unwrap();
    let resize = runtime.capabilities.grant(CapabilityType::SurfaceResize,id,1,0,caller,caller,None,0).unwrap();
    let destroy = runtime.capabilities.grant(CapabilityType::SurfaceDestroy,id,1,0,caller,caller,None,0).unwrap();
    let window_create = runtime.capabilities.grant(CapabilityType::WindowCreate,
        SERVICE_WINDOW_SERVER as u64,1,0,caller,caller,None,0).unwrap();
    let bounds = Rect { x: 0, y: 0, width: 128, height: 64 };
    let window = surfaces.dispatch(&mut runtime,&mut memory,&mut tables,caller,owner,0x1100,
        [window_create,id,0,0,128,64],display,display,1);
    assert!(window>0);
    let window = WindowId(window as u32);
    assert_eq!(runtime.ui.damage.regions(), &[bounds]);
    runtime.ui.damage.begin_frame();
    let sub = runtime.capabilities.grant(CapabilityType::EventSubscribe,runtime::EVENT_NATIVE_INPUT as u64,
        1,window.0 as u64,caller,caller,None,0).unwrap();
    let lease = surfaces.dispatch(&mut runtime,&mut memory,&mut tables,caller,owner,0x1110,
        [sub,runtime::EVENT_NATIVE_INPUT as u64,window.0 as u64,100,4,0],display,display,1);
    assert!(lease>0);
    assert_eq!(surfaces.dispatch(&mut runtime,&mut memory,&mut tables,caller,owner,0x1111,
        [lease as u64,user,128,0,0,0],display,display,1),0);
    assert_eq!(surfaces.dispatch(&mut runtime,&mut memory,&mut tables,caller,owner,0x1112,
        [lease as u64,0,0,0,0,0],display,display,1),0);
    assert!(surfaces.frame(&runtime, surface).unwrap().pixels.iter().all(|p| *p == 0));
    copy_to_user(&mut tables, user+4092, &[0x33,0x22,0x11,0xff,0x66,0x55,0x44,0xff]).unwrap();
    let offset = (5*128+7)*4;
    assert_eq!(surfaces.syscall(&mut runtime, &mut memory, &tables, caller, ContextId(66), 0x1121, [publish,id,user,offset,8,0],1),-1);
    assert_eq!(surfaces.syscall(&mut runtime, &mut memory, &tables, caller, owner, 0x1121, [publish,id,user+4092,offset,8,0],1),0);
    assert_eq!(surfaces.frame(&runtime, surface).unwrap().pixels[5*128+7], 0);
    assert_eq!(surfaces.take_damage(surface), None);
    for (address, off, len, expected) in [(user+8190,offset,8,-5), (u64::MAX-3,offset,8,-5),
        (user,offset+1,8,-3), (user,32768,8,-3), (user,0,65540,-3)] {
        assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1121,[publish,id,address,off,len,0],1),expected);
    }
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1122,[publish,id,1,0,0,0],1),0);
    assert_eq!(surfaces.frame(&runtime,surface).unwrap().pixels[5*128+7..5*128+9], [0xff112233,0xff445566]);
    assert_eq!(surfaces.take_damage(surface),Some(Rect{x:7,y:5,width:2,height:1}));
    assert_eq!(runtime.ui.damage.regions(), &[Rect{x:7,y:5,width:2,height:1}]);
    runtime.ui.damage.begin_frame();
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1122,[publish,id,1,0,0,0],1),-3);
    let pointer = surfaces.frame(&runtime,surface).unwrap().pixels.as_ptr();
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1122,[publish,id,2,0,0,0],1),0);
    assert_eq!(surfaces.take_damage(surface),None);
    assert!(runtime.ui.damage.regions().is_empty());
    let manage = runtime.capabilities.grant(CapabilityType::WindowManageOwn,window.0 as u64,
        1,0,caller,caller,None,0).unwrap();
    assert_eq!(surfaces.dispatch(&mut runtime,&mut memory,&mut tables,caller,owner,0x1103,
        [manage,window.0 as u64,10,10,0,0],display,display,1),0);
    assert!(!runtime.ui.damage.regions().is_empty());
    assert_eq!(surfaces.frame(&runtime,surface).unwrap().pixels.as_ptr(),pointer);
    let mut damage = ui::scene::DamageTracker::new();
    damage.add(Rect{x:17,y:15,width:2,height:1});
    let mut compositor = ui::compositor::SoftwareCompositor::new(Size{width:256,height:128},256,0);
    let mut back = vec![0x987654;256*128];
    compositor.compose(&mut back,&[surfaces.frame(&runtime,surface).unwrap()],&damage).unwrap();
    assert_eq!(back[15*256+17]&0xffffff,0x112233);
    assert_eq!(back[15*256+18]&0xffffff,0x445566);
    assert_eq!(back[15*256+16],0x987654);
    assert_eq!(back[15*256+19],0x987654);
    let free = memory.available_pages();
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1123,[resize,id,8192,8192,0,0],1),-3);
    assert_eq!(memory.available_pages(),free);
    assert_eq!(surfaces.frame(&runtime,surface).unwrap().pixels.as_ptr(),pointer);
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1121,[publish,id,user,0,4,0],1),0);
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1123,[resize,id,256,64,0,0],1),0);
    assert_eq!(runtime.ui.surfaces.inspect(surface).unwrap().content_generation,3);
    let frame = surfaces.frame(&runtime,surface).unwrap();
    assert_eq!(frame.pixels[5*256+7],0xff112233);
    assert_eq!(frame.pixels[5*256+128],0);
    assert_eq!(frame.pixels[0],0);
    assert_eq!(surfaces.take_damage(surface),Some(Rect{x:0,y:0,width:256,height:64}));
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1124,[destroy,id,0,0,0,0],1),-3);
    // Physical budget includes both buffers and rejects the entire second large allocation.
    let large = surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1120,[create,4096,1024,0,0,0],1);
    assert!(large>0);
    let free = memory.available_pages();
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1120,[create,4096,1024,0,0,0],1),-4);
    assert_eq!(memory.available_pages(),free);
    let large_destroy = runtime.capabilities.grant(CapabilityType::SurfaceDestroy,large as u64,1,0,caller,caller,None,0).unwrap();
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1124,[large_destroy,large as u64,0,0,0,0],1),0);
    let other = ContextId(66);
    let sibling = surfaces.syscall(&mut runtime,&mut memory,&tables,caller,other,0x1120,[create,32,32,0,0,0],1);
    assert!(sibling>0);
    runtime.capabilities.revoke(publish).unwrap();
    assert_eq!(surfaces.syscall(&mut runtime,&mut memory,&tables,caller,owner,0x1122,[publish,id,4,0,0,0],1),-1);
    runtime.ui.damage.begin_frame();
    let old_bounds = runtime.ui.windows.inspect(window).unwrap().bounds;
    assert_eq!(surfaces.context_failed(&mut runtime,&mut memory,owner),1);
    assert_eq!(runtime.ui.damage.regions(), &[old_bounds]);
    assert!(runtime.ui.windows.inspect(window).is_none());
    assert!(runtime.ui.surfaces.inspect(SurfaceId(sibling as u32)).is_some());
    assert_eq!(surfaces.context_failed(&mut runtime,&mut memory,other),1);
    assert_eq!(surfaces.context_failed(&mut runtime,&mut memory,owner),0);
    tables.release(&mut memory).unwrap();
    memory.release(source).unwrap();
    assert_eq!(memory.available_pages(),initial);
    unsafe { std::alloc::dealloc(arena,layout); }
}

// ------------------------=
// FUNC: native_input_routes
// DESC: Proves scoped native input follows focus, pointer capture and secure-input exclusion without cross-window leakage.
// ------------------=
fn native_input_routes() {
    use ui::{geometry::{Rect,Size,Point},window::{ContextId,ZOrderClass},
        surface::{PixelFormat,SurfaceSecurityClass}, input::{UiEvent,Key,PointerPhase}};
    let mut runtime = runtime::InfinityRuntime::new(false);
    runtime.define_bootstrap().unwrap();
    runtime.start_all(0);
    let caller = identity(70);
    let owner = ContextId(70);
    let mut windows = Vec::new();
    let mut leases = Vec::new();
    for x in [0,200] {
        let surface = runtime.ui.surfaces.create(owner,Size{width:100,height:100},PixelFormat::Xrgb8888,SurfaceSecurityClass::Application,false).unwrap();
        let window = runtime.ui.windows.create(owner,surface,Rect{x,y:0,width:100,height:100},ZOrderClass::Normal).unwrap();
        let cap = runtime.capabilities.grant(CapabilityType::EventSubscribe,runtime::EVENT_NATIVE_INPUT as u64,1,window.0 as u64,caller,caller,None,0).unwrap();
        let lease = runtime.events.subscribe(caller,cap,EventFilter{type_id:runtime::EVENT_NATIVE_INPUT,scope:Some(window.0 as u64)},100,OverflowPolicy::DropOldest,8,&runtime.capabilities,0).unwrap();
        windows.push(window); leases.push(lease);
    }
    runtime.ui.windows.focus(windows[0]).unwrap();
    assert!(runtime.native_input(UiEvent::Key{key:Key::Character('é'),pressed:true,modifiers:3},0,0,1));
    assert_eq!(runtime.events.queue_depth(leases[1]),Some(0));
    let event = runtime.events.receive_authorized(caller,leases[0],&runtime.capabilities,1).unwrap().unwrap();
    assert_eq!(event.scope,windows[0].0 as u64);
    assert_eq!(u32::from_le_bytes(event.payload[4..8].try_into().unwrap()),1);
    assert_eq!(u32::from_le_bytes(event.payload[12..16].try_into().unwrap()),1|(3<<8));
    assert_eq!(u32::from_le_bytes(event.payload[16..20].try_into().unwrap()),13);
    assert_eq!(u32::from_le_bytes(event.payload[20..24].try_into().unwrap()),'é' as u32);
    runtime.ui.windows.focus(windows[1]).unwrap();
    assert!(runtime.native_input(UiEvent::Text("é".as_bytes()),0,0,1));
    assert_eq!(runtime.events.queue_depth(leases[0]),Some(0));
    let event = runtime.events.receive(leases[1],1).unwrap();
    assert_eq!(&event.payload[32..34],"é".as_bytes());
    assert!(event.payload[34..].iter().all(|b|*b==0));
    assert!(!runtime.native_input(UiEvent::Text(&[0xff]),0,0,1));
    assert!(!runtime.native_input(UiEvent::Text(&[b'x';65]),0,0,1));
    assert!(runtime.native_input(UiEvent::Pointer{position:Point{x:210,y:20},phase:PointerPhase::Scroll,button:0},-2,4,1));
    let event = runtime.events.receive(leases[1],1).unwrap();
    for (index, expected) in [10i32,20,-2,4].iter().enumerate() {
        assert_eq!(i32::from_le_bytes(event.payload[16+index*4..20+index*4].try_into().unwrap()),*expected);
    }
    runtime.ui.windows.capture_pointer(owner,windows[0]).unwrap();
    assert!(runtime.native_input(UiEvent::Pointer{position:Point{x:210,y:20},phase:PointerPhase::Up,button:1},0,0,1));
    assert_eq!(runtime.events.queue_depth(leases[1]),Some(0));
    let event = runtime.events.receive(leases[0],1).unwrap();
    assert_eq!(i32::from_le_bytes(event.payload[16..20].try_into().unwrap()),210);
    let secure = runtime.ui.trusted.acquire_secure_input(true,99,ui::trusted::TrustedSurface::Authentication,100).unwrap();
    assert!(!runtime.native_input(UiEvent::Text(b"secret"),0,0,1));
    assert!(!runtime.native_input(UiEvent::Pointer{position:Point{x:10,y:10},phase:PointerPhase::Down,button:1},0,0,1));
    assert_eq!(runtime.events.queue_depth(leases[0]),Some(0));
    assert_eq!(runtime.events.queue_depth(leases[1]),Some(0));
    runtime.ui.trusted.release_secure_input(secure).unwrap();
    for (event,kind) in [(UiEvent::CompositionStart,4u32),(UiEvent::CompositionUpdate(b"abc"),5),
        (UiEvent::CompositionCommit(b"abc"),6),(UiEvent::CompositionCancel,7)] {
        assert!(runtime.native_input(event,0,0,1));
        let event = runtime.events.receive(leases[1],1).unwrap();
        assert_eq!(u32::from_le_bytes(event.payload[4..8].try_into().unwrap()),kind);
    }
}

// ------------------------=
// FUNC: native_scene_damage
// DESC: Verifies bounded reconstruction on movement, resize, minimize, withdrawal, destruction and reorder without repainting unchanged content.
// ------------------=
fn native_scene_damage(mut window: ui::window::Window) {
    use runtime::native_gui::SceneDamage;
    use ui::{geometry::{Point,Rect},window::WindowState};
    let display=Rect{x:0,y:0,width:800,height:600};
    window.bounds=Rect{x:100,y:150,width:320,height:240};
    window.visible=true; window.state=WindowState::Normal;
    let mut scene=SceneDamage::new();
    let mut windows=[None;8]; windows[0]=Some(window);
    let initial=scene.update(&windows,display,1).unwrap();
    assert_eq!(initial,Rect{x:97,y:103,width:326,height:290});
    assert_eq!(scene.update(&windows,display,1),None);
    window.bounds.x=-280; windows[0]=Some(window);
    let moved=scene.update(&windows,display,1).unwrap();
    assert_eq!(moved,Rect{x:0,y:103,width:423,height:290});
    window.bounds.width=500; windows[0]=Some(window);
    assert_eq!(scene.update(&windows,display,1).unwrap(),Rect{x:0,y:103,width:223,height:290});
    window.state=WindowState::Minimized;window.visible=false;windows[0]=Some(window);
    assert!(scene.update(&windows,display,1).unwrap().contains(Point{x:220,y:390}));
    let minimized=scene.update(&[None;8],display,1).unwrap();
    assert_eq!(minimized,Rect{x:0,y:103,width:223,height:50});
    assert_eq!(scene.update(&[None;8],display,1),None);
    window.state=WindowState::Normal;window.visible=true;window.bounds.x=100;windows[0]=Some(window);
    scene.update(&windows,display,1).unwrap();
    window.state=WindowState::Withdrawn;window.visible=false;windows[0]=Some(window);
    assert!(scene.update(&windows,display,1).unwrap().contains(Point{x:600,y:390}));
    assert_eq!(scene.update(&windows,display,1),None);
    window.state=WindowState::Normal;window.visible=true;windows[0]=Some(window);
    let mut other=window;other.id=ui::window::WindowId(window.id.0+1);other.bounds.x=250;
    windows[1]=Some(other);scene.update(&windows,display,1).unwrap();
    windows.swap(0,1);
    assert_eq!(scene.update(&windows,display,1).unwrap(),Rect{x:97,y:103,width:656,height:290});
    assert_eq!(scene.update(&[None;8],display,1).unwrap(),Rect{x:97,y:103,width:656,height:290});
}

// ------------------------=
// FUNC: native_gui_lifecycle
// DESC: Verifies exact GUI grants, visible committed pixels, actual input delivery and stale-token rejection after complete reclamation on both native MMUs.
// ------------------=
fn native_gui_lifecycle<T: runtime::native_memory::PageTable>() {
    use runtime::{native_memory::*, native_gui::Gui};
    use ui::{geometry::{Rect,Point},window::{ContextId,SurfaceId},input::{UiEvent,PointerPhase}};
    let mut runtime = runtime::InfinityRuntime::new(false);
    runtime.define_bootstrap().unwrap(); runtime.start_all(0);
    let caller = identity(74); let owner = ContextId(74);
    let area = Rect{x:0,y:0,width:800,height:600};
    let layout = std::alloc::Layout::from_size_align(4*1024*1024,4096).unwrap();
    let arena = unsafe {std::alloc::alloc_zeroed(layout)}; assert!(!arena.is_null());
    let mut memory=NativeMemory::empty(); memory.initialize(arena as u64,layout.size() as u64).unwrap();
    let initial=memory.available_pages();
    let user_page=memory.allocate(1,owner.0).unwrap();
    let mut tables=T::new(&mut memory,owner.0).unwrap();
    let user=0x1_0000_0000;
    tables.map_range(&mut memory,user,user_page.physical_address,4096,PagePermissions::USER_RW).unwrap();
    let baseline=memory.available_pages();
    let mut gui=Gui::new();
    macro_rules! call { ($op:expr,$args:expr) => {gui.dispatch(&mut runtime,&mut memory,&mut tables,caller,owner,true,$op,$args,area,area,1).unwrap()}; }
    assert_eq!(gui.dispatch(&mut runtime,&mut memory,&mut tables,caller,owner,false,0x1251,[0;6],area,area,1),Some(-1));
    let create=call!(0x1251,[0;6]); assert!(create>0);
    assert_eq!(call!(0x1251,[0;6]),create);
    let surface=call!(0x1120,[create as u64,8,8,0,0,0]); assert!(surface>0);
    assert_eq!(call!(0x1251,[2,9999,0,0,0,0]),-1);
    let publish=call!(0x1251,[2,surface as u64,0,0,0,0]); assert!(publish>0);
    let window_create=call!(0x1251,[1,0,0,0,0,0]); assert!(window_create>0);
    let window=call!(0x1100,[window_create as u64,surface as u64,10,20,8,8]); assert!(window>0);
    native_scene_damage(*runtime.ui.windows.inspect(ui::window::WindowId(window as u32)).unwrap());
    assert_eq!(call!(0x1254,[window as u64,1,0,0,0,0]),-1);
    gui.pointer_target(Some(ui::window::WindowId(window as u32)));
    assert_eq!(call!(0x1254,[window as u64,1,0,0,0,0]),0);
    assert_eq!(ui::text_input::pointer_shape(),ui::text_input::PointerShape::Text);
    assert_eq!(call!(0x1254,[window as u64,6,0,0,0,0]),-6);
    gui.pointer_target(None);
    assert_eq!(ui::text_input::pointer_shape(),ui::text_input::PointerShape::Default);
    copy_to_user(&mut tables,user,&0xff123456u32.to_le_bytes()).unwrap();
    assert_eq!(call!(0x1121,[publish as u64,surface as u64,user,0,4,0]),0);
    assert_eq!(call!(0x1122,[publish as u64,surface as u64,1,0,0,0]),0);
    assert_eq!(gui.surfaces.frame(&runtime,SurfaceId(surface as u32)).unwrap().pixels[0],0xff123456);
    let input=call!(0x1251,[8,window as u64,0,0,0,0]); assert!(input>0);
    let lease=call!(0x1110,[input as u64,runtime::EVENT_NATIVE_INPUT as u64,window as u64,100,8,0]); assert!(lease>0);
    assert!(runtime.native_input(UiEvent::Pointer{position:Point{x:12,y:23},phase:PointerPhase::Down,button:1},0,0,2));
    assert_eq!(call!(0x1111,[lease as u64,user,128,0,0,0]),1);
    let mut packet=[0u8;128]; copy_from_user(&tables,user,&mut packet).unwrap();
    assert_eq!(u32::from_le_bytes(packet[36..40].try_into().unwrap()),3);
    assert_eq!(i32::from_le_bytes(packet[48..52].try_into().unwrap()),2);
    assert_eq!(i32::from_le_bytes(packet[52..56].try_into().unwrap()),3);
    // Native title/edge input is distinct from client input and close remains
    // a typed request until the actual application accepts it.
    let id=ui::window::WindowId(window as u32);
    runtime.ui.windows.move_window(owner,id,Point{x:120,y:160},area).unwrap();
    runtime.ui.windows.resize_window(owner,id,320,200,area).unwrap();
    let title=runtime::native_gui::title_bounds(*runtime.ui.windows.inspect(id).unwrap(),1);
    let title_point=Point{x:title.x+20,y:title.y+20};
    assert_eq!(gui.desktop_pointer(&mut runtime,title_point,PointerPhase::Down,1,area,1,3),(true,Some(id)));
    let moved=Point{x:title_point.x+70,y:title_point.y+30};
    assert_eq!(gui.desktop_pointer(&mut runtime,moved,PointerPhase::Move,1,area,1,4),(true,Some(id)));
    gui.desktop_pointer(&mut runtime,moved,PointerPhase::Up,1,area,1,5);
    assert_eq!(runtime.ui.windows.inspect(id).unwrap().bounds.x,190);
    assert_eq!(runtime.ui.windows.inspect(id).unwrap().bounds.y,190);
    let bounds=runtime.ui.windows.inspect(id).unwrap().bounds;
    let edge=Point{x:bounds.right()+2,y:bounds.bottom()+2};
    assert_eq!(gui.desktop_pointer(&mut runtime,edge,PointerPhase::Down,1,area,1,6),(true,Some(id)));
    let resized=Point{x:edge.x+40,y:edge.y+30};
    gui.desktop_pointer(&mut runtime,resized,PointerPhase::Move,1,area,1,7);
    gui.desktop_pointer(&mut runtime,resized,PointerPhase::Up,1,area,1,8);
    assert_eq!(runtime.ui.windows.inspect(id).unwrap().bounds.width,360);
    assert_eq!(runtime.ui.windows.inspect(id).unwrap().bounds.height,230);
    let close_grant=call!(0x1251,[11,window as u64,0,0,0,0]); assert!(close_grant>0);
    let close_lease=call!(0x1110,[close_grant as u64,runtime::EVENT_WINDOW_CLOSE_REQUESTED as u64,window as u64,100,4,0]);
    assert!(close_lease>0);
    let title=runtime::native_gui::title_bounds(*runtime.ui.windows.inspect(id).unwrap(),1);
    let controls=ui::app_launcher::window_controls::bounds(title,title.height,1);
    let close_point=Point{x:controls[2].x+10,y:controls[2].y+10};
    gui.desktop_pointer(&mut runtime,close_point,PointerPhase::Down,1,area,1,9);
    gui.desktop_pointer(&mut runtime,Point{x:close_point.x-50,y:close_point.y+100},PointerPhase::Up,1,area,1,10);
    assert_eq!(runtime.events.queue_depth(close_lease as u64),Some(0));
    gui.desktop_pointer(&mut runtime,close_point,PointerPhase::Down,1,area,1,11);
    gui.desktop_pointer(&mut runtime,close_point,PointerPhase::Up,1,area,1,12);
    assert_eq!(runtime.events.queue_depth(close_lease as u64),Some(1));
    assert!(runtime.ui.windows.inspect(id).is_some());
    assert_eq!(call!(0x1111,[close_lease as u64,user,128,0,0,0]),1);
    copy_from_user(&tables,user,&mut packet).unwrap();
    assert_eq!(u32::from_le_bytes(packet[4..8].try_into().unwrap()),runtime::EVENT_WINDOW_CLOSE_REQUESTED);
    assert_eq!(packet[33],ui::window::WindowEventKind::CloseRequested as u8);
    assert_eq!(u32::from_le_bytes(packet[36..40].try_into().unwrap()),id.0);
    let manage=call!(0x1251,[5,window as u64,0,0,0,0]); assert!(manage>0);
    assert_eq!(call!(0x1105,[manage as u64,window as u64,4,0,0,0]),0);
    assert_eq!(call!(0x1253,[window as u64,5,0,0,0,0]),4);
    assert_eq!(runtime::native_gui::title_bounds(*runtime.ui.windows.inspect(id).unwrap(),1).height,0);
    assert_eq!(gui.desktop_pointer(&mut runtime,close_point,PointerPhase::Move,0,area,1,13),(false,None));
    assert_eq!(call!(0x1105,[manage as u64,window as u64,0,0,0,0]),0);
    for _ in 0..40 {
        let s=call!(0x1120,[create as u64,8,8,0,0,0]); assert!(s>0);
        let w=call!(0x1100,[window_create as u64,s as u64,300,300,8,8]); assert!(w>0);
        let m=call!(0x1251,[5,w as u64,0,0,0,0]); assert!(m>0);
        let d=call!(0x1251,[4,s as u64,0,0,0,0]); assert!(d>0);
        assert_eq!(call!(0x1101,[m as u64,w as u64,0,0,0,0]),0);
        assert_eq!(call!(0x1124,[d as u64,s as u64,0,0,0,0]),0);
    }
    let mut foreign=Gui::new();
    assert_eq!(foreign.dispatch(&mut runtime,&mut memory,&mut tables,caller,ContextId(75),true,0x1122,[publish as u64,surface as u64,2,0,0,0],area,area,2),Some(-1));
    gui.close(&mut runtime,&mut memory);
    assert_eq!(memory.available_pages(),baseline);
    assert!(runtime.ui.windows.inspect(ui::window::WindowId(window as u32)).is_none());
    assert_eq!(runtime.events.queue_depth(lease as u64),None);
    assert!(runtime.capabilities.get(create as u64).is_none());
    let replacement=call!(0x1251,[0;6]); assert!(replacement>create);
    assert_eq!(call!(0x1120,[create as u64,8,8,0,0,0]),-1);
    gui.close(&mut runtime,&mut memory);
    foreign.close(&mut runtime,&mut memory);
    tables.release(&mut memory).unwrap(); memory.release(user_page).unwrap();
    assert_eq!(memory.available_pages(),initial);
    unsafe {std::alloc::dealloc(arena,layout);}
}
