---
name: Neo AutoClicker
description: Dark macro control station for configuring and running click sequences on Windows.
colors:
  primary: "#4FD8CA"
  primary-deep: "#1D7E77"
  signal-amber: "#F7B95C"
  signal-red: "#EF676F"
  danger-deep: "#5E272F"
  neutral-background: "#0C1116"
  neutral-surface: "#151D24"
  neutral-raised: "#1D2830"
  neutral-border: "#2E3E48"
  neutral-text: "#E7EFF2"
  neutral-muted: "#91A3AC"
typography:
  display:
    fontSize: "20px"
    fontWeight: 700
  title:
    fontSize: "17px"
    fontWeight: 700
  body:
    fontSize: "14px"
    fontWeight: 400
  label:
    fontSize: "12px"
    fontWeight: 600
rounded:
  control: "7px"
  surface: "10px"
  status: "18px"
spacing:
  compact: "8px"
  control-gap: "9px"
  section-gap: "12px"
  panel-padding: "16px"
components:
  button-primary:
    backgroundColor: "{colors.primary-deep}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.control}"
    padding: "8px 12px"
    height: "48px"
  button-danger:
    backgroundColor: "{colors.danger-deep}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.control}"
    padding: "8px 12px"
  shortcut-chip:
    backgroundColor: "{colors.neutral-raised}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.control}"
    padding: "8px 12px"
  input-field:
    backgroundColor: "{colors.neutral-raised}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.control}"
    padding: "8px 10px"
  destination-tab:
    backgroundColor: "{colors.neutral-raised}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.control}"
    padding: "8px 12px"
  status-pill:
    backgroundColor: "{colors.neutral-raised}"
    textColor: "{colors.primary}"
    rounded: "{rounded.status}"
    padding: "5px 10px"
  surface-panel:
    backgroundColor: "{colors.neutral-surface}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.surface}"
    padding: "14px 16px"
---

# Design System: Neo AutoClicker

## Overview

**Creative North Star: “Estação de comando”.**

Neo AutoClicker uses a dark, modern control-station language for people configuring repeated clicks while another Windows app is in front. The visual system is calm and information-first: graphite surfaces carry the controls, a measured teal identifies active actions, and amber or red is reserved for status and warnings.

The interface gives execution and its stop action a fixed home in the left rail. The main area holds destination, repetition, points, and their shortcuts. A single soft pulse on the running status supplies the technological note; the rest of the surface stays still and readable.

**Key Characteristics:**

- Dark graphite surfaces with teal control highlights.
- Persistent execution controls and visible shortcut chords.
- Thin borders and tonal layering instead of drop shadows.
- One restrained status animation while clicks are running.

## Colors

The palette pairs cool graphite neutrals with a restrained signal teal; amber and red communicate attention rather than decoration.

### Primary
- **Signal Teal** (`primary`): Shortcut capture, selected destinations, and active execution status.
- **Deep Teal** (`primary-deep`): The primary start action and selected control surfaces.

### Secondary
- **Instrument Amber** (`signal-amber`): Timing and scheduler warnings.
- **Signal Red** (`signal-red`) and **Deep Signal Red** (`danger-deep`): The red signal and its darker emergency-stop surface.

### Neutral
- **Night Graphite** (`neutral-background`): The window canvas.
- **Panel Graphite** (`neutral-surface`): Main panels and control groups.
- **Raised Graphite** (`neutral-raised`): Buttons, inputs, and shortcut chips above the canvas.
- **Instrument Border** (`neutral-border`): Quiet boundaries and table rules.
- **Readout White** (`neutral-text`): Primary labels and values.
- **Muted Steel** (`neutral-muted`): Supporting labels and explanations.

**The Signal Rule.** Teal marks selection and activity; amber and red retain warning and stop meaning.

## Typography

**Display Font:** egui default proportional face  
**Body Font:** egui default proportional face  
**Label/Mono Font:** egui monospace face for ordered point indices.

**Character:** Compact and direct. Size and weight carry hierarchy; uppercase is limited to short status and metric labels.

### Hierarchy
- **Display** (bold, 20 px): Product name in the application header.
- **Title** (bold, 17 px): Section names.
- **Body** (regular, 14 px): Controls and descriptive text.
- **Label** (semibold, 12 px): Status and metric names.

## Layout

The window opens at 1280×900 logical pixels and keeps a 980×720 minimum. A fixed 310 px left rail contains profiles, run controls, the start/stop chord, and click rhythm. The central area scrolls independently and contains the destination selector, repetition rules, point table, and global point shortcuts. A bottom strip keeps session metrics visible.

Use 8–16 px internal spacing. Keep the point table aligned and compact, while headings receive more separation from the content that follows. The left rail may scroll on shorter windows; the center remains a separate reading surface.

## Elevation & Depth

The UI is layered with darker and lighter graphite fills, then separated by a fine border. It does not use shadows or glass. The selected shortcut and destination use a tinted teal surface so their state reads without lifting every panel.

## Shapes

Panels use softly rounded 10 px corners. Controls use 7 px corners; the status badge is a small pill. Borders remain 1 px and low contrast except on focused or active controls.

## Components

- **Start control:** Full-width primary button with a 48 px target. It changes to the stop state while running.
- **Emergency stop:** Red-tinted action kept adjacent to the status.
- **Destination tabs:** Three mutually exclusive controls for cursor, fixed point, and sequence.
- **Shortcut chip:** Clicking the displayed chord starts global listening. Every pressed key appears in the preview; the chord is committed after all keys are released.
- **Point list:** Ordered coordinate rows with direct editing, reorder controls, and removal.
- **Status badge:** Shows the current state. Its small pulse only runs during execution.

## Do's and Don'ts

- Keep the primary start and emergency stop actions easy to locate.
- Keep a shortcut's full chord visible in its chip.
- Use amber for timing warnings and red for stop or failure feedback.
- Keep actions operable by keyboard and pointer.
- Don't add a separate speed-mode selector; retain the destination and repetition choices.
- Don't use glow, grids, or animation as background decoration.
- Don't describe requested CPS as guaranteed or as clicks acknowledged by another app.
