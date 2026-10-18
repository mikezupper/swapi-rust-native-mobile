# Product brief

Create a visually distinctive Star Wars explorer that makes browsing
people, worlds, vessels, species and films enjoyable on iOS and Android.

## Confirmed requirements

Use https://swapi.dev/ with strictly native SwiftUI/Swift and Jetpack
Compose/Kotlin interfaces. Both apps consume application state from a shared
Rust core through Mozilla UniFFI. Rust owns HTTP interactions and shared
behavior. The apps use a shared design system and native platform UX.

Visual quality, motion, and interaction polish are central to the project.
Use the supplied harness engineering approach and Beads for work tracking.
The initial environment strategy tried OSX-KVM before using a Mac over SSH.
The bounded VM experiment stopped at recovery boot; supported iOS verification
uses macOS/Xcode, locally or in hosted CI. See the
[environment guide](development.md) for setup and the historical observations.

## Current scope

Holocron is a working title. Following the initial People slice, the user
requested the remaining SWAPI categories. Both native apps expose People,
Planets, Starships, Vehicles, Species and Films with collection search, pagination,
details, refresh and retry through the Rust core.

Explore, search results, and character detail are the first design surfaces.
The app includes persistent favorites, SQLite-backed offline
records, local search, downloadable collections and native motion/haptics.
Connected records, discovery, ship comparison, cinematic crawl playback and
home-screen widgets extend that foundation. A full backend, authentication, and social features have no
current product requirement.

## Experience expectations

Users can identify the primary action, navigate back naturally, and understand
loading or errors without losing their place. Search results stay correct
while typing quickly. Larger text and reduced motion preserve a complete,
readable experience. Empty and failure screens are designed alongside content.

Each completed feature includes visual quality, accessibility and lifecycle
handling. The initial People slice established that foundation before expansion.

## Experience directions

| Addition | Example experience | What it demonstrates |
| --- | --- | --- |
| Connected stories | Move from Luke to Tatooine, his films and his ships | Rust-owned typed relationships and cache reuse with native navigation |
| Starship comparison | Compare length, crew, cargo and hyperdrive side by side | Shared normalization, explicit unknown values, native charts and selection transitions |
| Cinematic opening crawl | Play/pause a film crawl, with an accessible static reading mode | Native animation frames and reduced-motion behavior using shared content |
| Home-screen widgets and record routes | Open a favorite or discover a saved record of the day | Native system integration backed by the saved archive |
| Adaptive tablet/foldable layouts | Keep the collection beside the selected detail | One state model with platform-specific navigation and layout |
| Shareable record links | Share a record and reopen directly into its native detail | Native share sheets and deep-link routing into typed Rust identities |
| Demo inspector | View HTTP timing, cache hits, saved counts and request cancellation | Observable shared-core behavior that makes the architecture understandable |

Connected stories, comparison, the cinematic crawl and widgets are implemented.
Discovery adds featured saved worlds, recent visits and related saved suggestions.
Adaptive layouts, public shareable web links and the demo inspector remain future
options. Personal named collections were not selected for this milestone.
Background synchronization can follow if there is a useful freshness requirement;
the current catalog changes slowly.

The implemented widgets use
[WidgetKit](https://developer.apple.com/documentation/widgetkit) on iOS and
[RemoteViews](https://developer.android.com/develop/ui/views/appwidgets) on Android.
App Intents, launcher shortcuts, native share sheets and adaptive list/detail
layouts are possible extensions, not currently implemented integrations.
