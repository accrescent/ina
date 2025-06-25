# Contributing guidelines

Thank you for your interest in contributing to Ina! Here are a few resources to get you started.

## Building

Ina uses [dependency verification] for its Gradle dependencies. If you run into a build error
related to dependency verification and aren't developing on Linux, temporarily delete
`gradle/verification-metadata.xml` and try again. Only the Linux version of aapt2 is represented in
the verification metadata, so it may cause build errors if you're running another operating system.

## Code style

Rust code style is enforced via `rustfmt` in CI, so you should automatically see whether your code
conforms after making a pull request.

Kotlin code style is not fully enforced in CI, but has the following guidelines:

- Wrap lines at 100 columns. This isn't a hard limit, but will be enforced unless wrapping a line
  looks uglier than extending it by a few columns.
- Don't use glob imports. You can have Android Studio create single name imports automatically by
  going to `File -> Settings -> Editor -> Code Style -> Kotlin` and enabling "Use single name
  import".
- Format via Android Studio's formatter. You can do this by navigating to `Code -> Reformat Code`
  and checking "Rearrange entries" and "Cleanup code" before clicking "Run".

## Code conventions

Because of how Ina is used, Ina has a strict security posture. Thus, it has the following code
conventions meant to enforce this posture:

- Avoid unnecessary third-party libraries. When a third-party library is needed, it should be
  well-maintained, widely used, and ideally written in a memory-safe way (i.e. using only the safe
  subsets of Rust, Kotlin, Java, etc.).
- Avoid unsafe code if possible. This includes the `unsafe` keyword in Rust, unsafe interfaces such
  as `sun.misc.Unsafe` in JVM-compiled languages, and other unsafe subsets of otherwide memory-safe
  programming languages.

## Licensing

Contributing to Ina requires signing a Contributor License Agreement (CLA). To sign [Accrescent's
CLA], just make a pull request, and our CLA bot will direct you. If you've already signed the CLA
for another Accrescent project, you won't need to do so again.

We require all code to have valid copyright and licensing information. If your contribution creates
a new file, be sure to add the following header in a code comment:

```
Copyright <current-year> Logan Magee

SPDX-License-Identifier: MPL-2.0
```

## Vulnerability reports

Ina's GitHub repository has [private vulnerability reporting] enabled. If you have a security issue
to report, either [submit a report] privately on GitHub or email us at <security@accrescent.app>.
Also be sure to read Ina's [security policy] before creating a report.

[Accrescent's CLA]: https://gist.github.com/lberrymage/1be5c6a041131b9fd0b54b442023ad21
[dependency verification]: https://docs.gradle.org/current/userguide/dependency_verification.html
[private vulnerability reporting]: https://github.blog/security/supply-chain-security/private-vulnerability-reporting-now-generally-available/
[security policy]: SECURITY.md
[submit a report]: https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability#privately-reporting-a-security-vulnerability
