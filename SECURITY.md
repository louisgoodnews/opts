# Security Policy

## Supported Versions

OPTS is in early development. Only the `master` branch is supported. There are no stable releases yet.

## Reporting a Vulnerability

If you believe you have found a security issue, please do **not** open a public issue. Instead, use GitHub's private vulnerability reporting:

https://github.com/louisgoodnews/opts/security/advisories/new

Or send a private message to a repository maintainer.

## Scope

Security issues for OPTS include:

- Parser crashes or memory issues that a malicious asset can trigger (see below).
- Code execution from loaded content.
- Anything that compromises a user's system via OPTS.

The following are **not** security issues in OPTS:

- Bugs in third-party OMSI assets. Report those to their authors.
- Any behaviour of `OMSI.exe` itself. That's a separate product.
- Visual glitches. Those are ordinary bugs.

## Expected Response

This is a volunteer project. We aim to acknowledge reports within one week and provide an assessment within two. We do not guarantee a fix timeline.

## Known Content Risks

OPTS reads untrusted content from user-supplied OMSI installations. All parsers are designed to fail safely on malformed input. If you find a case where they don't, that's a security issue.
