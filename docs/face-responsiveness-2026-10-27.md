# Face setup responsiveness

The isolated worker batches short service traps for at most 4 ms and 32 calls.
Each native entry retains the existing timer preemption. Input dispatch runs
between batches; terminal workers are never reentered. Camera USB polling stops
after the captured RGB/preview pixels are copied, before inference. The next
preview reopens with the existing exposure-settling policy. Recognition criteria,
camera permissions, cancellation, and deadlines remain unchanged.

The existing canonical mesh now completes a continuous 12-second yaw cycle,
rather than reversing at approximately 60 degrees. Normalized sine/cosine
preserve rigid geometry. Elapsed pose progression tolerates frame intervals up
to 100 ms without slowing normal lower-frame-rate VMs; longer stalls remain
bounded. Existing scan illumination, antialiasing, size and reduced motion stay.

Focused evidence: production capture lifecycle and bounded scheduling tests;
real packaged YuNet/SFace output equivalence; production painter continuity,
full-turn geometry, damage bounds, repaint stability, and reduced-motion checks.
These are not installed-VM camera or single-click responsiveness acceptance.
Fresh-install packaging embeds the same worker and kernel renderer for each ISA.
