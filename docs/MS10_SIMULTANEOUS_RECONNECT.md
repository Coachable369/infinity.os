# Cold reconnect collision

The installed failure reported wire stage 8 (`SessionResponse`) and error 26
(`UnsupportedState`) on two trusted peers. Both persistent Pool publishers
initiated reconnect before receiving the other offer. The receiver previously
rejected every incoming session-init while awaiting its own session response.

A focused production-wire host fixture reproduced both endpoints remaining in
that state after sixteen simulated seconds. The correction only handles this
authenticated simultaneous-open state: the lower NodeId owns the winning
proposal. The other endpoint retires its proposal and responds without extending
its original deadline. The winner records the losing transaction in the existing
bounded replay history so a delayed replay cannot replace the established session.

Signature, peer identity, pairing receipt and scope validation precede resolution.
No new trust, authority, queue space, retries, or timeouts are introduced.

Behavioral verification covers both offer delivery orders, forged offers,
duplicate winning offers, repeated losing offers, convergence to one protocol
reference before the original deadline, established-session preservation and
actual application bytes. Installed cold-resume acceptance still requires the
rebuilt generation; host reproduction alone is not installed completion.
