# Product brief

Create a visually distinctive Star Wars explorer that makes browsing
characters and their connections enjoyable on iOS and Android.

## Confirmed requirements

Use https://swapi.dev/ with strictly native SwiftUI/Swift and Jetpack
Compose/Kotlin interfaces. Both apps consume application state from a shared
Rust core through Mozilla UniFFI. Rust owns HTTP interactions and shared
behavior. The apps use a shared design system and native platform UX.

Visual quality, motion, and interaction polish are central to the project.
Use the supplied harness engineering approach and Beads for work tracking.
Try OSX-KVM for the iOS build environment first. The user has a Mac available
as a fallback and wants assistance enabling SSH and remote build/test commands.

## Proposed first release

Holocron is a working title. The initial experience focuses on People:
browse, search, paginate, open character details, and recover from failures.
Start with one end-to-end slice that displays Luke Skywalker through the real
Rust core in both native apps, with loading, failure, and retry behavior.

Explore, search results, and character detail are the first design surfaces.
Linked planets and films, favorites, and persistent offline caching are later
scope decisions. A full backend, authentication, and social features have no
current product requirement.

## Experience expectations

Users can identify the primary action, navigate back naturally, and understand
loading or errors without losing their place. Search results stay correct
while typing quickly. Larger text and reduced motion preserve a complete,
readable experience. Empty and failure screens are designed alongside content.

The first implemented feature should already exhibit the intended visual
quality. Broader content coverage follows a working and inspectable foundation.
