# Cinematography IR domain language

- **Source IR** — authored semantic cinematography intent: story, blocking, framing, camera operations, continuity, lighting, and transitions.
- **Solved Camera IR** — compatibility representation containing sampled camera and subject world-space state.
- **Compiled Guidance IR** — the shared compiler result consumed by views, prompts, execution adapters, and comparison. It retains authored intent while adding phases, world tracks, screen tracks, executable constraints, tolerances, prohibitions, and edit relations.
- **Shot phase** — a half-open interval inside a take or shot with a stable purpose such as hold, motion, focus handoff, reveal, or settle.
- **Screen track** — per-frame observable image-space state for a subject, including normalized bounds, center, visibility, depth, and focus state.
- **Shot constraint** — one typed requirement whose requested and actual values are shared by solve, view, prompt, execute, and compare.
- **Take** — camera coverage over performance/story time before editorial selection.
- **Edit clip** — a mapping from a take's source range into the final edit timeline, including transition and match metadata.
- **Execution profile** — backend and video-model capability policy that selects only supported guidance channels and encodings.
- **Shot bundle** — a transactional, self-describing output package containing guidance, prompts, review artifacts, control passes, metadata, manifests, and a completion marker.
