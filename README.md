** See [issues after cutover]{}

## Introduction

Fanling10 is is a version of fanling using rust.

## Fanling

Fanling is a program/app for storing personal data.

### Cross-platform

Fanling can currently run on:

* Linux PC
* Android

Because of the design, Fanling should be able to be ported to MS
Windows, Apple and possibly iPhone.

Most of the code is written in Rust, and uses cross-platform
crates. In particular, Git (or, rather, git2) is used to synchronise
data across multiple installations, and sqlite is used to search a
library. This means that the same data can be synchronised across
multiple installations, even though they are on different platforms.

### Different kinds of items

Fanling can store different kinds of data. At present, it stores:

* text pages with Markdown (similar to wiki pages)
* to-do items

Fanling can be extended to add new kinds of data.

## Detailed documentation

Use `cargo doc --all --open` to generate the existing code generation
for the Rust code. (Add `--document-private-items` for documentation
of non-public items).

## Current status

Currently preparing for release on GitHub.

The initial version is still being coded and tested. There are still
many TODOs and the tests only cover part of the code.
