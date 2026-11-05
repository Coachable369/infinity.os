# C application agent instructions

Read `/InfinitySDK/AGENTS.md` first. Select a C example from
`/InfinitySDK/Examples/catalog.json` and inspect its referenced headers.

- Use `<infinity/app.h>` for grid applications or documented UIKit/interaction
  APIs for custom layout. Do not redefine SDK types or helper functions.
- Check service results, validate IDs and arguments, bound buffers and integer
  arithmetic, and preserve partial I/O. Native C is not a POSIX environment.
- Use `app_run` to preserve yielding, close handling and AI acknowledgement;
  render current state and offer only legal actions. Share input handlers.
- Compile with the installed native toolchain through the editor's Run flow.
  Inspect diagnostics and correct source before launching. Preserve source and
  distinguish successful compilation from successful runtime behavior.
- Network operations are asynchronous: poll/yield with bounded waits, handle
  short reads/writes and close handles on errors. Never confuse queued work with
  completion or weaken TLS. Use the explicit `--allow-network` launch grant.
- Camera access requires authorization. The camera sample demonstrates the
  public API, not a facial-authentication implementation.
- Keep architecture differences out of app logic. Verify both targets before
  advertising both as tested.
