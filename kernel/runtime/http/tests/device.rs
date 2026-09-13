use infinity_http::{
    device::EthernetQueue,
    request::{get, Error},
    smoltcp::{
        phy::{Device, RxToken, TxToken},
        time::Instant,
    },
};

#[test]
// ------------------------=
// FUNC: queues_preserve_frames_until_hardware_accepts
// DESC: Verifies FIFO capacity, receive bounds, transmit retries and no overwrite under backpressure.
// ------------------=
fn queues_preserve_frames_until_hardware_accepts() {
    let mut device = EthernetQueue::new();
    let now = Instant::from_millis(0);
    assert!(!device.ingest(&[0; 13]));
    assert!(!device.ingest(&[0; 1515]));
    for n in 0..4 {
        assert!(device.ingest(&[n; 60]));
    }
    assert!(!device.ingest(&[9; 60]));
    for n in 0..4 {
        let (rx, tx) = device.receive(now).unwrap();
        rx.consume(|bytes| assert_eq!(bytes, &[n; 60]));
        tx.consume(60, |bytes| bytes.fill(n + 10));
    }
    assert!(device.transmit(now).is_none());
    for n in 0..4 {
        assert_eq!(device.pending(), Some([n + 10; 60].as_slice()));
        assert_eq!(device.pending(), Some([n + 10; 60].as_slice()));
        device.transmitted();
    }
    assert_eq!(device.pending(), None);
    assert!(device.transmit(now).is_some());
}

#[test]
// ------------------------=
// FUNC: request_encoding_is_bounded_and_injection_safe
// DESC: Verifies exact request bytes and rejects injected headers without modifying the output buffer.
// ------------------=
fn request_encoding_is_bounded_and_injection_safe() {
    let mut output = [0; 256];
    let count = get("example.com", "/weather?units=f", &mut output).unwrap();
    assert_eq!(&output[..count], b"GET /weather?units=f HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n");
    let before = output;
    assert_eq!(
        get("evil\r\nHost: other", "/", &mut output),
        Err(Error::InvalidAuthority)
    );
    assert_eq!(
        get("example.com", "/\r\nInjected: yes", &mut output),
        Err(Error::InvalidTarget)
    );
    assert_eq!(output, before);
    assert_eq!(
        get("example.com", "/", &mut output[..2]),
        Err(Error::Capacity)
    );
    assert_eq!(output, before);
}
