use super::*;
use core::sync::atomic::{AtomicUsize, Ordering};
static CALLS: AtomicUsize = AtomicUsize::new(0);
// ------------------------=
// FUNC: signed_bundle_uses_exact_operator_scope_and_canonical_payload
// DESC: Exercises production bundle construction with real node keys, explicit destination leases, signed read principals and tamper rejection.
// ------------------=
#[test]
fn signed_bundle_uses_exact_operator_scope_and_canonical_payload() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let mut r = InfinityRuntime::new(false);
            r.define_bootstrap().unwrap();
            r.start_all(0);
            r.nodes.initialize(&[91; 32], true).unwrap();
            r.node_clock = Some(10);
            let mut b = node::NodeRuntime::new();
            b.initialize(&[92; 32], true).unwrap();
            let mut c = node::NodeRuntime::new();
            c.initialize(&[93; 32], true).unwrap();
            for (i, n) in [&b, &c].iter().enumerate() {
                r.nodes
                    .discover(n.advertise(1, 1, 10).unwrap(), 10)
                    .unwrap();
                r.storage_metadata.peers[i] = Some(Peer {
                    node: n.local_id().unwrap(),
                    grant: i as u64 + 1,
                    expires: 100 + i as u64,
                });
            }
            r.identity
                .create_machine(b"signed-metadata", 64, 1, 0)
                .unwrap();
            let user = r
                .identity
                .create_user(b"operator", b"Operator", 0)
                .unwrap()
                .id;
            r.identity.create_password(user, b"Fixture901", 0).unwrap();
            let session = r
                .identity
                .create_session(user, b"Fixture901", 1)
                .unwrap()
                .id;
            start(&mut r, user, session, [7; 16], b"/Shared/Test", None).unwrap();
            let j = r.storage_metadata.job.as_ref().unwrap();
            let mut manifest = fabric::manifest::Manifest {
                object: [7; 16],
                version: 1,
                length: 128,
                hash: Sha256::digest([7u8; 128]).into(),
                policy: fabric::placement::StorageClass::Protected,
                minimum_available: 1,
                generation: 1,
                authority: r.nodes.local_id().unwrap(),
                authority_generation: 1,
                chunks: [None; fabric::manifest::MAX_CHUNKS],
                placements: [None; fabric::manifest::MAX_PLACEMENTS],
                healing: None,
            };
            manifest.chunks[0] = Some(fabric::manifest::Chunk {
                content: [8; 16],
                bytes: 128,
                hash: manifest.hash,
            });
            let bundle = build_bundle(&r, j, manifest, 10).unwrap();
            bundle.validate().unwrap();
            assert_eq!(bundle.group.members[1], b.local_id().unwrap());
            assert_eq!(bundle.path(), b"/Shared/Test");
            for grant in bundle.grants.iter().flatten() {
                assert_eq!(grant.principal, principal());
                assert_eq!(grant.object, [7; 16]);
            }
            for grant in bundle.repair_grants.iter().flatten() {
                assert_eq!(grant.expires, 100);
                assert_eq!(grant.destinations[0], b.local_id().unwrap());
                assert_eq!(grant.destinations[1], c.local_id().unwrap());
                assert_eq!(grant.destinations[2], NodeId([0; 32]));
                grant
                    .validate_anchor(&bundle.group, &bundle.value, 99)
                    .unwrap();
                assert!(grant
                    .validate_anchor(&bundle.group, &bundle.value, 100)
                    .is_err());
            }
            let encoded = bundle.encode().unwrap();
            let decoded = Bundle::decode(&encoded).unwrap();
            assert_eq!(decoded.manifest, manifest);
            let mut reader = InfinityRuntime::new(false);
            reader.define_bootstrap().unwrap();
            reader.start_all(0);
            reader.nodes.initialize(&[92; 32], true).unwrap();
            reader.node_clock = Some(10);
            assert!(service_delegated(&reader, &decoded, 99));
            assert!(!service_delegated(&reader, &decoded, 100));
            assert!(!service_delegated(&r, &decoded, 10));
            let service_id = fresh_start_service(&mut reader, decoded.manifest.object).unwrap();
            assert!(matches!(
                fresh_take_service(&mut reader, service_id + 1),
                Err(RemoteError::NotFound)
            ));
            let service_job = reader.storage_metadata.job.as_mut().unwrap();
            service_job.bundle = Some(decoded);
            service_job.result = Some(Ok(service_job.read.unwrap()));
            reader.node_clock = Some(100);
            assert!(matches!(
                fresh_take_service(&mut reader, service_id),
                Err(RemoteError::AccessDenied)
            ));
            assert!(reader.storage_metadata.job.is_none());
            let mut tampered = encoded;
            tampered[6240] ^= 1;
            assert!(Bundle::decode(&tampered).is_err());
            assert!(bundle
                .authorize(b.local_id().unwrap(), principal(), 10)
                .is_err());
            let mut job = r.storage_metadata.job.take().unwrap();
            job.bundle = Some(decoded);
            job.phase = Phase::RemoteRead;
            job.read = Some(StorageOperationV1 {
                operation: Operation::ObjectRead,
                object: [7; 16],
                authority_generation: 1,
                manifest_generation: 1,
                object_version: 1,
                offset: 10,
                scope: 0,
                value: 64,
                length: 0,
                data: [0; 64],
            });
            let req = request(&job, 6);
            let pending = Pending {
                id: 1,
                cap: 1,
                peer: b.local_id().unwrap(),
                deadline: 20,
                request: req,
            };
            let mut response = req;
            response.length = 64;
            response.data = [7; 64];
            advance_reply(&mut r, &mut job, pending, response, 10).unwrap();
            assert!(job.result.is_none());
            assert_eq!(job.read_result, [0; 64]);
            response.offset = 64;
            response.data[0] = 9;
            assert_eq!(
                advance_reply(&mut r, &mut job, pending, response, 10),
                Err(RemoteError::RemoteFailure)
            );
            assert!(job.result.is_none());
            assert_eq!(job.read_result, [0; 64]);
            job.remote_at = 0;
            response.offset = 0;
            response.data = [7; 64];
            advance_reply(&mut r, &mut job, pending, response, 10).unwrap();
            response.offset = 64;
            advance_reply(&mut r, &mut job, pending, response, 10).unwrap();
            let result = job.result.unwrap().unwrap();
            assert_eq!(result.length, 64);
            assert_eq!(result.offset, 10);
            assert_eq!(result.data, [7; 64]);
        })
        .unwrap()
        .join()
        .unwrap();
}
// ------------------------=
// FUNC: empty_backend
// DESC: Counts typed native reads and forbids unintended mutation during namespace scanning.
// ------------------=
fn empty_backend(request: NativeRequest) -> Result<NativeReply, RemoteError> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    match request {
        NativeRequest::ConfigLoad => Ok(NativeReply::Config([0; CONFIG_BYTES])),
        NativeRequest::PendingMutation { index } => {
            assert!(index < 8);
            Ok(NativeReply::Pending(None))
        }
        NativeRequest::Load { index } => {
            assert!(index < 8);
            Ok(NativeReply::Bundle(None))
        }
        _ => panic!("unexpected mutation"),
    }
}
// ------------------------=
// FUNC: namespace_scan_is_bounded_and_unauthorized_requests_do_not_mutate
// DESC: Exercises actual service startup and polling cadence, explicit no-authority defaults, one-job limits, and session-bound asynchronous admission.
// ------------------=
#[test]
fn namespace_scan_is_bounded_and_unauthorized_requests_do_not_mutate() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let mut r = InfinityRuntime::new(false);
            r.define_bootstrap().unwrap();
            r.start_all(0);
            r.nodes.initialize(&[81; 32], true).unwrap();
            r.node_clock = Some(10);
            r.storage_metadata.handler = Some(empty_backend);
            let before = CALLS.load(Ordering::SeqCst);
            poll(&mut r, 10);
            assert_eq!(CALLS.load(Ordering::SeqCst), before + 1);
            assert!(r.storage_metadata.loaded);
            let before = CALLS.load(Ordering::SeqCst);
            for _ in 0..100 {
                poll(&mut r, 11);
            }
            assert_eq!(CALLS.load(Ordering::SeqCst), before + 1);
            for t in 12..28 {
                poll(&mut r, t);
            }
            assert!(r.storage_metadata.scan < 9);
            assert!(r.storage_metadata.names.iter().all(Option::is_none));
            let local = r.nodes.local_id().unwrap();
            assert_eq!(peer_grant(&r, local, 28), Err(RemoteError::AccessDenied));
            r.identity
                .create_machine(b"metadata-test", 64, 1, 0)
                .unwrap();
            let user = r
                .identity
                .create_user(b"operator", b"Operator", 0)
                .unwrap()
                .id;
            r.identity.create_password(user, b"Fixture901", 0).unwrap();
            let session = r
                .identity
                .create_session(user, b"Fixture901", 1)
                .unwrap()
                .id;
            let id = fresh_start(&mut r, user, session, [1; 16]).unwrap();
            assert_eq!(id >> 62, 3);
            assert_eq!(
                fresh_start(&mut r, user, session, [2; 16]),
                Err(RemoteError::QueueFull)
            );
            assert!(fresh_take(&mut r, user, session, id).unwrap().is_none());
            for t in 29..38 {
                poll(&mut r, t);
            }
            assert!(matches!(
                r.storage_metadata.job.as_ref().unwrap().result,
                Some(Err(RemoteError::NotFound))
            ));
            assert!(fresh_take(&mut r, user, session, id + 1).is_err());
            assert!(matches!(
                fresh_take(&mut r, user, session, id),
                Err(RemoteError::NotFound)
            ));
            assert!(r.storage_metadata.job.is_none());
            let retry = fresh_start(&mut r, user, session, [1; 16]).unwrap();
            fresh_cancel(&mut r, user, session, retry + 1);
            assert!(r.storage_metadata.job.is_some());
            fresh_cancel(&mut r, user, session, retry);
            assert!(r.storage_metadata.job.is_none());
        })
        .unwrap()
        .join()
        .unwrap();
}
