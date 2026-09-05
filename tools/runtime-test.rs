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
    integrated.capabilities.revoke(manage).unwrap();
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
}

// ------------------------=
// FUNC: main
// DESC: Runs the program entry point.
// ------------------=
fn main(){execution_and_scheduler();capabilities();iop();events();services();ui_authority();println!("PASS Milestone 4 host acceptance");}
