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
- The initial scaffold uses explicitly named scaffold rules. Never describe the
  demo damage/turn rules, curated AI JSON, or item core effects as verified complete
  OT2 behavior. Exact encounter rules and live CRT trace validation come later.
- Keep the existing workspace and old engine unchanged. Work in this repository.
