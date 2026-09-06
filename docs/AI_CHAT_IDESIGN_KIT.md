# InfinityOS AI Chat IDesign Kit

Status: IMPLEMENTED

## Surface

The desktop AI surface extends the existing right-side AI Status glass card. It uses the active skin's window outline, focus, selection, header, and widget colors so primary and secondary theme changes remain authoritative. No raster chrome is used.

## Layout

- Header: `AI CHAT`, local/ready state, minimize, close.
- Model control: one full-width semantic selector populated from installed chat models.
- Timeline: bounded user/assistant message cards with distinct alignment and restrained tint.
- Composer: high-contrast text field and a themed Send action.
- Minimized: header-only card that preserves a visible restore affordance.
- Disabled: no desktop card; System Settings remains the recovery and enablement path.

## Interaction states

- Hover: brighter outline and surface tint.
- Press: inset surface treatment.
- Keyboard: Tab moves between model, composer, Send, minimize, and close; Enter activates or submits.
- Close: persistently disables the boot panel.
- Minimize: session-only collapse; restoring does not alter the persistent preference.

## Resource limits

Conversation history is fixed-capacity and oldest-first evicted. Input and response buffers are bounded. Chat models are selected by typed identifiers and do not gain ambient system authority.
