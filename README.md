# `nye` - Nyeki's Package Manager

For a long time I've wanted a predictable package manager. I had installed a package for Hyprland on
Arch Linux, it looked great. Then one day, it all breaks. Wallpapers not loading, apps not working
or rendering badly, configs seemingly broken. I thought of uninstalling the package and reinstalling
it, it should update and fix. I go to the package's docs, nothing about uninstalling. Installation
had been via shell script. I search the package's GitHub repo and find it. My search for an
`uninstall.sh` had come to an end. I clone the repo, `chmod +x uninstall.sh`, then `./uninstall.sh`.
The following got printed to my terminal:

```
Hi there!
This script 1. will uninstall [end-4/dots-hyprland > illogical-impulse] dotfiles
            2. will try to revert *mostly everything* installed using "./setup install", so it's pretty destructive
            3. has not been tested, use at your own risk.
            4. will show all commands that it runs.
Ctrl+C to exit. Enter to continue.
```

What? What do you mean "mostly everything?" "has not been tested?" "pretty destructive?" Yeah no. I
felt corageous, so I pressed enter. I went from broken theme to no theme apparently, yet
dependencies got bugged in the package manager, re-installation didn't work at all, and I knew this
had left leftovers everywhere.

Ever since, I knew I needed a proper package manager, not some shell script. Don't get me wrong, I
already use `pacman` and `yay`, yet let's be honest: they're `install.sh` with colors.

`nye` is declarative, no installation scripts, no uninstallation scripts. Everything installed is
installed in places the package manager knows about, everything exposed is exposed by the package
manager, and uninstallation is as easy as removing the declared files.

This may look to you like `nix`, `flatpak`, or `snap`. It's not. I started designing `nye` before I
even knew how `nix` worked, and I don't want to sandbox applications. I want them in their place,
not disturbing the user. I don't care about reproducibility, I care about install/uninstall
predictability, speed, ease of use (fym `-Syu == install`), and tidiness. I also took the freedom of
redesigning my assumptions about the filesystem hierarchy, which means I'll soon enough have to
build a distro for `nye`.

Also, I don't care about the dependencies packages install. Why do I have to see some random
`audio-dependency-abc`'s package files when I just wanted to install Spotify? Why do I have to see
all the bins in `PATH` when I just care about a subset? That's why `nye` only exposes to the user
what the user installed.

Why does every package just dump their config files into my `$HOME`? Why are there no user-specific
packages? Why's `root` not in `/home` instead of in `/root`? Why `/sbin`, `/bin`, `/usr/bin`,
`$HOME/.cargo/bin`, `$HOME/.go/bin`? Why `$HOME/.config` when there's `/etc` and `/usr/etc`? Who
thought `$HOME/.local` was a good idea? Who thought dumping everything user-specific to `$HOME` was
a good idea? Packages nowadays dump their data wherever they feel best, can't someone come and put
some order to it? Well, now `nye` takes care of it.

Those and many more things are my complaints to the current state of things that `nye` solves.

## Installation

Installing `nye` is simple. You can either

1. Install it directly from the repository via
   `cargo install --git https://github.com/Nekidev/nye.git --bin nye`, or
2. Run `cargo install nye` directly for the latest released version.

You could also package `nye` itself in a `nye` package by building it with `cargo`, moving it to the
`nye` package's directory, packaging the package, installing the package file, and uninstalling the
`cargo`-installed `nye`, but that's out of scope for releases at the moment until `nye` gets its own
distro.

## File System Assumptions

`nye` takes the freedom to assume a different file system layout than the one commonly seen in Linux
systems. The file system assumed looks like this:

```
/bin                       -  System-wide exposed installed binaries.                                     -  Owner: root
/etc                       -  System-wide package-specific editable text configuration file directories.  -  Owner: root
/etc/{package-name}        -  System-wide package-specific editable text configuration files.             -  Owner: root
/env                       -  System-wide exposed environment variables.                                  -  Owner: root
/pkg                       -  System-wide package installation context.                                   -  Owner: root
/pkg/store                 -  Store for system-wide installed packages.                                   -  Owner: root
/usr                       -  User-specific data directories.                                             -  Owner: root
/usr/{username}            -  User-specific data.                                                         -  Owner: {username}
/usr/{username}/bin        -  User-specific exposed installed binaries.                                   -  Owner: {username}
/usr/{username}/env        -  User-specific exposed environment variables.                                -  Owner: {username}
/usr/{username}/pkg        -  User-specific package installation context.                                 -  Owner: {username}
/usr/{username}/pkg/store  -  Store for user-specific installed packages.                                 -  Owner: {username}
/usr/{username}/room       -  User-specific user data.                                                    -  Owner: {username}
```

Note that `root` is itself treated as another user. It'll have its own `/usr/root/` namespace
instead of `/root`.

## CLI Usage

Usage is quite simple. By default, all commands will be run in the current user's user-specific
installation (`/usr/{username}/pkg`). To run commands on the system-wide installation instead, pass
the `--system` flag after `nye` (e.g. `nye --system install package-name`).

The following command examples show a basic use of the commands. To see up-to-date and detailed
usage descriptions, run `nye --help`.

### Installing Packages

```
Install one or more packages

Usage: nye install [OPTIONS]

Options:
  -p, --path <PATH>  The path to one or more installable package files
  -h, --help         Display instructions on how to use nye install

Example:
  nye install --path package-1.zip --path package-2.zip
```

### Uninstalling Packages

```
Uninstall one or more packages

Usage: nye uninstall [PACKAGES]...

Arguments:
  [PACKAGES]...  The names of the packages to uninstall

Options:
  -h, --help  Display instructions on how to use nye uninstall

Example:
  nye uninstall package-1 package-2
```

### Create a Package Project

```
Initialize a new package project

Usage: nye dev init [OPTIONS] [PATH]

Arguments:
  [PATH]  The directory to use for the new package project [default: .]

Options:
  -n, --name <NAME>  The name to give the package project. Defaults to the path's directory name
  -h, --help         Display instructions on how to use nye dev init

Example:
  nye dev init . --name package-1
```

### Package a Package Project

```
Package the current project into an installable file

Usage: nye dev pack [OPTIONS]

Options:
  -t, --target <TARGETS>  Filter the supported targets to package
  -o, --overwrite         Overwrite existing packages in the dist folder
  -h, --help              Display instructions on how to use nye dev pack

Example:
  nye dev pack --target linux-x86_64 --overwrite
```

## Package Projects

Package projects let you bundle all package artifacts into target-specific installable package
files.

The currently supported targets are the following:

- `linux-x86`
- `linux-x86_64`
- `linux-arm`
- `linux-aarch64`
- `linux-m68k`
- `linux-mips`
- `linux-mips32r6`
- `linux-mips64`
- `linux-mips64r6`
- `linux-csky`
- `linux-powerpc`
- `linux-powerpc64`
- `linux-riscv32`
- `linux-riscv64`
- `linux-s390x`
- `linux-sparc`
- `linux-sparc64`
- `linux-hexagon`
- `linux-loongarch32`
- `linux-loongarch64`

A default project structure looks like follows:

```
nye.toml           # Package project manifest.
.gitignore         # .gitignore
src/               # Package project's source files.
    linux-x86_64/  # Target-specific source files.
        bin/
        etc/
        lib/
    shared/        # Cross-target fallback source files.
        bin/
        etc/
        lib/
dist/              # Where built package files will be placed after `nye dev pack`.
```

### Package Manifests

Package manifests are located at the root of each package project. They're called `nye.toml`, and by
default look something like this:

```toml
[package]
name = "example"             # The package's name
version = "0.0.0"            # The package's semver-compliant version

[targets.linux-x86_64]
source = "src/linux-x86_64"  # The directory under where target-specific files for this package are located in the project.

[targets.shared]
source = "src/shared"        # The directory under where cross-target files for this package are located in the project.
```

You can edit all fields. Specific targets are all optional, though you must always have at least one
specific target configured (i.e. any target but `shared`). You can have as many unique targets
configured as you want, provided they're supported by `nye`.

#### Exposing Binaries

Not every binary under `src/{target}/bin` will be placed in the installation context's (`/` or
`/usr/{username}/`) `bin` directory. For them to be available, they have to be **exposed**.

```toml
[[exposes.bin]]
path = "path/to/binary"     # The path under each target's `bin` dir where this binary will be located.
links = ["a", "b"]          # The names under which this binary will be made available. In this example, both "a" and "b" will link to the binary. When unspecified, it'll default to the binary's name.
targets = ["linux-x86_64"]  # The targets under which this binary will be available. When unspecified, all targets in the manifest's `targets.*` will be used.
```

Assuming our project directory structure to be the following:

```
nye.toml
src/linux-x86/bin/
    ...
src/linux-x86_64/bin/
    path/to/binary
src/shared/bin/
    path/to/binary
```

- When packaging for `linux-x86_64`, the one under `src/linux-x86_64/bin` will be packaged.
- When packaging for `linux-x86`, the one under `src/shared/bin` will be packaged.

Not all `src/*/bin` directories have to have the file under `path`. For example, if you have

```toml
[target.linux-x86]
source = "src/linux-x86"

[target.linux-x86_64]
source = "src/linux-x86_64"

[target.shared]
source = "src/shared"

[[exposed.bin]]
path = "path/to/binary"
links = ["a", "b"]
targets = ["linux-x86_64"]
```

Then you only need `path/to/binary` to be found under either `src/shared/bin` or `src/linux-x86_64`.
As long as the path is found for every target that requires it, you're good to go.

Target-specific binaries will always be selected over shared binaries.
