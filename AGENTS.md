# Active project

This is the new standalone Rust simulation core. The sibling C++ project is
research/reference material, not a runtime or build dependency.

- HEADLESS: no graphics, UI, animation output, particles or audio playback.
- Track only the main CRT rand stream. Ignore Unreal/CRI RNG unless it changes
  battle outcomes or correct CRT advancement. Preserve relevant conditional draws.
- Prioritize a running small battle and explicit supported behavior over general
  Unreal compatibility. Do not port the old interpreter/object system.
- Use structs/enums, small mutable state and shared immutable definitions.
- Reject unsupported data. Do not silently substitute guessed OT2 behavior.
- The opening Throné encounter runs under source-derived `ot2_opening_v1` rules.
  Keep `scaffold_v1` for synthetic tests. The mechanics and imported records are
  tested, but full live-game CRT alignment is not established; see
  docs/throne-opening.md and docs/opening-routine.md for coverage and the exact
  seed/exit boundary. Never claim manipulation
  accuracy from reproducible simulator output alone.
- Keep the existing workspace and old engine unchanged. Work in this repository.
