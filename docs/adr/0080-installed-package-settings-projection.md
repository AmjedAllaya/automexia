# ADR 0080: Installed package settings projection

- Status: accepted
- Date: 2026-09-30
- Owners: Automexia maintainers
- Amends: [ADR 0079](0079-shared-settings-and-presentation-preferences.md)

## Context

Verified packages may declare bounded `settings.v1.json` metadata, but the
Settings sheet previously exposed only built-in extension controls. A declared
choice must not grant a capability or start package code. Optional package
storage errors must not make core Settings unusable.

## Decision

The package store owns committed installation inventory and reads extracted
settings on a bounded worker. It checks the stored receipt, committed identity,
and extracted content digest before the application projects any package text.
The application owns one immutable inventory snapshot and rejects stale refresh
generations. The Settings sheet projects each installed package as a category,
each declared feature as a page, and each option as a typed control. Removal
discards the page and its saved overrides only after a successful inventory
refresh. An unavailable inventory preserves preferences and core controls.
An uninitialized store is also insufficient evidence of removal. After a reset,
confirmed removal prunes both saved preferences and the in-memory undo snapshot;
explicit restoration cannot resurrect uninstalled package choices.

Declaration updates retain retired choices for rollback within the existing
preference limits. An edit that would exceed those limits reclaims only the
minimum retired control identities belonging to the edited package. Reclamation
preserves current control choices and unrelated owners. Feature and package resets clear
all choices in that exact owner scope, including retired controls.

Package choices are presentation-only desired values. They do not execute a
component, install a package, or confer filesystem, network, process, provider,
PTY, or credential authority. The private version-1 package preference file has
its own bounded snapshot and rollback copy. Its write intent is separate from
ordinary core preference saves, so a damaged package file does not block core
changes. The existing application preference writer remains the only writer.

## Limits and evidence

The public product still has no third-party download or install workflow. A
stored receipt and content digest detect changes against the installation
record; this read does not renew publisher signature trust against an actor who
can also rewrite local profile state. Tests cover signed install/restart,
malformed and modified metadata, maximum package count, stale refresh,
uninstall, option validation, and corrupt preference recovery. Native visual
and assistive-technology evidence is separate from model and UI tests.
