# Development workflow

## Branching model


We follow a Gitflow branching model for its clear history and its structured release management.

The branching model is structured as followed:

```
├── main
├── dev
    ├── hotfix
    │   ├── hotfix/bugfix1
    │   └── hotfix/bugfix2
    ├── feat
    │   ├── feat/feature1
    │   └── feat/feature2
    └── release
```

- `main`: Stores official release history, commits should be tagged with a version number (starting at v0.1). The branch must be protected
- `dev`: Integration branch, created from main. The branch must also be protected
- `release`: Once enough features in `develop`, fork a `release` branch off of `develop`, merge into `develop` and `main` when done. Naming convention is adjectives linked to honey texture
- `feat`: Created from `develop`, merges into `develop` when completed
- `hotfix`: When issue detected in `main` branch, create a hotfix branch from main, once completed, merge into both `develop` and `main`

[More info here](https://www.atlassian.com/git/tutorials/comparing-workflows/gitflow-workflow)

## Commit message convention

Should be structured as followed:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

With the following structural elements:

- **fix**: commit of `type` *fix* patches a bug in codebase
- **feat**: commit of `type` *feat*  introduces new feature to the codebase
- **BREAKING CHANGE**: commit with *BREAKING CHANGE* `footer` or appends *!* after type/scope introduces a breaking API change. Can be part of any type
- `types` other than *fix* and *feat* are allowed
  - Non exhaustive list: *build*, *doc*, *refactor*, *test* 
- `footers` other than *BREAKING CHANGE* should follow a `key: value` format

[More info](https://www.conventionalcommits.org/en/v1.0.0/#summary)

## Documenting

Keeping the codebase documented benefits every team member and future contributors. Do your best to keep clean and clear documentation for every feature you submit.

Documentation should follow rustdoc conventions:

> rustdoc comments supports markdown notation

**Commenting crates (at a global level):**
```
//! # Crate
//!
//! Comment describing the crate and how to use it
```

**Commenting modules, functions, structs, etc.:**
```
/// [short sentence explaining what it is]
/// [more detailed explanation]
/// [at least one code example that user can copy/paste to try it]
/// [even more advanced explanations if necessary]
```

Here are some examples of headings commonly used in documentation:

```
# Examples

How to use the function, if code blocks are added, `cargo test` will run these chunks of code too, so go for it !

# Panics

Scenarios in which function could panic, so user of the function knows in which situation not to use it

# Errors

If function returns a `Result`, describe here the kinds of errors that might occur and what conditions might cause those errors to be returned.

# Safety

If function is `unsafe` explain why
```

### Generate and access the documentation in the nice way

To generate the documentation based off of those comments simply run `cargo doc`. Add `--open` to simultaneously open your browser and browse the HTML formatted version of the documentation.


## Code review and pull requests

As we're working with a small team, handling very small PR is not manageable, try to make PRs as small as possible (so no full features at once), but avoid PRs of less than 50 lines of code. Aim is to point out issues ASAP without overloading the team with review duties

- Every PR should be reviewed and validated by an other person than the one opening the PR. We follow a *squash and merge* logic so small commits are taken as one
- Every PR from `feature` into `develop` should be reviewed by at least one other team member
- Every change made to `main` or `release` branches should be reviewed by all team members
- Rotate reviewers on every other PR so the team keeps a global overview of the project
- Depending on the type of PRs (feature addition, bugfix, documentation update) the corresponding template should be used.

[More info](https://blog.mergify.com/pull-request-review-best-practices-code-excellence/)


Actual templates lie in `.github/pull_request_template/` and can be used directly when opening a PR by adding the corresponding query parameter to the URL:
- `?template=feature.md` for feature addition
- `?template=bugfix.md` for bugfixes
- `?template=documentation.md` for documentation updates

>[!INFO]
>
> If adding the query parameter in the URL doesn't work for you, verify that no other attributes are found at the end of the URL. If that's the case simply remove it and replace it with the *template* one

## Testing expectations

As Rust integrates testing well, adopting a test-driven development process should benefit keeping error rate low

*Reminder:* You start by writing a test that won't pass (feature not implemented), then you implement the minimal code to make the test pass, then you refactor and add whatever while keeping tests passing

- In optimal situation, every Rust file should contain a unit testing section
- Integration testing should be done at the same level as the binary code (so not in the lib)
- Minimal should be integration testing to prevent error propagation
- Performance testing must be done by integrating tools in the source code and
using conditional compilation to run them only in debug mode.
