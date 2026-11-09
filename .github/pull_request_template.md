## Summary

<!-- What does this PR do? One paragraph is enough for small changes. -->

## Related issue

Closes #<!-- issue number -->

## Changes

<!-- Bullet list of what changed. Focus on behavior, not file names. -->

-
-

## Testing

<!-- How did you verify this works? Which tests cover it? -->

- [ ] `cargo test --all --features testutils` passes
- [ ] `cargo clippy --all-targets --features testutils -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] New behavior is covered by tests (or explain why not)

## Breaking changes

<!-- Does this change any public API, CLI flags, or output format? If yes, describe the impact. -->

None / <!-- describe -->

## Checklist

- [ ] Doc comments updated for any changed public items
- [ ] CHANGELOG.md updated under `[Unreleased]`
- [ ] No `unwrap()` on user-controlled input in library code
