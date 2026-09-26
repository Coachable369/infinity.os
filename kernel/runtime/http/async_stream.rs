//! Cooperative adapter: the native NIC service pumps packets; TLS never spins waiting for IO.
use crate::transport::{Error, Transport};
use core::{
    cell::RefCell,
    future::poll_fn,
    task::{Poll, Waker},
};
use embedded_io_async::{ErrorKind, ErrorType, Read, Write};
use smoltcp::{phy::Device, time::Instant};

pub struct Session<'a> {
    transport: RefCell<Transport<'a>>,
    reader: RefCell<Option<Waker>>,
    writer: RefCell<Option<Waker>>,
}
pub struct Stream<'s, 'a>(&'s Session<'a>);
impl<'a> Session<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Wraps one already-authorized native TCP connection without heap allocation or host sockets.
    // ------------------=
    pub fn new(transport: Transport<'a>) -> Self {
        Self {
            transport: RefCell::new(transport),
            reader: RefCell::new(None),
            writer: RefCell::new(None),
        }
    }
    // ------------------------=
    // FUNC: stream
    // DESC: Borrows the session exclusively for one TLS transaction so readers cannot steal each other's bytes.
    // ------------------=
    pub fn stream(&mut self) -> Stream<'_, 'a> {
        Stream(self)
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Advances the bounded native stack and wakes only IO that can complete or fail, not every timer tick.
    // ------------------=
    pub fn poll(&self, device: &mut impl Device, now: Instant) {
        self.transport.borrow_mut().poll(device, now);
        let readiness = self.transport.borrow().readiness();
        let reader = if readiness.readable { self.reader.borrow_mut().take() } else { None };
        let writer = if readiness.writable { self.writer.borrow_mut().take() } else { None };
        // Invoke callbacks after releasing RefCell guards; waking can re-enter an executor.
        if let Some(waker) = reader {
            waker.wake();
        }
        if let Some(waker) = writer {
            waker.wake();
        }
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Stops the TCP stream and wakes TLS promptly when permission is revoked or the user cancels.
    // ------------------=
    pub fn cancel(&self) {
        self.transport.borrow_mut().cancel();
        let reader = self.reader.borrow_mut().take();
        let writer = self.writer.borrow_mut().take();
        if let Some(waker) = reader {
            waker.wake();
        }
        if let Some(waker) = writer {
            waker.wake();
        }
    }
}
impl<'s, 'a> Stream<'s, 'a> {
    // ------------------------=
    // FUNC: session
    // DESC: Exposes the shared pump handle while the transaction exclusively owns stream IO.
    // ------------------=
    pub fn session(&self) -> &'s Session<'a> {
        self.0
    }
}
impl ErrorType for Stream<'_, '_> {
    type Error = ErrorKind;
}
impl Read for Stream<'_, '_> {
    // ------------------------=
    // FUNC: read
    // DESC: Suspends on empty TCP input and preserves cancellation and hard-deadline failures.
    // ------------------=
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, ErrorKind> {
        poll_fn(|cx| match self.0.transport.borrow_mut().receive(bytes) {
            Ok(count) => Poll::Ready(Ok(count)),
            Err(Error::WouldBlock) => {
                *self.0.reader.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
            Err(_) => Poll::Ready(Err(ErrorKind::ConnectionAborted)),
        })
        .await
    }
}
impl Write for Stream<'_, '_> {
    // ------------------------=
    // FUNC: write
    // DESC: Suspends rather than polling in a loop when the native TCP send buffer is full.
    // ------------------=
    async fn write(&mut self, bytes: &[u8]) -> Result<usize, ErrorKind> {
        if bytes.is_empty() {
            return Ok(0);
        }
        poll_fn(|cx| match self.0.transport.borrow_mut().send(bytes) {
            Ok(count) => Poll::Ready(Ok(count)),
            Err(Error::WouldBlock) => {
                *self.0.writer.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
            Err(_) => Poll::Ready(Err(ErrorKind::ConnectionAborted)),
        })
        .await
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Leaves queued TCP delivery to the native pump; writes are already retained in the transport's bounded buffer.
    // ------------------=
    async fn flush(&mut self) -> Result<(), ErrorKind> {
        Ok(())
    }
}
