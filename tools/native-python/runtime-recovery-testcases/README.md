# Generic runtime-recovery acceptance fixtures

These are small buggy applications, not generated-app templates. Their independent
verification scenarios must never be included in model-editable source or changed
to accommodate a candidate. A model repair must preserve each public function and
its intended behavior. No repaired implementation is supplied here.

| Candidate | Real defect | Native original outcome | Independent checks |
| --- | --- | --- | --- |
| `invoice.py` | Integer conversion of a decimal unit-price string | Uncaught `ValueError` during ordinary main execution | Decimal totals, multiple invoices, quoted CSV field, empty invoice, invalid quantity |
| `calendar.py` | Mixed date/string sort keys for dated and undated events | Uncaught `TypeError` during ordinary main execution | Chronological order, stable ties, undated-last behavior, nonmutation, invalid date |

Neither defect uses a deliberate `raise` to manufacture a failure.

The complete bounded sequence is available as:

```sh
./build-kit run python3 tools/native-python/runtime-recovery-acceptance.py
```

It preserves each native receipt before the next probe and only reports success
when both independently checked applications pass. The stages below can also be
run separately for diagnosis. The real-model probe never receives the scenarios.

## Evidence sequence

1. Run the **original exact source** through the native Python process probe to
   capture its actual exception and diagnostic bytes. Construction or host syntax
   checks alone do not establish that either original fails natively.
2. Pass that saved source and actual diagnostics to the generic
   `--repair-runtime-source` model probe, together with the original application
   objective. Keep the trusted `*-scenario.py` file separate.
3. Construct a verification bundle for the **exact repaired source**:

   ```sh
   ./build-kit run python3 tools/native-python/runtime-recovery-check.py \
     --source build/REPAIR_RUN/source-after.py \
     --scenario tools/native-python/runtime-recovery-testcases/invoice-scenario.py \
     --output-dir build/UNIQUE_VERIFICATION_RUN
   ```

4. Run the constructed guest script using the native interpreter:

   ```sh
   ./build-kit run python3 tools/native-process-probe/run.py \
     --arch aarch64 \
     --compiler builds/toolchains/aarch64/python.elf \
     --python-script \
     --python-source build/UNIQUE_VERIFICATION_RUN/guest.py \
     --python-exit 0
   ```

Use the corresponding calendar scenario for calendar acceptance. A result only
passes when the actual native process exits successfully after the independent
functional checks; compilation, launch, or an LLM statement is insufficient.
The separate native process receipt must bind the wrapper SHA-256 from
`binding.json`. That wrapper binds both original candidate bytes and trusted
scenario bytes. The constructor leaves `domain_behavior_verified` false because
it never executes them.

The wrapper runs the candidate as `__main__`, supplies `/candidate.py` as its
diagnostic filename and `sys.argv[0]`, and makes it the actual `__main__` module.
It then invokes a separately compiled `verify(candidate_namespace, check)`
function. Native exit zero requires the scenario's declared minimum number of
satisfied checks. Main-program exceptions, failed assertions, absent checks,
hash mismatch, and non-native execution cannot pass.

Candidate and verifier snapshots are read-only and separate; model-produced
output only replaces the candidate. This is an acceptance harness, **not an
adversarial same-interpreter Python sandbox**. It does not prove installed-OS UI,
microphone input, or broad application correctness beyond the tested behaviors.

Constructor bounds/provenance tests:

```sh
./build-kit run python3 tools/native-python/runtime-recovery-check-test.py
```

Those host tests never execute candidate or scenario code. They execute only the
wrapper's early native-platform guard, which must exit with status 126.
