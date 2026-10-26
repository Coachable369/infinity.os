//! Model-independent acceptance requirements. Prompt guidance is not test evidence.
use super::editor_compile_intent::{self, ApplicationKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Criterion {
    Startup, RequestedBehavior, InputValidation, CleanExit,
    Arithmetic, DecimalInput, Clear, DivisionByZero,
    LegalMoves, TurnOrder, EndState, Restart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contract<'a> {
    pub request: &'a [u8],
    pub kind: ApplicationKind,
    pub criteria: &'static [Criterion],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Untested, Passed, Failed }

/// One attempt's evidence. Create a fresh assessment whenever the source changes;
/// never carry passing observations forward from an older executable.
pub struct Assessment<'a> {
    pub contract: Contract<'a>,
    outcomes: [Outcome; 12],
}

impl<'a> Assessment<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Starts every required behavior unverified, regardless of compilation or launch success.
    // ------------------=
    pub fn new(contract: Contract<'a>) -> Self {
        Self { contract, outcomes: [Outcome::Untested; 12] }
    }

    // ------------------------=
    // FUNC: record
    // DESC: Records a required criterion's observed result; unrelated evidence cannot satisfy a contract.
    // ------------------=
    pub fn record(&mut self, criterion: Criterion, outcome: Outcome) -> bool {
        if !self.contract.criteria.contains(&criterion) { return false; }
        self.outcomes[criterion as usize] = outcome;
        true
    }

    // ------------------------=
    // FUNC: outcome
    // DESC: Returns typed evidence for a requirement, or None when it is outside this contract.
    // ------------------=
    pub fn outcome(&self, criterion: Criterion) -> Option<Outcome> {
        self.contract.criteria.contains(&criterion).then_some(self.outcomes[criterion as usize])
    }

    // ------------------------=
    // FUNC: passed
    // DESC: Requires explicit passing evidence for every obligation, including the original requested behavior.
    // ------------------=
    pub fn passed(&self) -> bool {
        self.contract.criteria.iter().all(|criterion| self.outcome(*criterion) == Some(Outcome::Passed))
    }
}

const GENERAL: &[Criterion] = &[Criterion::Startup, Criterion::RequestedBehavior,
    Criterion::InputValidation, Criterion::CleanExit];
const CALCULATOR: &[Criterion] = &[Criterion::Startup, Criterion::RequestedBehavior,
    Criterion::InputValidation, Criterion::CleanExit, Criterion::Arithmetic,
    Criterion::DecimalInput, Criterion::Clear, Criterion::DivisionByZero];
const INTEGER_CALCULATOR: &[Criterion] = &[Criterion::Startup, Criterion::RequestedBehavior,
    Criterion::InputValidation, Criterion::CleanExit, Criterion::Arithmetic,
    Criterion::Clear, Criterion::DivisionByZero];
const GAME: &[Criterion] = &[Criterion::Startup, Criterion::RequestedBehavior,
    Criterion::InputValidation, Criterion::CleanExit, Criterion::LegalMoves,
    Criterion::TurnOrder, Criterion::EndState, Criterion::Restart];

impl<'a> Contract<'a> {
    // ------------------------=
    // FUNC: from_request
    // DESC: Selects trusted baseline criteria from the original explicit request, never generated source or model assertions.
    // ------------------=
    pub fn from_request(request: &'a [u8]) -> Option<Self> {
        let kind = editor_compile_intent::application_kind(request)?;
        Some(Self { request, kind, criteria: match kind {
            ApplicationKind::Calculator if request.split(|b| !b.is_ascii_alphanumeric())
                .any(|word| word.eq_ignore_ascii_case(b"integer")) => INTEGER_CALCULATOR,
            ApplicationKind::Calculator => CALCULATOR,
            ApplicationKind::Game => GAME,
            ApplicationKind::General => GENERAL,
        } })
    }

    // ------------------------=
    // FUNC: guidance
    // DESC: Supplies implementation obligations without representing them as passed behavioral checks.
    // ------------------=
    pub fn guidance(self) -> &'static [u8] {
        match self.kind {
            ApplicationKind::Calculator if !self.criteria.contains(&Criterion::DecimalInput) => b"\nACCEPTANCE CONTRACT: Implement the entire user request. Integer calculator: correct +,-,*,/; signed values; clear/reset; reject invalid input and arithmetic overflow; handle division by zero without crashing; repeated operations and clean exit. Keep arithmetic separate from UI handlers for tests. Compilation or launch is NOT functional acceptance. Preserve these requirements during repair.\n",
            ApplicationKind::Calculator => b"\nACCEPTANCE CONTRACT: Implement the entire user request. Calculator: correct +,-,*,/; decimals and negative values; clear/reset; reject invalid input and arithmetic overflow; division by zero must not crash; repeated operations and clean exit. C can use checked scaled-integer arithmetic and integer formatting for decimals. Keep arithmetic separate from UI handlers for tests. Compilation or launch is NOT functional acceptance. Preserve these requirements during repair.\n",
            ApplicationKind::Game => b"\nACCEPTANCE CONTRACT: Implement the entire requested game's rules, legal moves, turn order, win/draw termination and restart. Reject illegal input without mutating game state. Keep rules separate from rendering so they can be tested. Startup and clean exit must work. Compilation or publishing controls is NOT functional acceptance. Do not weaken requirements during repair.\n",
            ApplicationKind::General => b"\nACCEPTANCE CONTRACT: Implement every requested feature, real input validation, repeated interaction, startup and clean exit. Keep domain logic separate from UI handlers so it can be tested. Do not substitute scaffolding or omit features to fit output limits. Compilation or launching is NOT functional acceptance. Do not weaken requirements during repair.\n",
        }
    }
}

// ------------------------=
// FUNC: guidance
// DESC: Returns no additional obligations for non-creation requests rather than inventing an application task.
// ------------------=
pub fn guidance(request: &[u8]) -> &'static [u8] {
    Contract::from_request(request).map_or(b"", Contract::guidance)
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: acceptance_requires_complete_current_evidence
    // DESC: Exercises partial evidence, failed behavior, correction and fresh-attempt invalidation.
    // ------------------=
    #[test]
    fn acceptance_requires_complete_current_evidence() {
        let contract=Contract::from_request(b"Create a calculator application").unwrap();
        let mut assessment=Assessment::new(contract);
        assert!(!assessment.passed());
        assert!(!assessment.record(Criterion::LegalMoves,Outcome::Passed));
        assert_eq!(assessment.outcome(Criterion::LegalMoves),None);
        for criterion in contract.criteria {
            if *criterion != Criterion::DivisionByZero {
                assert!(assessment.record(*criterion,Outcome::Passed));
            }
        }
        assert!(!assessment.passed());
        assessment.record(Criterion::DivisionByZero,Outcome::Failed);
        assert!(!assessment.passed());
        assessment.record(Criterion::DivisionByZero,Outcome::Passed);
        assert!(assessment.passed());
        assert!(!Assessment::new(contract).passed());
    }
    // ------------------------=
    // FUNC: calculator_requires_behavior_not_just_launch
    // DESC: Checks typed required coverage independently of prompt prose and generated source.
    // ------------------=
    #[test]
    fn calculator_requires_behavior_not_just_launch() {
        for request in [b"Create a C calculator application".as_slice(), b"Create a calculator in Python"] {
            let contract = Contract::from_request(request).unwrap();
            assert_eq!(contract.kind, ApplicationKind::Calculator);
            assert_eq!(contract.criteria, CALCULATOR);
            assert!(contract.criteria.contains(&Criterion::RequestedBehavior));
            assert!(contract.criteria.contains(&Criterion::DivisionByZero));
        }
    }
    // ------------------------=
    // FUNC: contracts_are_task_specific
    // DESC: Keeps game and general requirements distinct and refuses to create a contract for explanatory requests.
    // ------------------=
    #[test]
    fn contracts_are_task_specific() {
        assert_eq!(Contract::from_request(b"Create a chess game").unwrap().criteria, GAME);
        assert_eq!(Contract::from_request(b"Create a calendar application").unwrap().criteria, GENERAL);
        assert_eq!(Contract::from_request(b"Explain how to create a calculator"), None);
        assert_eq!(Contract::from_request(b"Fix the code"), None);
        let integer=Contract::from_request(b"Create an integer calculator application").unwrap();
        assert!(!integer.criteria.contains(&Criterion::DecimalInput));
        assert!(integer.criteria.contains(&Criterion::Arithmetic));
    }
}
