## What does this change?

<!-- One or two sentences. Link the issue it belongs to: "Fixes #123". -->

## How was it tested?

<!-- New or changed tests, and anything you tried by hand. For a change on
     screen, a picture taken with `pencraft --screenshot shot.png`. -->

## Checklist

- [ ] `scripts/check.sh` (or `scripts\check.ps1`) passes in `app/`: format, clippy with warnings as errors, tests
- [ ] Tests cover the change
- [ ] No new `#[allow]`, `unwrap`, `expect` or `panic!` in shipped code
- [ ] No new dependency, or it was agreed on in the issue first
- [ ] `CHANGELOG.md` has a line under "Unreleased", if users will notice the change
