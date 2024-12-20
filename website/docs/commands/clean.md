---
sidebar_position: 7
---

## clean | purge
<!--
  @generated SignedSource<<98b5cb7370d82107ca055986d5826da7>>
  Run `./scripts/generate-command-markdown.py` to regenerate.
-->


**delete untracked files**

Delete all untracked files in your working copy. Untracked files are files
that are unknown to Sapling. They are marked with &quot;?&quot; when you run
`sl status`.

By default, `sl clean` implies `--files`, so only untracked
files are deleted. If you add `--ignored`, ignored files are also
deleted. If you add `--dirs`, empty directories are deleted and
`--files` is no longer implied.

If directories are given on the command line, only files in these
directories are considered.

Caution: `sl clean` is irreversible. To avoid accidents, first
perform a dry run with `sl clean --print`.

## arguments
| shortname | fullname | default | description |
| - | - | - | - |
| `-a`| `--abort-on-err`| | abort if an error occurs|
| | `--ignored`| | delete ignored files too|
| | `--dirs`| | delete empty directories|
| | `--files`| | delete files|
| `-p`| `--print`| | print filenames instead of deleting them|
| `-0`| `--print0`| | end filenames with NUL, for use with xargs (implies -p/--print)|
| `-I`| `--include`| | include files matching the given patterns|
| `-X`| `--exclude`| | exclude files matching the given patterns|
