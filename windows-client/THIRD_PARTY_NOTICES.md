<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# Windows client third-party notices

The authoritative third-party notice, versioned package manifest, and
distribution requirement are in [../THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).
The Inno Setup wizard contains a self-contained Windows client. Before
distributing it, run the locked installer build and notice collection against
the exact publish payload. The artifact must contain the notices for every
included .NET runtime, native/package asset, and the installer builder. A
self-contained payload with any missing runtime notice is not distributable.
