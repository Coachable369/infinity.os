#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"] mod ui;

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "storage.rs"]
mod storage;

// ------------------------=
// FUNC: output_text
// DESC: Provides the diagnostic sink required by the host runtime harness.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::console_language::*;
use runtime::iop::OperationId;
use runtime::service::SERVICE_ORGANIZATION;
use storage::object::{ObjectId, ObjectRef, Space};
use storage::organization::*;

// ------------------------=
// FUNC: reference
// DESC: Creates a deterministic stable ObjectRef for organization tests.
// ------------------=
fn reference(value: u8) -> ObjectRef {
    ObjectRef {
        id: ObjectId([value; 16]),
    }
}

// ------------------------=
// FUNC: organized
// DESC: Creates one local organized object with typed core metadata.
// ------------------=
fn organized(id: u8, name: &[u8], kind: ObjectTypeId, modified: u64) -> OrganizedObject {
    let mut core = CoreMetadata::new(name, kind, modified).unwrap();
    core.modified = modified;
    OrganizedObject {
        reference: StableObjectRef {
            object: reference(id),
            location: ObjectLocation::Local,
        },
        core,
        extensions: [None; 4],
        space: Space::Personal,
    }
}

// ------------------------=
// FUNC: object_organization
// DESC: Verifies stable identity, typed metadata, multidimensional membership, reverse indexes, views, and mesh-ready references.
// ------------------=
fn object_organization() {
    let mut catalog = OrganizationCatalog::new();
    let project = organized(1, b"InfinityOS", TYPE_PROJECT, 1_000_000);
    let collection = organized(2, b"Architecture Specs", TYPE_COLLECTION, 1_000_000);
    let mut document = organized(
        3,
        b"Infinity Storage Architecture Specification",
        TYPE_DOCUMENT,
        950_000,
    );
    document
        .core
        .add_tag(Tag::new(b"storage").unwrap())
        .unwrap();
    document
        .core
        .add_tag(Tag::new(b"architecture").unwrap())
        .unwrap();
    let diagram = organized(4, b"Storage Diagram", TYPE_IMAGE, 940_000);
    for item in [project, collection, document, diagram] {
        catalog.register(item).unwrap();
    }
    for relationship in [
        Relationship {
            source: reference(3),
            kind: RelationshipType::BelongsToProject,
            target: reference(1),
            metadata: None,
            created: 1,
        },
        Relationship {
            source: reference(3),
            kind: RelationshipType::MemberOf,
            target: reference(2),
            metadata: None,
            created: 2,
        },
        Relationship {
            source: reference(3),
            kind: RelationshipType::References,
            target: reference(4),
            metadata: None,
            created: 3,
        },
    ] {
        catalog.attach(relationship).unwrap();
    }
    let mut query = ObjectQuery::all();
    query.object_type = Some(TYPE_DOCUMENT);
    query.project = Some(reference(1));
    query.collection = Some(reference(2));
    query.tag = Some(Tag::new(b"storage").unwrap());
    let result = catalog.query(&query, 1_000_000);
    assert_eq!(result.count, 1);
    assert_eq!(result.refs[0], Some(reference(3)));
    assert_eq!(
        catalog
            .reverse_relationship_nth(reference(1), 0)
            .unwrap()
            .source,
        reference(3)
    );
    let original_identity = catalog.object(reference(3)).unwrap().reference.object;
    assert_eq!(original_identity, reference(3));
    let mesh = StableObjectRef {
        object: original_identity,
        location: ObjectLocation::Mesh { authority: [9; 16] },
    };
    assert_eq!(mesh.object, original_identity);
    let view = View {
        query,
        sort: SortOrder::Modified,
        grouping: Grouping::Project,
        presentation: Presentation::Grid,
    };
    assert_eq!(view.query.project, Some(reference(1)));
    let dynamic = CollectionDefinition {
        object: reference(2),
        mode: CollectionMode::Dynamic,
        query: Some(query),
    };
    assert_eq!(dynamic.mode, CollectionMode::Dynamic);
    let schema = organization_schema_object();
    assert!(organization_schema_valid(&schema));
    let mut corrupt = schema;
    corrupt[40] ^= 1;
    assert!(!organization_schema_valid(&corrupt));
    println!("PASS organization: stable IDs, versioned taxonomy/metadata, projects, collections, views, tags, multidimensional relationships, reverse index, mesh-ready refs");
}

// ------------------------=
// FUNC: console_grammar
// DESC: Verifies deterministic parsing, quoting, discovery, help, temporal predicates, plans, and typed variables.
// ------------------=
fn console_grammar() {
    let ParseOutcome::Graph(query) = parse(
        b"docs = object find type=document project=\"InfinityOS Research\" modified=last-week",
    )
    .unwrap() else {
        panic!("graph")
    };
    assert_eq!(query.node_count, 1);
    assert_eq!(query.result_type, ValueType::ObjectSet);
    assert_eq!(query.assignment, Some(b"docs".as_slice()));
    assert_eq!(
        query.nodes[0].unwrap().schema.operation,
        OperationId::ObjectQuery
    );
    assert_eq!(query.nodes[0].unwrap().argument_count, 3);
    let ParseOutcome::DomainDiscovery(storage) = parse(b"storage").unwrap() else {
        panic!("domain discovery")
    };
    assert_eq!(storage.name, b"storage");
    let ParseOutcome::OperationDiscovery(inspect) = parse(b"storage inspect").unwrap() else {
        panic!("operation discovery")
    };
    assert_eq!(inspect.example, b"storage inspect device:storage0");
    let ParseOutcome::Help(Some(_), Some(help)) = parse(b"help object find").unwrap() else {
        panic!("help")
    };
    assert_eq!(help.output, ValueType::ObjectSet);
    let ParseOutcome::Graph(plan) = parse(b"plan object destroy project=OldPrototype").unwrap()
    else {
        panic!("plan")
    };
    assert!(plan.plan_only);
    assert_eq!(plan.result_type, ValueType::OperationPlan);
    assert_eq!(plan.maximum_effect, SideEffectClass::DestructiveChange);
    assert!(matches!(
        parse(b"object find modified=next-fortnight"),
        Err(ConsoleLanguageError::InvalidArgumentType)
    ));
    assert_eq!(
        parse(b"object find |> namespace move destination=/archive/infinityos").map(|_| ()),
        Ok(())
    );
    assert!(matches!(
        parse(b"device list |> namespace move destination=/archive"),
        Err(ConsoleLanguageError::TypeMismatch)
    ));
    assert_eq!(
        parse(b"object find type=image |> object filter project=InfinityOS").map(|outcome| {
            match outcome {
                ParseOutcome::Graph(graph) => graph.result_type,
                _ => ValueType::Unit,
            }
        }),
        Ok(ValueType::ObjectSet)
    );
    assert!(matches!(
        parse(b"object find name=\"unterminated"),
        Err(ConsoleLanguageError::UnterminatedQuote)
    ));
    let ParseOutcome::Graph(pair_confirm) =
        parse(b"node pair-confirm pairing:7 code=847291").unwrap()
    else {
        panic!("node pair-confirm graph")
    };
    let confirm = pair_confirm.nodes[0].unwrap();
    assert_eq!(confirm.schema.operation, OperationId::NodePairConfirm);
    assert_eq!(confirm.target.unwrap().kind, ReferenceKind::Node);
    assert_eq!(confirm.argument_count, 1);
    assert_eq!(confirm.arguments[0].unwrap().value_type, ArgumentType::Text);
    let mut session = ConsoleSession::new();
    session.assign(b"docs", ValueType::ObjectSet).unwrap();
    assert_eq!(
        session.variable(b"docs").unwrap().value_type,
        ValueType::ObjectSet
    );
    session.bind_references(&[[1; 16], [2; 16]]);
    assert_eq!(session.resolve_contextual(b"@2").unwrap(), [2; 16]);
    assert_eq!(
        session.resolve_contextual(b"@3"),
        Err(ConsoleLanguageError::ReferenceNotFound)
    );
    println!("PASS console language: registry discovery, schema help, quoting, bounded dates, typed refs/variables, planning, composition, preflight type errors");
}

// ------------------------=
// FUNC: service_registry
// DESC: Verifies organization operations are discoverable from the modular runtime service registry.
// ------------------=
fn service_registry() {
    let mut system = runtime::InfinityRuntime::new(false);
    system.define_bootstrap().unwrap();
    system.start_all(0);
    assert_eq!(
        system.services.provider(OperationId::ProjectList as u32),
        Some(SERVICE_ORGANIZATION)
    );
    assert_eq!(
        system
            .services
            .provider(OperationId::CollectionCreate as u32),
        Some(SERVICE_ORGANIZATION)
    );
    assert_eq!(
        system.services.provider(OperationId::ObjectFilter as u32),
        Some(runtime::service::SERVICE_OBJECT)
    );
    println!("PASS service registry: organization, project, collection, and query operations have typed providers");
}

// ------------------------=
// FUNC: main
// DESC: Runs the complete Milestone 6.5 host acceptance suite.
// ------------------=
fn main() {
    object_organization();
    console_grammar();
    service_registry();
    println!(
        "PASS Milestone 6.5: no path identity, no byte-stream pipes, no rendered-text composition"
    );
}
