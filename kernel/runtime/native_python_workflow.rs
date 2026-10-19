//! Ordered evidence for generated Python validation and execution.

// Only this trusted validation wrapper emits EX_DATAERR for source syntax.
// Import/startup, missing-input, resource and runtime failures keep their own
// status and must never trigger a model rewrite of valid user source.
pub const SYNTAX_ERROR_EXIT: i64 = 65;
pub const VALIDATE_SOURCE: &[u8] = b"import sys\ntry:\n    compile(open(sys.argv[1], 'rb').read(), sys.argv[1], 'exec')\nexcept SyntaxError as error:\n    sys.stderr.write(type(error).__name__ + ': ' + str(error) + '\\n')\n    sys.exit(65)\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Validate, Run }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Failed, Validated, Launch, Executed(u32) }

// ------------------------=
// FUNC: repairable
// DESC: Admits source repair only for a reclaimed native validation process that reported the trusted syntax-error status.
// ------------------=
pub fn repairable(phase: Phase, exit: Option<i64>, reclaimed: bool) -> bool {
    phase == Phase::Validate && reclaimed && exit == Some(SYNTAX_ERROR_EXIT)
}

// ------------------------=
// FUNC: outcome
// DESC: Requires real zero exit, successful memory reclamation and an observed native execution identity before issuing receipts.
// ------------------=
pub fn outcome(phase: Phase, exit: Option<i64>, reclaimed: bool, launch: bool, executed: Option<u32>) -> Outcome {
    if exit != Some(0) || !reclaimed || executed.is_none_or(|id| id == 0) { return Outcome::Failed; }
    match phase {
        Phase::Validate => if launch { Outcome::Launch } else { Outcome::Validated },
        Phase::Run => match executed { Some(id) if id != 0 => Outcome::Executed(id), _ => Outcome::Failed },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: validation_and_execution_require_native_evidence
    // DESC: Rejects syntax errors, crashes, unreclaimed jobs and missing execution receipts without publishing false success.
    // ------------------=
    #[test]
    fn validation_and_execution_require_native_evidence() {
        assert_eq!(outcome(Phase::Validate, Some(0), true, false, Some(3)), Outcome::Validated);
        assert_eq!(outcome(Phase::Validate, Some(0), true, true, Some(3)), Outcome::Launch);
        for phase in [Phase::Validate, Phase::Run] {
            assert_eq!(outcome(phase, Some(1), true, true, Some(3)), Outcome::Failed);
            assert_eq!(outcome(phase, None, true, true, Some(3)), Outcome::Failed);
            assert_eq!(outcome(phase, Some(0), false, true, Some(3)), Outcome::Failed);
            assert_eq!(outcome(phase, Some(0), true, true, None), Outcome::Failed);
            assert_eq!(outcome(phase, Some(0), true, true, Some(0)), Outcome::Failed);
        }
        assert_eq!(outcome(Phase::Run, Some(0), true, true, None), Outcome::Failed);
        assert_eq!(outcome(Phase::Run, Some(0), true, true, Some(0)), Outcome::Failed);
        assert_eq!(outcome(Phase::Run, Some(0), true, true, Some(47)), Outcome::Executed(47));
        assert!(repairable(Phase::Validate, Some(SYNTAX_ERROR_EXIT), true));
        for code in [None, Some(0), Some(1), Some(125)] {
            assert!(!repairable(Phase::Validate, code, true));
        }
        assert!(!repairable(Phase::Run, Some(SYNTAX_ERROR_EXIT), true));
        assert!(!repairable(Phase::Validate, Some(SYNTAX_ERROR_EXIT), false));
    }

    // ------------------------=
    // FUNC: interpreter_validation_does_not_execute_user_source
    // DESC: Exercises the production validation argument against real host CPython as a syntax-semantics test, not native-runtime acceptance.
    // ------------------=
    #[test]
    fn interpreter_validation_does_not_execute_user_source() {
        use std::{fs, process::Command, time::{SystemTime, UNIX_EPOCH}};
        assert_eq!(std::env::var("INFINITY_BUILD_KIT_ACTIVE").as_deref(), Ok("1"));
        let root = std::env::current_dir().unwrap();
        let work = root.join("build/behavior-tests/python-validation").join(
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().to_string());
        fs::create_dir_all(&work).unwrap();
        let script = work.join("side_effect.py");
        let marker = work.join("executed");
        let invalid = work.join("invalid.py");
        fs::write(&script, format!("open({:?}, 'w').write('executed')\n", marker.to_str().unwrap())).unwrap();
        fs::write(&invalid, b"if True\n    pass\n").unwrap();
        let python = std::env::var_os("INFINITY_TEST_PYTHON").unwrap_or("python3".into());
        let validate = std::str::from_utf8(VALIDATE_SOURCE).unwrap();
        let good = Command::new(&python).args(["-I", "-B", "-c", validate]).arg(&script).output().unwrap();
        assert!(good.status.success());
        assert!(!marker.exists());
        let bad = Command::new(&python).args(["-I", "-B", "-c", validate]).arg(&invalid).output().unwrap();
        assert_eq!(bad.status.code(), Some(SYNTAX_ERROR_EXIT as i32));
        let missing = Command::new(&python).args(["-I", "-B", "-c", validate]).arg(work.join("missing.py")).output().unwrap();
        assert!(!missing.status.success());
        assert!(!repairable(Phase::Validate, missing.status.code().map(i64::from), true));
        assert!(!marker.exists());
        let run = Command::new(&python).args(["-I", "-B"]).arg(&script).output().unwrap();
        assert!(run.status.success());
        assert_eq!(fs::read(&marker).unwrap(), b"executed");
    }
}
