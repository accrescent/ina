<!--
Copyright 2024 Logan Magee

SPDX-License-Identifier: LicenseRef-Proprietary
-->

# Ina

Secure, robust, and efficient delta updates for executables.

## About

Ina is a CLI tool and set of libraries for creating and applying binary patches between files. It is
especially well-suited for producing small patches between executable files and was designed for
reducing the size of app updates in [Accrescent].

The products of this repository are the Ina CLI, Rust library, and Android library. JNI interfaces
and other crates are not part of this repository's public API and so may change at any time. CLI
interfaces should not be considered stable.

## License

Copyright 2023-2024 Logan Magee. All rights reserved.

[Accrescent]: https://accrescent.app
