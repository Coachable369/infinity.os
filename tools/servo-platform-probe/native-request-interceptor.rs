// Native transport interception uses Servo's normal streaming fetch channel.
#[cfg(infinity_native)]
impl RequestInterceptor {
    // ------------------------=
    // FUNC: intercept_streaming
    // DESC: Returns authenticated response headers immediately and delivers body chunks through Servo's cancellable fetch pipeline.
    // ------------------=
    pub async fn intercept_streaming(
        &self, request: &mut Request, response: &mut Option<Response>,
        done_chan: &mut crate::fetch::methods::DoneChannel, context: &FetchContext,
    ) {
        use crate::fetch::methods::Data;
        let payload = match tokio::time::timeout(std::time::Duration::from_secs(30), native_request_body(request,context)).await {
            Ok(Ok(body))=>body,
            _=>{*response=Some(Response::network_error(NetworkError::ConnectionFailure));return;},
        };
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        self.embedder_proxy.send(NetToEmbedderMsg::WebResourceRequested(
            request.target_webview_id,
            WebResourceRequest {
                method: request.method.clone(), body: payload, url: request.current_url().clone().into_url(),
                headers: request.headers.clone(), destination: request.destination,
                referrer_url: request.referrer.to_url().map(|url| url.as_url().clone()),
                is_for_main_frame: matches!(request.destination, Destination::Document),
                is_redirect: request.redirect_count > 0,
            }, sender,
        ));
        let Some(WebResourceResponseMsg::Start(head)) = receiver.recv().await else {
            *response = Some(Response::network_error(NetworkError::ConnectionFailure));
            return;
        };
        let mut result = Response::new(head.url.into(), context.timing.inner().clone());
        result.headers = head.headers;
        result.status = HttpStatus::new(head.status_code, head.status_message);
        *result.body.lock() = ResponseBody::Receiving(Vec::new());
        let body = result.body.clone();
        let (sender, receiver_done) = tokio::sync::mpsc::unbounded_channel();
        *done_chan = Some((sender.clone(), receiver_done));
        let cancellation = context.cancellation_listener.clone();
        let timing = context.timing.clone();
        *response = Some(result);
        crate::async_runtime::spawn_task(async move {
            let mut success = false;
            while let Some(message) = receiver.recv().await {
                if cancellation.cancelled() { break; }
                match message {
                    WebResourceResponseMsg::SendBodyData(chunk) => {
                        if let ResponseBody::Receiving(bytes) = &mut *body.lock() {
                            bytes.extend_from_slice(&chunk);
                        }
                        let _ = sender.send(Data::Payload(chunk.into()));
                    },
                    WebResourceResponseMsg::FinishLoad => { success = true; break; },
                    _ => break,
                }
            }
            let mut body = body.lock();
            let bytes = if success {
                match &mut *body { ResponseBody::Receiving(bytes) => std::mem::take(bytes), _ => Vec::new() }
            } else { Vec::new() };
            *body = ResponseBody::Done(bytes);
            if !success {
                let _ = sender.send(if cancellation.cancelled() { Data::Cancelled }
                    else { Data::Error(NetworkError::ConnectionFailure) });
            }
            timing.set_attribute(net_traits::ResourceAttribute::ResponseEnd);
            let _ = sender.send(Data::Done);
        });
    }
}

#[cfg(infinity_native)]
// ------------------------=
// FUNC: native_request_body
// DESC: Pulls bounded script-owned body chunks without blocking the network thread or holding its stream lock across awaits.
// ------------------=
async fn native_request_body(request:&Request,context:&FetchContext)->Result<Vec<u8>,()> {
    use net_traits::request::{BodyChunkRequest,BodyChunkResponse};
    let Some(body)=request.body.as_ref() else {return Ok(Vec::new());};
    if body.len().is_some_and(|length|length>65536) {return Err(());}
    let stream=body.clone_stream();
    let requester=stream.lock().clone().ok_or(())?;
    let (sender,receiver)=ipc_channel::ipc::channel::<BodyChunkResponse>().map_err(|_|())?;
    let (queue,mut incoming)=tokio::sync::mpsc::unbounded_channel();
    ipc_channel::router::ROUTER.add_typed_route(receiver,Box::new(move |message| {let _=queue.send(message);}));
    requester.send(BodyChunkRequest::Connect(sender)).map_err(|_|())?;
    let mut bytes=Vec::new();
    loop {
        if context.cancellation_listener.cancelled() {return Err(());}
        requester.send(BodyChunkRequest::Chunk).map_err(|_|())?;
        match incoming.recv().await.ok_or(())?.map_err(|_|())? {
            BodyChunkResponse::Chunk(chunk)=>{
                if chunk.len()>65536-bytes.len() {return Err(());}
                bytes.extend_from_slice(&chunk);
            },
            BodyChunkResponse::Done=>return Ok(bytes),
            BodyChunkResponse::Error=>return Err(()),
        }
    }
}
