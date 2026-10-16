# Product

<!-- impeccable:product-schema 1 -->

## Platform

windows

## Users

People using Windows desktop applications who need to automate repetitive clicks at the current cursor, a fixed point, or an ordered set of screen positions.

## Product Purpose

Neo AutoClicker is a portable native Windows app for configuring and controlling repeated mouse clicks, including while another application is in the foreground. Success means users can set up a click run and its targets quickly, see what will happen, and stop it immediately.

## Positioning

It combines click timing, editable position sequences, global hotkeys, profiles, and an optional desktop overlay in one application without auxiliary processes or benchmark tools.

## Operating Context

Users configure the app, switch to another Windows application, and start, stop, or capture screen coordinates with global keyboard shortcuts. Screen coordinates may be negative on multi-monitor desktops. The interface and feedback are in Brazilian Portuguese.

## Capabilities and Constraints

- Preserve profiles, current-cursor and fixed-position destinations, ordered position sequences, editable/reorderable positions, and the transparent non-interactive overlay.
- Preserve CPS and interval controls, press duration, mouse button, single/double click, continuous or finite repetition, and click limits. CPS and interval remain synchronized.
- Global shortcuts are configurable through a listening capture that records every key held in a chord. Position capture must not move the cursor or generate clicks.
- The speed-mode selector (Normal, High precision, Unlimited) is removed; the destination and repetition choices remain.
- The Windows target is x64 MSVC. The app must not require administrator privileges for normal operation or attempt to bypass anti-cheat or other protections.
- Performance figures describe events accepted by the Windows input API; they do not prove that another application processed every click.

## Brand Commitments

The product name is Neo AutoClicker. User-facing copy is Brazilian Portuguese. The requested visual character is a polished dark mode with a restrained, modern technological feel.

## Evidence on Hand

The attached product brief and the current Rust/egui implementation define the existing features. No measured benchmark or external recognition data is available; the UI must not invent performance claims.

## Product Principles

- Make starting, stopping, and the emergency stop easy to find.
- Make shortcut setup direct and show the complete chord before saving it.
- Keep click destinations and repetition understandable before a run starts.
- Preserve configured profiles and positions through product renaming.
- State performance limits honestly.
