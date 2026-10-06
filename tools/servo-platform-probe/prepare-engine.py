"""Keep native engine clocks and object authority separate from host OS APIs."""
import importlib.util
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Stages explicit native monotonic time and rejects unsupported filesystem URL conversions.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    servo = root / "build/servo-port-audit"
    clipboard_spec = importlib.util.spec_from_file_location("clipboard_overlay", Path(__file__).with_name("prepare-clipboard.py"))
    clipboard_overlay = importlib.util.module_from_spec(clipboard_spec)
    clipboard_spec.loader.exec_module(clipboard_overlay)
    spec = importlib.util.spec_from_file_location("native", Path(__file__).with_name("prepare-mio.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    relative = "components/script/dom/node/node.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    anchor = "        selection.collapse_to_dom_position(cx, &container, offset);"
    if text.count(anchor) != 1:
        raise RuntimeError("Selection default-action integration changed")
    text = text.replace(anchor, anchor + "\n" + Path(__file__).with_name("document-multiclick.rs").read_text())
    (servo / relative).write_text(text)
    relative = "components/script/dom/html/form_controls/text_input.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace("3 if button == MouseButton::Primary => {",
                        "3..=i32::MAX if button == MouseButton::Primary => {")
    (servo / relative).write_text(text)
    relative = "components/url/lib.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = helper.native_body(text, 'pub fn to_file_path(&self)', 'Err(UrlError::ToFilePath)')
    text = helper.native_body(text, 'pub fn from_file_path<P:', 'let _ = path; Err(UrlError::FromFilePath)')
    (servo / relative).write_text(text)

    # Intercept HTTP at the transport boundary, after request policy and cookie
    # selection. Upper fetch layers retain redirects, CORS and cache semantics.
    relative = "components/net/fetch/methods.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace('''    // Intercept the request and maybe override the response.
    context
        .request_interceptor
        .lock()
        .await
        .intercept_request(request, &mut response, context)
        .await;''', '''    #[cfg(infinity_native)]
    let intercept_here = !matches!(current_scheme, "http" | "https");
    #[cfg(not(infinity_native))]
    let intercept_here = true;
    if intercept_here {
        context.request_interceptor.lock().await
            .intercept_request(request, &mut response, context).await;
    }''')
    (servo / relative).write_text(text)
    relative = "components/net/http_loader.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace('''    let mut response_end_timer = ResponseEndTimer(Some(context.timing.clone()));''', '''    #[cfg(infinity_native)]
    {
        let request = &mut fetch_params.request;
        let mut response = None;
        context.request_interceptor.lock().await
            .intercept_request(request, &mut response, context).await;
        let Some(mut response) = response else {
            return Response::network_error(NetworkError::ConnectionFailure);
        };
        response.referrer = request.referrer.to_url().cloned();
        response.referrer_policy = request.referrer_policy;
        if credentials_flag {
            set_cookies_from_headers(&request.current_url(), &response.headers, &context.state.cookie_jar);
        }
        context.state.hsts_list.write().update_hsts_list_from_response(
            &request.current_url(), &response.headers);
        context.timing.set_attribute(ResourceAttribute::ResponseEnd);
        return response;
    }
    let mut response_end_timer = ResponseEndTimer(Some(context.timing.clone()));''')
    (servo / relative).write_text(text)
    relative = "components/net/request_interceptor.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace("url: request.url().into_url(),", '''url: {
                #[cfg(infinity_native)]
                { request.current_url().clone().into_url() }
                #[cfg(not(infinity_native))]
                { request.url().into_url() }
            },''')
    text = text.replace("WebResourceResponseMsg::CancelLoad => {", '''WebResourceResponseMsg::FailLoad => {
                    *response = Some(Response::network_error(NetworkError::ConnectionFailure));
                    break;
                },
                WebResourceResponseMsg::CancelLoad => {''')
    (servo / relative).write_text(text)

    # A provider failure is not an intentional superseding-navigation cancel.
    # Preserve that distinction so Servo finishes its failed-document lifecycle.
    relative = "components/shared/embedder/lib.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace("    CancelLoad,", "    CancelLoad,\n    FailLoad,")
    (servo / relative).write_text(text)
    relative = "components/servo/webview_delegate.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace("    pub fn cancel(mut self) {", '''    // ------------------------=
    // FUNC: fail
    // DESC: Completes a failed provider request without treating it as intentional navigation cancellation.
    // ------------------=
    pub fn fail(mut self) {
        if let Err(error) = self.response_sender.send(WebResourceResponseMsg::FailLoad) {
            self.error_sender.raise_response_send_error(error);
        }
        self.finished = true;
    }
    pub fn cancel(mut self) {''')
    (servo / relative).write_text(text)

    # Native execution is in-process. Never link a Unix sandbox or attempt
    # fork/exec when an upstream caller accidentally enables multiprocess.
    for relative in ("components/constellation/Cargo.toml", "components/servo/Cargo.toml"):
        text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
        text = text.replace('all(not(target_os = "windows"),',
            'all(not(target_os = "none"), not(target_os = "windows"),')
        (servo / relative).write_text(text)
    relative = "components/constellation/sandboxing.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace('not(target_os = "windows"),',
        'not(target_os = "none"), not(target_os = "windows"),')
    text = text.replace('#[cfg(any(\n    target_os = "windows",',
        '#[cfg(all(not(target_os = "none"), any(\n    target_os = "windows",')
    text = text.replace('all(target_arch = "aarch64", not(target_os = "macos"))\n))]',
        'all(target_arch = "aarch64", not(target_os = "macos"))\n)))]')
    text = text.replace('target_arch = "riscv64"\n))]\npub fn spawn_multiprocess',
        'target_arch = "riscv64"\n)))]\npub fn spawn_multiprocess')
    text += '''
#[cfg(target_os = "none")]
// ------------------------=
// FUNC: content_process_sandbox_profile
// DESC: Rejects unsupported process sandbox initialization on the native platform.
// ------------------=
pub fn content_process_sandbox_profile() {
    panic!("Native browser requires in-process execution");
}
#[cfg(target_os = "none")]
// ------------------------=
// FUNC: spawn_multiprocess
// DESC: Fails closed without attempting host process creation.
// ------------------=
pub fn spawn_multiprocess(_: UnprivilegedContent)
    -> Result<crate::process_manager::Process, ipc_channel::IpcError> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported,
        "Native browser requires in-process execution").into())
}
'''
    (servo / relative).write_text(text)

    relative = "components/servo/servo.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace('not(target_os = "windows"),',
        'not(target_os = "none"), not(target_os = "windows"),')
    text = text.replace('#[cfg(any(\n    target_os = "windows",',
        '#[cfg(any(\n    target_os = "none",\n    target_os = "windows",')
    (servo / relative).write_text(text)

    # A closed in-process receiver must be retired just like an IPC receiver.
    # Otherwise ResourceManager spins on the profiler channel after its exit,
    # starving every other continuation on the native owner CPU.
    relative = "components/shared/base/generic_channel/generic_channelset.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = text.replace("Crossbeam(Vec<crossbeam_channel::Receiver<Result<T, SendError>>>)",
        "Crossbeam(Vec<crossbeam_channel::Receiver<Result<T, SendError>>>, Vec<bool>)")
    text = text.replace("GenericReceiverSetVariants::Crossbeam(vec![])", "GenericReceiverSetVariants::Crossbeam(vec![], vec![])")
    text = text.replace("GenericReceiverSetVariants::Crossbeam(_)", "GenericReceiverSetVariants::Crossbeam(_, _)")
    text = text.replace("GenericReceiverSetVariants::Crossbeam(receivers)", "GenericReceiverSetVariants::Crossbeam(receivers, closed)")
    text = text.replace("receivers.push(receiver);", "receivers.push(receiver);\n                closed.push(false);")
    text = text.replace('''for receiver in receivers.iter() {
                    sel.recv(receiver);
                }''', '''let mut ids = Vec::new();
                for (id, receiver) in receivers.iter().enumerate() {
                    if !closed[id] {
                        sel.recv(receiver);
                        ids.push(id);
                    }
                }''')
    text = text.replace("receivers: receivers.as_slice(),\n                    sel,",
        "receivers: receivers.as_slice(),\n                    closed: closed.as_mut_slice(),\n                    ids,\n                    sel,")
    text = text.replace("sel: crossbeam_channel::Select<'a>,",
        "sel: crossbeam_channel::Select<'a>,\n        closed: &'a mut [bool],\n        ids: Vec<usize>,")
    text = text.replace("SelectorInner::Crossbeam { receivers, sel }", "SelectorInner::Crossbeam { receivers, sel, closed, ids }")
    text = text.replace("let index = selected.index();", "let operation = selected.index();\n                let index = ids[operation];")
    text = text.replace("Err(_) => GenericSelectionResult::ChannelClosed(index as u64),", '''Err(_) => {
                            closed[index] = true;
                            sel.remove(operation);
                            GenericSelectionResult::ChannelClosed(index as u64)
                        },''')
    (servo / relative).write_text(text)

    # Restore temporary diagnostic-only edits from earlier probe runs.
    for relative in ("components/script/event_loop/script_thread.rs", "components/constellation/constellation.rs"):
        text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
        (servo / relative).write_text(text)

    # The beta owns session storage in memory. Do not manufacture a Unix temp
    # directory before selecting the upstream memory implementations.
    for component, handle, engine, thread_body in (
        ("client_storage", "ClientStorageThreadHandle", "SqliteEngine::memory().expect(\"Native session registry initialization failed\")",
         "ClientStorageThread::new(sender_clone, generic_receiver, engine).start();"),
        ("cache_storage", "CacheStorageThreadHandle", "MemCacheStorageEngine { name_to_cache_map: Default::default() }",
         "let mut worker = CacheStorageThread::new(sender_clone, generic_receiver, engine); worker.start();"),
    ):
        relative = "components/storage/" + component + ".rs"
        text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
        signature = "fn new(config_dir: Option<PathBuf>, temporary_storage: bool) -> " + handle
        body = '''assert!(config_dir.is_none(), "Native persistent profile storage is not available");
            let _ = temporary_storage;
            let (generic_sender, generic_receiver) = generic_channel::channel().unwrap();
            let sender_clone = generic_sender.clone();
            thread::Builder::new().name("NativeSessionStorage".to_owned()).spawn(move || {
                let engine = ''' + engine + ''';
                ''' + thread_body + '''
            }).expect("Native session storage thread unavailable");
            ''' + handle + '''::new(generic_sender)'''
        text = helper.native_body(text, signature, body)
        (servo / relative).write_text(text)

    relative = "components/net/disk_cache.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = helper.native_body(text, "fn storage_dir()", "None")
    (servo / relative).write_text(text)

    relative = "components/script/dom/navigator/navigatorinfo.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text += '''
#[cfg(all(target_os = "none", infinity_native))]
#[expect(non_snake_case)]
// ------------------------=
// FUNC: Platform
// DESC: Identifies the native platform consistently in windows and web workers.
// ------------------=
pub(crate) fn Platform() -> DOMString {
    DOMString::from_static("InfinityOS")
}
'''
    (servo / relative).write_text(text)
    relative = "components/shared/base/cross_process_instant.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text += '''
#[cfg(all(target_os = "none", infinity_native))]
mod platform {
    #[expect(unsafe_code)]
    unsafe extern "C" {
        fn infinity_std_clock(clock: u32, seconds: *mut u64, nanos: *mut u32) -> i32;
    }

    // ------------------------=
    // FUNC: now
    // DESC: Reads the native monotonic epoch shared by all engine workers; never substitutes wall time.
    // ------------------=
    #[expect(unsafe_code)]
    pub(super) fn now() -> u64 {
        let mut seconds = 0;
        let mut nanos = 0;
        // SAFETY: Both output pointers are live local values; the provider validates its owner.
        let result = unsafe { infinity_std_clock(0, &mut seconds, &mut nanos) };
        assert_eq!(result, 0, "native monotonic clock unavailable");
        seconds.checked_mul(1_000_000_000).and_then(|n| n.checked_add(nanos as u64))
            .expect("native monotonic clock overflow")
    }
}
'''
    (servo / relative).write_text(text)


    clipboard_overlay.apply(servo)


if __name__ == "__main__":
    main()
