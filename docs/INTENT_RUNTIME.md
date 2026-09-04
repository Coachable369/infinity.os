# Infinity Intent Runtime (IIR)

Milestone 2 implements the smallest useful provider-neutral intent foundation:

```text
Console input
  -> exact command resolver
  -> built-in intent resolver (only if exact matching fails)
  -> ResolvedIntent / typed SystemOperation
  -> OperationPolicy boundary
  -> dispatcher
  -> bounded InfinityOS service view
```

Resolvers never execute operations. `ResolvedIntent` contains a closed
`SystemOperation` enum, resolution source, and canonical command. The dispatcher
is the only component that turns an authorized typed operation into output or
navigation. There is no arbitrary string execution or function-pointer lookup.

## Built-in intent phrases

The deliberately small resolver recognizes these groups:

- device list: `show connected devices`, `what hardware is connected`,
  `list my devices`, `show devices`, `what devices do you see`;
- system status: `how is the system doing`, `show system status`,
  `what is the system status`;
- system info: `tell me about this machine`, `show system information`,
  `what system is this`;
- memory status: `show memory status`,
  `how much memory information is available`.

Natural-language matches display their canonical interpretation before dispatch.
For example, `show connected devices` and exact `device list` both dispatch the
same `SystemOperation::DeviceList` and emit `[operation] device.list` to serial.

## Context and policy boundary

`IntentContext` currently carries console mode, architecture, known-device count,
and the `boot-console` environment. It deliberately has no working directory,
user, application, or filesystem namespace. `KnownOperationPolicy` currently
passes the closed enum through; it is an explicit insertion seam, not a claim that
a capability system already exists.

## Future providers

`IntentProvider` and `IntentProviderDescriptor` define the provider plugin seam.
A future local or remote provider can advertise capabilities and propose a
`ResolvedIntent`, but cannot dispatch it. No provider is registered in Milestone 2,
no model is embedded, and no external API is called.
