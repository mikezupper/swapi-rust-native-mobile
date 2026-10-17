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
Try OSX-KVM for the iOS build environment first. The user has a Mac available
as a fallback and wants assistance enabling SSH and remote build/test commands.

## Current scope

Holocron is a working title. Following the initial People slice, the user
requested the remaining SWAPI categories. Both native apps expose People,
Planets, Starships, Vehicles, Species and Films with collection search, pagination,
details, refresh and retry through the Rust core.

Explore, search results, and character detail are the first design surfaces.
The next implemented milestone adds persistent favorites, SQLite-backed offline
records, local search, downloadable collections and native motion/haptics.
Navigation between related records remains a future feature. A full backend, authentication, and social features have no
current product requirement.

## Experience expectations

Users can identify the primary action, navigate back naturally, and understand
loading or errors without losing their place. Search results stay correct
while typing quickly. Larger text and reduced motion preserve a complete,
readable experience. Empty and failure screens are designed alongside content.

The first implemented feature should already exhibit the intended visual
quality. Broader content coverage follows a working and inspectable foundation.
