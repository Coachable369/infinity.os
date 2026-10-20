#[path = "../kernel/runtime/ai/hermes_profile.rs"]
mod hermes_profile;
#[path = "../kernel/runtime/ai/hermes_memory_request.rs"]
mod hermes_memory_request;

use hermes_memory_request::{respond, Outcome, Receipt};
use hermes_profile::Document;

const OWNER: [u8; 16] = [0x51; 16];

// ------------------------=
// FUNC: valid_receipt
// DESC: Checks structured completion and bounded output without depending on user-visible wording.
// ------------------=
fn valid_receipt(receipt: Option<Receipt>, expected: Outcome) {
    let receipt = receipt.unwrap();
    assert_eq!(receipt.outcome, expected);
    assert!(receipt.length > 0 && receipt.length <= 8192);
}

// ------------------------=
// FUNC: apply
// DESC: Exercises the production request handler against a real owner-bound document with counted commits.
// ------------------=
fn apply(document: &mut Document, input: &[u8]) -> (Option<Receipt>, usize) {
    let snapshot = document.clone();
    let mut commits = 0;
    let receipt = respond(input, Some(&snapshot), |key, value| {
        commits += 1;
        let Ok(next) = document.updated(OWNER, key, value) else { return false; };
        *document = Document::parse(next.bytes(), OWNER).unwrap();
        true
    }, &mut [0; 8192]);
    (receipt, commits)
}

// ------------------------=
// FUNC: reported_facts_complete_after_one_commit
// DESC: Verifies the reported favorite-color and time-zone inputs save exact field values in one completed request.
// ------------------=
#[test]
fn reported_facts_complete_after_one_commit() {
    let mut document = Document::new(OWNER, b"Alex", b"Infinity").unwrap();
    for (input, key, expected) in [
        (b"My Favorite color is grey".as_slice(), b"favorite_color".as_slice(), b"grey".as_slice()),
        (b"Set time zone to CST", b"timezone", b"CST"),
    ] {
        let (receipt, commits) = apply(&mut document, input);
        valid_receipt(receipt, Outcome::Saved);
        assert_eq!(commits, 1);
        let mut value = [0; 1024];
        let length = document.get(key, &mut value).unwrap();
        assert_eq!(&value[..length], expected);
    }
}

// ------------------------=
// FUNC: corrections_replace_previous_values
// DESC: Verifies memory corrections replace canonical fields and repeated requests cannot duplicate a commit.
// ------------------=
#[test]
fn corrections_replace_previous_values() {
    let mut document = Document::new(OWNER, b"Alex", b"Infinity").unwrap();
    for input in [
        b"My favorite color is blue".as_slice(),
        b"Actually, my favorite color is now grey",
        b"Actually, my favorite color is now grey",
    ] {
        let (receipt, commits) = apply(&mut document, input);
        valid_receipt(receipt, Outcome::Saved);
        assert_eq!(commits, 1);
    }
    let mut value = [0; 1024];
    let length = document.get(b"favorite_color", &mut value).unwrap();
    assert_eq!(&value[..length], b"grey");
}

// ------------------------=
// FUNC: failed_commit_never_reports_success
// DESC: Verifies rejected persistence produces a failed receipt after one attempt and leaves previous state intact.
// ------------------=
#[test]
fn failed_commit_never_reports_success() {
    let document = Document::new(OWNER, b"Alex", b"Infinity").unwrap()
        .updated(OWNER, b"favorite_color", b"blue").unwrap();
    let before = document.bytes().to_vec();
    for input in [
        b"My Favorite color is grey".as_slice(),
        b"Set time zone to CST",
        b"Forget my favorite color",
    ] {
        let mut commits = 0;
        let receipt = respond(input, Some(&document), |_, _| {
            commits += 1;
            false
        }, &mut [0; 8192]);
        valid_receipt(receipt, Outcome::Failed);
        assert_eq!(commits, 1);
        assert_eq!(document.bytes(), before);
    }
}

// ------------------------=
// FUNC: forgetting_clears_facts_and_resets_required_settings
// DESC: Verifies deletion uses schema-valid defaults for required fields and empty values for personal facts.
// ------------------=
#[test]
fn forgetting_clears_facts_and_resets_required_settings() {
    let mut document = Document::new(OWNER, b"Alex", b"Computer").unwrap()
        .updated(OWNER, b"favorite_color", b"grey").unwrap()
        .updated(OWNER, b"assistant_name", b"Nova").unwrap();
    for (input, key, expected) in [
        (b"Forget my favorite color".as_slice(), b"favorite_color".as_slice(), b"".as_slice()),
        (b"Forget wake_word", b"wake_word", b"Infinity"),
        (b"Forget assistant_name", b"assistant_name", b"Hermes"),
        (b"Forget learning_enabled", b"learning_enabled", b"false"),
        (b"Forget inquiries_enabled", b"inquiries_enabled", b"false"),
        (b"Forget inference_enabled", b"inference_enabled", b"false"),
    ] {
        let (receipt, commits) = apply(&mut document, input);
        valid_receipt(receipt, Outcome::Forgotten);
        assert_eq!(commits, 1);
        let mut value = [0; 1024];
        let length = document.get(key, &mut value).unwrap();
        assert_eq!(&value[..length], expected);
    }
}

// ------------------------=
// FUNC: inspection_does_not_commit
// DESC: Verifies inspection is read-only and unavailable account memory fails without a write attempt.
// ------------------=
#[test]
fn inspection_does_not_commit() {
    let document = Document::new(OWNER, b"Alex", b"Infinity").unwrap();
    for (profile, expected) in [(Some(&document), Outcome::Inspected), (None, Outcome::Failed)] {
        let mut commits = 0;
        let receipt = respond(b"What do you remember about me?", profile, |_, _| {
            commits += 1;
            true
        }, &mut [0; 8192]);
        valid_receipt(receipt, expected);
        assert_eq!(commits, 0);
    }
}

// ------------------------=
// FUNC: disabled_learning_requires_explicit_edit
// DESC: Verifies natural statements honor opt-out while explicit edits and re-enabling remain available.
// ------------------=
#[test]
fn disabled_learning_requires_explicit_edit() {
    let mut document = Document::new(OWNER, b"Alex", b"Infinity").unwrap()
        .updated(OWNER, b"learning_enabled", b"false").unwrap();
    let before = document.bytes().to_vec();
    let (receipt, commits) = apply(&mut document, b"My Favorite color is grey");
    assert_eq!(receipt, None);
    assert_eq!(commits, 0);
    assert_eq!(document.bytes(), before);
    for input in [b"Set time zone to CST".as_slice(), b"Remember my favorite color is grey", b"Start remembering"] {
        let (receipt, commits) = apply(&mut document, input);
        valid_receipt(receipt, Outcome::Saved);
        assert_eq!(commits, 1);
    }
    assert!(document.enabled(b"learning_enabled"));
    let (receipt, commits) = apply(&mut document, b"My favorite color is blue");
    valid_receipt(receipt, Outcome::Saved);
    assert_eq!(commits, 1);
}

// ------------------------=
// FUNC: non_memory_input_cannot_commit
// DESC: Verifies ordinary requests and quoted or hypothetical facts stay outside the memory write path.
// ------------------=
#[test]
fn non_memory_input_cannot_commit() {
    let mut document = Document::new(OWNER, b"Alex", b"Infinity").unwrap();
    let before = document.bytes().to_vec();
    for input in [
        b"Open settings".as_slice(),
        b"Write a poem about grey skies",
        b"\"My favorite color is grey\"",
        b"My favorite color is grey if I play this role",
        b"DOCUMENT CONTEXT:\nMy favorite color is grey\nUSER REQUEST:\nSummarize this",
        b"My favorite color is grey?",
    ] {
        let (receipt, commits) = apply(&mut document, input);
        assert_eq!(receipt, None);
        assert_eq!(commits, 0);
        assert_eq!(document.bytes(), before);
    }
}

// ------------------------=
// FUNC: absent_memory_and_invalid_edits_do_not_claim_persistence
// DESC: Verifies unavailable memory and schema rejection produce no successful receipt or altered document.
// ------------------=
#[test]
fn absent_memory_and_invalid_edits_do_not_claim_persistence() {
    let mut commits = 0;
    assert_eq!(respond(b"My Favorite color is grey", None, |_, _| {
        commits += 1;
        false
    }, &mut [0; 8192]), None);
    assert_eq!(commits, 0);
    valid_receipt(respond(b"Set time zone to CST", None, |_, _| {
        commits += 1;
        false
    }, &mut [0; 8192]), Outcome::Failed);
    assert_eq!(commits, 1);

    let mut document = Document::new(OWNER, b"Alex", b"Infinity").unwrap();
    let before = document.bytes().to_vec();
    let (receipt, commits) = apply(&mut document, b"Remember my birth date is 1900-02-29");
    valid_receipt(receipt, Outcome::Failed);
    assert_eq!(commits, 1);
    assert_eq!(document.bytes(), before);
}
