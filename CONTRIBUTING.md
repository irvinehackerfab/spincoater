# Contributing

## Branches
- Each spin coater generation has its own branch: `v3`, `v4` and so on. The default branch is the newest model, and
  that's where most work happens.
- A new generation starts as a branch forked from the one before it, so it keeps the code it reuses.
- Older model branches stay as a record of that design, for reference or for anyone who wants to build it. They only
  get fixes when needed.
- Do your work on a short-lived branch named after the change (e.g. `fix-motion-profile`) and open a pull request into
  the model branch.
- If a fix also applies to another model, copy it to that model's branch with `git cherry-pick -x <commit>` and open a
  pull request there too.

## Tags and releases
- Tags use [semantic versioning](https://semver.org/), and the major number is the generation:
  - `v3.0.0`: first V3 release
  - `v3.1.0`: new feature
  - `v3.1.1`: bug fix
  - `v4.0.0-beta.1`: test build, published as a pre-release
- A tag never moves. Every tag gets a [GitHub release](https://github.com/irvinehackerfab/spincoater/releases) with
  notes and downloads, and the Hacker Fab website links to the releases page.
- Only tag a commit after it has been tested on a real spin coater.

<!-- TODO: list what a release includes once the first one is made (firmware .bin, host_tui.exe, STL zip?) and who can publish releases. -->

## Writing docs
- Put images in [`images/`](images) and data in [`data/`](data), then link them from the docs with relative paths.
- Resize photos to 2048 px on the long edge (JPG quality 80, or 85 for photos with text, labels or wiring) and remove
  unnecessary EXIF data before committing.
- Only commit files the team made. Link vendor datasheets and CAD instead of committing them.
