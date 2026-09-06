# Liquid Hacker terminal experience

Liquid Hacker is Automexia's public visual language for the free terminal:
compact, readable, keyboard-first, and quiet enough to keep shell content
primary.

## Principles

- Terminal cells and the active prompt remain the visual priority.
- One pane has a clear active outline.
- Tabs, pane-local tabs, scrollbars, and passive status avoid covering PTY rows.
- Cyan, blue, purple, and coral accents are redundant with text, shape, and
  state; color alone never carries meaning.
- Motion is brief, non-repeating, and removable with reduced motion.
- Narrow layouts hide or compact secondary chrome before shrinking the terminal
  below a usable size.

## Layout contract

Terminal content owns the available grid. Chrome uses bounded geometry, never
overlaps PTY rows, clips inside the viewport, and keeps focus, hit targets,
selection, scrollback, overlays, and resize handles route-local. Tiny through
high-resolution layouts preserve a usable terminal before optional decoration.

## Windows, tabs, and panes

Global tabs belong to a window. Pane-local tabs belong to a pane and own
independent PTYs. Splits preserve clear boundaries, focus, and resize handles.
Fresh and cloned sessions remain visibly and behaviorally distinct.

## Prompt and command output

### Per-pane operational context

Supported shell integration may show bounded pane-scoped path, Git, status, and
duration metadata. It never reconstructs a command from rendered cells or
changes the shell's input.

Completed output can use restrained spacing, tint, rule, timestamp, or status
cues, but terminal text remains unchanged and selectable. Silent commands do not
create invented output regions.

## Search and overlays

Search, command palette, diagnostics, compatibility inspection, appearance
controls, image preview, and quit confirmation are explicit overlays. They own
input while open, stay within the viewport, restore focus when closed, and keep
terminal content visible where safe.

## Selection, clipboard, and mouse

### Keyboard selection

Keyboard and pointer selection agree on terminal cells and grapheme boundaries.
Copy is explicit. Paste never adds Enter. Mouse-reporting applications keep
their documented ownership, and left click never becomes an implicit paste.

## Images

Inline protocol images move and clip with their terminal state. Local preview
uses a bounded card with filename, dimensions, and size text. Images never carry
the only required meaning.

## Accessibility

All public surfaces support keyboard operation, visible focus, semantic names
and states, high contrast, reduced motion, long text, Unicode/IME, and high
scale. Native screen-reader evidence remains platform-specific.

## Privacy

Public chrome and diagnostics avoid terminal history, credentials, private
paths, account data, and hidden environment values. Errors are actionable but
redacted.

Unreleased product surfaces and commercial visual systems are private.
