# Architecture

The two native apps share application behavior through a Rust library.
UniFFI generates the language boundary; native build tooling compiles and
packages the library for each target.

## Ownership

| Layer | Owns |
| --- | --- |
| `crates/swapi-core` | HTTP, DTO parsing, resource references, application state, search, pagination, request cancellation and cache policy |
| `crates/swapi-ffi` | Concrete exported records/enums, commands, subscription handles, and mapping to the core |
| Swift adapter | Main-actor delivery, native observable state, and subscription lifetime |
| Kotlin adapter | StateFlow adaptation, lifecycle integration, and subscription lifetime |
| SwiftUI / Compose | Rendering, navigation stacks, focus, accessibility, localization, gestures, animations and haptics |
| `design/` | Shared visual tokens and component/motion specifications |

All four code layers exist for People browsing and character detail. Add abstractions only
when they solve a concrete boundary or testing problem.

## Data and state contract

Native interaction sends a typed command to Rust. Rust updates application
state and publishes immutable snapshots. Each native adapter renders the same
meaning while retaining platform navigation and interaction conventions.

Use a small listener interface implemented by the native adapters, exposed
through UniFFI foreign traits. A subscription must have explicit cleanup.
Its initial snapshot and subsequent updates must have defined ordering; use
revisions or another verified ordering mechanism to reject stale delivery.
Serialize state mutation and invoke foreign listeners outside internal locks.

The binding and native tests verify async execution, runtime lifetime,
callback threading, cancellation, and disposal behavior. Native task
cancellation does not automatically stop Rust work. A disposed subscriber must
not update a screen; an older query must not overwrite newer search results.

Model loading, content, empty, and failure meaning explicitly. Preserve useful
content during refresh or pagination when appropriate. Keep animation progress,
keyboard focus, and scroll position native. No animation frame crosses FFI.

The implemented `CharacterStore` serializes commands in a Tokio actor. Replacing
or cancelling a load drops its HTTP future. Retry is permitted only for a
retryable failure and retains the failed resource identity. Watch receivers
deliver the latest snapshot; slow observers may skip intermediate states.
Every publication increments a revision. This is a state stream, not an event
log that guarantees every transition reaches every observer.

`CharacterSession` exposes load/retry/cancel/subscribe/shutdown. Its shared
process-lifetime runtime has two worker threads. Listener calls execute outside
watch locks and must promptly enqueue native delivery. Listener errors end the
subscription. `Subscription.cancel()` aborts future delivery, but an already
running callback may finish. Both native adapters invalidate pending UI updates
before cancellation and reject stale revisions. Kotlin owns the session in a
ViewModel and explicitly destroys generated handles; Swift uses weak listener
ownership and shuts down on model deinitialization. `shutdown` avoids colliding
with UniFFI's generated Kotlin `AutoCloseable.close`.

`PeopleStore` owns the query, a 300 ms cancellable debounce, loaded people,
total count, next page and a typed loading/failure phase. Query replacement
drops the old future and clears results belonging to the old query. Refresh
and page loading retain current records; failed operations retain their exact
page for retry. Duplicate in-flight commands are ignored and page results are
deduplicated by person ID. Explicit refresh replaces the accumulated pages.
Both state actors use command-first selection to prevent a queued new query
from losing a race with an older response.

`PeopleSession` uses the same listener and lifetime contract as character
sessions. SwiftUI NavigationStack and Compose NavHost own routes; each detail
destination owns a session with the selected ID. Android saves the search draft
in SavedStateHandle, while native scroll state survives navigating back.
Native fields own editing/focus only; Rust interprets and executes searches.

## API boundary

Use `https://swapi.dev/api/` as the production endpoint. Provide an injectable
loopback base URL for deterministic local tests. The implemented client uses
pinned reqwest, serde, and Tokio versions in the workspace manifest. Explicit
Rustls/ring and WebPKI roots avoid platform TLS initialization across FFI.
Requests have a 15-second deadline and 1 MiB response limit; redirects and
automatic retries are disabled. Rust classifies retryable failures.

Parse transport DTOs once. SWAPI represents many measurements as strings and
uses values such as `unknown` and `n/a`; preserve the distinction where useful.
Validate relationship and pagination URLs before turning them into resource
references or following them. Native screens consume typed records, not JSON
or raw HTTP response objects. Localized user-facing error copy belongs native;
Rust supplies structured failure categories and retry information.

## Packaging

Generate bindings from the same revision and matching UniFFI versions as the
compiled library. iOS packaging must distinguish device and simulator slices;
Android packaging must include the ABIs supported by the selected devices and
emulators. Verify the generated interface in both native toolchains early.

The apps target Android API 26+ and iOS 18+. Android packages arm64-v8a and
x86_64 shared libraries with 16 KiB segment alignment. Apple builds package
arm64 device and simulator static libraries in an XCFramework. Rust 1.97.1
and UniFFI 0.32.2 are pinned; [the README](README.md) lists native toolchain
versions and [build commands](docs/development.md). Local artwork ships in
native asset bundles. Generated libraries and build output are not committed.

## References

- [UniFFI guide](https://mozilla.github.io/uniffi-rs/latest/)
- [UniFFI async support](https://mozilla.github.io/uniffi-rs/latest/futures.html)
- [UniFFI foreign traits](https://mozilla.github.io/uniffi-rs/latest/foreign_traits.html)
- [SWAPI contract](https://swapi.dev/documentation)
