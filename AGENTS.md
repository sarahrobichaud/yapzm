# Code Style Guidelines

Apply these rules when writing or editing source files.

## Comments

- Keep at most one short sentence at the top of a file describing what it is. Remove long narration comments.
- Every function gets a comment of one concise sentence max. State the contract or intent — never narrate the implementation.
- Only add inline comments for legitimate non-obvious edge cases.

## Comment content

- Keep specifics out of comments: env var names, example values, paths, platform behavior. Anything already visible in the code below the comment doesn't need restating.
- Details owned by other code (e.g. where a name is resolved into a path) belong there, not here. State the contract in general terms; the code is the source of truth for specifics.

## Organization

- Group functions, classes, and other definitions by role or domain.
- Logic may be broken up or consolidated to facilitate readability, as long as behavior is preserved.

## Code

- Write expressive, descriptive code that prioritizes readability over being concise or clever.

## Testing

- Only test load-bearing ideas: contracts other code relies on, safety guarantees, and non-obvious parsing or translation logic. Skip tests for trivial wrappers, third-party behavior, and cases another test already covers.
- Name each test as a plain-language statement of the behavior it protects, so the test list reads like a specification.
- Write tests to read top to bottom: set up the scenario, act, then assert. Avoid `Result` plumbing; helpers panic on setup failures.
- Exercise behavior through the most meaningful entry point available, such as the function that reads from disk rather than its inner parser.
- Use the shared fixtures in `src/testing.rs` for filesystem setup and common test data. When a setup step would be repeated across modules, add a helper there instead of rolling it inline.
- Tests must not leave anything behind on disk or depend on the machine they run on.

## Environment

- Run Clippy and rustfmt through the Nix development shell: `nix develop -c cargo clippy` and `nix develop -c cargo fmt`.

## Repository layout

- `proto/` contains a prototype used for exploration and reference.
- `src/` contains the real implementation. New work goes here, not in the prototype.
