Commands for MVP:

- DONE nye dev new [name] [--path <path>]: Creates a new package project.
- DONE nye dev pack: Packages the current package project.
- nye dev install: Installs the current package project.
- nye dev uninstall: Uninstalls the current package project.

- nye install
- nye uninstall


Filesystem:

- /bin: System-wide binaries
- /var: System-wide variable data
- /pkg: System-wide packages
- /pkg/tmp: System-wide temporary package data (e.g. extraction)
- /pkg/packages: System-wide-installed package data
- /usr/{username}: User-specific namespace
- /usr/{username}/bin: User-specific binaries
- /usr/{username}/pkg: User-specific packages
- /usr/{username}/pkg/tmp: User-specific temporary package data (e.g. extraction)
- /usr/{username}/pkg/packages: User-installed package data
