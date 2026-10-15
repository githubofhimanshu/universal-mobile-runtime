# Project Brief

## Working name

**Universal Mobile Runtime (UMR)** — temporary name; perform trademark and domain checks before public use.

## Vision

Make supported mobile applications accessible from desktop computers through one coherent product experience, even when the applications were designed for different mobile operating systems.

## Mission

Build a desktop application shell and a set of isolated, replaceable execution backends that can load, launch, manage, and diagnose supported Android and iOS applications on Windows, macOS, and Linux.

## Problem

Mobile applications are tied to a runtime, framework APIs, operating-system services, graphics behavior, CPU architecture, package format, and distribution/signing model. Users and developers lack one dependable interface that abstracts the differences across those environments.

## Proposed product

UMR consists of:

- A common desktop shell and application library.
- A package import and validation workflow.
- An Android execution backend.
- An iOS execution backend whose strategy must be proven through feasibility experiments.
- Host adapters for Windows, macOS, and Linux.
- Graphics, input, audio, storage, networking, and permission adapters.
- Compatibility diagnostics, test evidence, and a documented support matrix.

The interface can be unified while execution remains platform-specific. “One product” does not imply one emulator or one identical mechanism on every host.

## Initial users

1. Developers who need to test or use mobile applications from desktop environments.
2. QA teams running repeatable compatibility checks.
3. Technical users who have rights to install the app packages they import.

A public consumer app store is not required to prove the first execution workflows and should follow after package security, distribution rights, and compatibility reporting are established.

## Product principles

- Evidence before claims.
- Backend isolation over a monolithic emulator.
- Reuse mature components after license and platform review.
- Host-native integration where it improves reliability.
- Secure-by-default package handling and least-privilege device access.
- Clear separation between local execution and remote execution.
- Explicit limitations instead of silent failures.

## Non-goals for the first proof of concept

- Guaranteeing that every Android or iOS application works.
- Recreating all of Android, iOS, Google Play Services, Apple services, DRM, anti-cheat, or hardware attestation.
- Distributing third-party mobile applications without permission.
- Replacing the operating system or bypassing platform security controls.
- Supporting Android/iOS as host operating systems during this desktop-first round.

## First milestone

Produce a feasibility evidence pack and a minimal vertical slice. It must identify concrete Android apps that run on target hosts and a legally supportable, technically demonstrated path for a defined subset of iOS apps. If iOS local execution is not feasible on a target combination, that limitation must be surfaced as a product decision—not hidden behind a mock or remote session.
