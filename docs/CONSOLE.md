# Infinity Console Language v1

Each interactive session owns an explicit `ConsoleNavigationContext.CurrentNamespaceRef`; `path`, `idir`, and native `cd` operate only on that state. There is no kernel or process CWD. Compatibility Shell Profiles are validated command mappings, and `pwd` resolves to `path` only while a profile that provides it is enabled.

The Infinity Console is a human interface to typed InfinityOS operations. It
is not a POSIX shell: it has no process launch, string expansion, file
descriptor, byte-stream pipe, or ambient storage authority.

```text
exact command / natural language / voice
                 |
          typed operation graph
                 |
       capability and consequence policy
                 |
                 IOP
                 |
          owning system service
                 |
            typed result
                 |
       Console / GUI / voice renderer
```

Deterministic commands are parsed before local AI. Natural language resolves
directly to the same operation IDs; it is never converted into Console text and
executed.

## Grammar and references

The canonical grammar is `DOMAIN ACTION [TARGET] [key=value ...]`:

```text
system status
device inspect device:storage0
object find type=document project=InfinityOS modified=last-week
project inspect project:InfinityOS
collection list
```

References use `obj:`, `project:`, `collection:`, `device:`, and `service:`.
A `/path` is a Namespace reference, not identity or authority. Display names
must be resolved by the owning service and cannot authorize access alone.

Values containing spaces use double quotes. Inside quotes, `\"` and `\\` are
reserved escapes for the operation's typed decoder. Booleans are `true` or
`false`; single-letter flag bundles are not canonical syntax. Deterministic
dates are limited to `today`, `yesterday`, `this-week`, `last-week`,
`last-30-days`, `before=YYYY-MM-DD`, and `after=YYYY-MM-DD`.

## Registry and discovery

The Domain Registry contains system, device, storage, object, namespace,
project, collection, service, runtime, capability, event, AI, model, agent, and
voice. Operation schemas declare description, arguments, input/result type,
stable IOP operation ID, capability class, side-effect class, and example.

`storage`, `storage inspect`, `help storage`, and `help object find` project
discovery from these schemas, so incomplete input provides useful guidance.

## Typed composition

`|>` creates a bounded typed graph:

```text
object find type=image modified=last-week
|> object filter project=InfinityOS
|> namespace move destination=/archive/infinityos
```

No stage consumes rendered text. Each edge is checked before execution.
`DeviceSet |> Namespace.Move` therefore produces `TypeMismatch` before either
stage runs. Graphs are currently bounded to four stages and eight arguments per
stage.

Results include `DeviceSet`, `ObjectSet`, `ProjectSet`, and `OperationPlan`.
Rendering is separate. `docs = object find ...` binds the result type, not a
string. A session may bind `@1` through `@16` to its latest ObjectSet; these
aliases are ephemeral and never become persistent identity.

## Policy and status

Operations declare QUERY, REVERSIBLE_CHANGE, DESTRUCTIVE_CHANGE,
SECURITY_CHANGE, or EXTERNAL_EFFECT. `plan <operation>` returns an
`OperationPlan`. A command token cannot grant authority or bypass OS-owned
confirmation.

- Registry, grammar, quoting, bounded dates, discovery/help, plans, variables,
  references, and graph type checking: **TESTED**.
- Single query dispatch and ObjectSet session binding: **TESTED IN VM**.
- Multi-stage execution after preflight: **SCAFFOLDED**. The graph is
  policy-visible, but only single-stage queries dispatch today.
- Transactions, conditions, event triggers, and full automation: **PLANNED**.
# Milestone 7 identity domains

The registry now includes `user`, `identity`, `machine`, `credential`,
`session`, `personal-space`, `ai-profile`, `voice-profile`, and `settings`.
Results remain typed internally. Credential secrets are intentionally excluded
from ordinary command arguments; the Console directs creation to masked private
entry instead of storing secrets in command history.
