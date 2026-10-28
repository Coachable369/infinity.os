//! Ordered evidence for generated Python validation and execution.

// Only this trusted validation wrapper emits EX_DATAERR for source syntax.
// Import/startup, missing-input, resource and runtime failures keep their own
// status and must never trigger a model rewrite of valid user source.
pub const SYNTAX_ERROR_EXIT: i64 = 65;
pub const DEPENDENCY_ERROR_EXIT: i64 = 66;
pub const INCOMPLETE_APPLICATION_EXIT: i64 = 67;
pub const VALIDATE_SOURCE: &[u8] = b"import sys\nfrom _infinity_modules import validate_source, DependencyError\ntry:\n    validate_source(open(sys.argv[1], 'rb').read(), sys.argv[1])\nexcept SyntaxError as error:\n    sys.stderr.write(type(error).__name__ + ': ' + str(error) + '\\n')\n    sys.exit(65)\nexcept DependencyError as error:\n    for issue in error.issues:\n        sys.stderr.write('%s:%d:%d: %s: %s\\n' % (issue['filename'], issue['line'], issue['column'], issue['module'], issue['reason']))\n    sys.exit(66)\n";
pub const VALIDATE_GENERATED_APPLICATION: &[u8] = b"import sys\nfrom _infinity_modules import validate_generated_application, DependencyError, IncompleteApplicationError\ntry:\n    validate_generated_application(open(sys.argv[1], 'rb').read(), sys.argv[1])\nexcept SyntaxError as error:\n    sys.stderr.write(type(error).__name__ + ': ' + str(error) + '\\n')\n    sys.exit(65)\nexcept DependencyError as error:\n    for issue in error.issues:\n        sys.stderr.write('%s:%d:%d: %s: %s\\n' % (issue['filename'], issue['line'], issue['column'], issue['module'], issue['reason']))\n    sys.exit(66)\nexcept IncompleteApplicationError as error:\n    for issue in error.issues:\n        sys.stderr.write('%s:%d:%d: %s: %s\\n' % (issue['filename'], issue['line'], issue['column'], issue['kind'], issue['reason']))\n    sys.exit(67)\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Validate, Run }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Failed, Validated, Launch, Executed(u32) }

// ------------------------=
// FUNC: authorized
// DESC: Keeps preparation bound to its editor snapshot but transfers a proven launched process to session authority.
// ------------------=
pub fn authorized(phase:Phase,launched:bool,snapshot_current:bool,session_current:bool)->bool{
    session_current&&if phase==Phase::Run&&launched{true}else{snapshot_current}
}

// ------------------------=
// FUNC: repairable
// DESC: Admits source repair only for reclaimed generated validation jobs reporting trusted syntax, dependency or whole-placeholder errors.
// ------------------=
pub fn repairable(phase: Phase, exit: Option<i64>, reclaimed: bool) -> bool {
    phase == Phase::Validate && reclaimed && matches!(exit,
        Some(SYNTAX_ERROR_EXIT | DEPENDENCY_ERROR_EXIT | INCOMPLETE_APPLICATION_EXIT))
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
        assert!(!authorized(Phase::Validate,false,false,true));
        assert!(!authorized(Phase::Run,false,false,true));
        assert!(authorized(Phase::Run,true,false,true));
        assert!(!authorized(Phase::Run,true,true,false));
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
        assert!(repairable(Phase::Validate, Some(DEPENDENCY_ERROR_EXIT), true));
        assert!(repairable(Phase::Validate, Some(INCOMPLETE_APPLICATION_EXIT), true));
        assert!(!repairable(Phase::Validate, Some(INCOMPLETE_APPLICATION_EXIT), false));
        assert!(!repairable(Phase::Run, Some(INCOMPLETE_APPLICATION_EXIT), true));
        assert_eq!(outcome(Phase::Validate, Some(INCOMPLETE_APPLICATION_EXIT), true, true, Some(3)), Outcome::Failed);
        assert!(!repairable(Phase::Run, Some(DEPENDENCY_ERROR_EXIT), true));
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
        let unavailable = work.join("unavailable.py");
        fs::write(&script, format!("open({:?}, 'w').write('executed')\n", marker.to_str().unwrap())).unwrap();
        fs::write(&invalid, b"if True\n    pass\n").unwrap();
        fs::write(&unavailable, b"import missing_infinity_fixture_module\n").unwrap();
        let python = std::env::var_os("INFINITY_TEST_PYTHON").unwrap_or("python3".into());
        // The host fixture explicitly supplies inventory metadata; production
        // always imports the frozen inventory from the native interpreter.
        let validate = format!("import sys, types\nm=types.ModuleType('_infinity_inventory')\nm.INVENTORY={{'limitations':{{}},'required_backends':{{}}}}\nsys.modules[m.__name__]=m\nsys.path.insert(0,{:?})\n{}",
            root.join("sdk/python").to_str().unwrap(), std::str::from_utf8(VALIDATE_SOURCE).unwrap());
        let good = Command::new(&python).args(["-I", "-B", "-c", &validate]).arg(&script).output().unwrap();
        assert!(good.status.success());
        assert!(!marker.exists());
        let bad = Command::new(&python).args(["-I", "-B", "-c", &validate]).arg(&invalid).output().unwrap();
        assert_eq!(bad.status.code(), Some(SYNTAX_ERROR_EXIT as i32));
        let dependency = Command::new(&python).args(["-I", "-B", "-c", &validate]).arg(&unavailable).output().unwrap();
        assert_eq!(dependency.status.code(), Some(DEPENDENCY_ERROR_EXIT as i32));
        let missing = Command::new(&python).args(["-I", "-B", "-c", &validate]).arg(work.join("missing.py")).output().unwrap();
        assert!(!missing.status.success());
        assert!(!repairable(Phase::Validate, missing.status.code().map(i64::from), true));
        assert!(!marker.exists());
        let run = Command::new(&python).args(["-I", "-B"]).arg(&script).output().unwrap();
        assert!(run.status.success());
        assert_eq!(fs::read(&marker).unwrap(), b"executed");
    }

    // ------------------------=
    // FUNC: generated_placeholder_lint_is_separate_from_manual_validation
    // DESC: Exercises trusted wrapper exits without executing candidates, preserving ordinary scripts and implemented no-op callbacks.
    // ------------------=
    #[test]
    fn generated_placeholder_lint_is_separate_from_manual_validation() {
        use std::{fs, process::Command, time::{SystemTime, UNIX_EPOCH}};
        assert_eq!(std::env::var("INFINITY_BUILD_KIT_ACTIVE").as_deref(), Ok("1"));
        let root=std::env::current_dir().unwrap();
        let work=root.join("build/behavior-tests/python-validation").join(
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().to_string());
        fs::create_dir_all(&work).unwrap();
        let stub=work.join("stub.py");let implemented=work.join("implemented.py");let marker=work.join("side-effect");
        fs::write(&stub,b"def main():\n    pass\nmain()\n").unwrap();
        fs::write(&implemented,format!("def callback():\n    pass\nopen({:?},'w').write('bad')\n",marker.to_str().unwrap())).unwrap();
        let setup=format!("import sys, types\nm=types.ModuleType('_infinity_inventory')\nm.INVENTORY={{'limitations':{{}},'required_backends':{{}}}}\nsys.modules[m.__name__]=m\nsys.path.insert(0,{:?})\n",
            root.join("sdk/python").to_str().unwrap());
        let python=std::env::var_os("INFINITY_TEST_PYTHON").unwrap_or("python3".into());
        for (wrapper,path,expected) in [
            (VALIDATE_SOURCE,&stub,0),(VALIDATE_GENERATED_APPLICATION,&stub,67),
            (VALIDATE_GENERATED_APPLICATION,&implemented,0)] {
            let validate=format!("{}{}",setup,std::str::from_utf8(wrapper).unwrap());
            let result=Command::new(&python).args(["-I","-B","-c",&validate]).arg(path).output().unwrap();
            assert_eq!(result.status.code(),Some(expected));
            assert!(!marker.exists());
        }
    }
}
