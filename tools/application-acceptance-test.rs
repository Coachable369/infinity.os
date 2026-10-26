#![allow(dead_code, unused_imports)]
#[path = "../kernel/ui/mod.rs"]
mod ui;
use ui::app_assistant::{Panel, editor_compile_intent::SourceLanguage,
    application_acceptance::{Contract, Criterion}};

// ------------------------=
// FUNC: original_contract_survives_repair
// DESC: Exercises original-request retention and all-or-nothing prompt assembly in both production language paths.
// ------------------=
#[test]
fn original_contract_survives_repair() {
    for (language, request, source) in [
        (SourceLanguage::C, b"Create a C calculator application".as_slice(), b"int main(void){return 0;}".as_slice()),
        (SourceLanguage::Python, b"Create a calculator in Python".as_slice(), b"print(0)".as_slice()),
    ] {
        let mut panel=Panel::new();
        panel.source_language=language;
        panel.input[..request.len()].copy_from_slice(request);panel.length=request.len();
        panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
        let contract=Contract::from_request(request).unwrap();
        assert_eq!(panel.acceptance_contract(),Some(contract));
        assert!(contract.criteria.contains(&Criterion::DivisionByZero));
        let mut output=[0xa5;6144];
        let capacity=output.len()-if language==SourceLanguage::Python {512} else {0};
        let length=panel.generation_prompt(true,b"",&mut output[..capacity]).unwrap();
        let mut short=vec![0xa5;length-1];
        assert_eq!(panel.generation_prompt(true,b"",&mut short),None);
        assert!(short.iter().all(|b|*b==0xa5));
        assert!(output[length..].iter().all(|b|*b==0xa5));
        panel.input[..12].copy_from_slice(b"Fix the code");panel.length=12;
        panel.repair_attempts=1;panel.repair_diagnostics[0]=1;panel.repair_diagnostics_len=1;
        assert_eq!(panel.acceptance_contract(),Some(contract));
        let length=panel.compiler_repair_prompt(source,&mut output[..capacity]).unwrap();
        let mut short=vec![0xa5;length-1];
        assert_eq!(panel.compiler_repair_prompt(source,&mut short),None);
        assert!(short.iter().all(|b|*b==0xa5));
        assert_eq!(panel.acceptance_contract().unwrap().request,request);
    }
}

// ------------------------=
// FUNC: supported_creation_contexts_fit
// DESC: Exercises production context capacity for both terminal and native-window applications without weakening requirements to make a prompt fit.
// ------------------=
#[test]
fn supported_creation_contexts_fit() {
    for request in [b"Create a graphical calculator application".as_slice(),
        b"Create a chess game", b"Create a graphical Python calculator application",
        b"Create a Python calendar application"] {
        let mut panel=Panel::new();
        panel.source_language=ui::app_assistant::editor_compile_intent::language(request);
        panel.input[..request.len()].copy_from_slice(request);panel.length=request.len();
        let contract=panel.acceptance_contract().unwrap();
        assert_eq!(contract.request,request);
        let mut output=[0;6144];
        let capacity=output.len()-if panel.source_language==SourceLanguage::Python {512} else {0};
        assert!(panel.generation_prompt(true,b"",&mut output[..capacity]).is_some());
    }
}
