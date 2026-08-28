Commands for MVP:

- DONE nye dev new [path] [--name <name>]: Creates a new package project.
- DONE nye dev pack [--target <target>]: Packages the current package project.

- nye install
- nye uninstall


Filesystem:

- /bin: System-wide binaries
- /var: System-wide variable data
- /pkg: System-wide packages
- /pkg/tmp: System-wide temporary package data (e.g. extraction)
- /pkg/store: System-wide-installed package data
- /usr/{username}: User-specific namespace
- /usr/{username}/bin: User-specific binaries
- /usr/{username}/pkg: User-specific packages
- /usr/{username}/pkg/tmp: User-specific temporary package data (e.g. extraction)
- /usr/{username}/pkg/store: User-installed package data


Registry:

-  GET /v1/packages
- POST /v1/packages
-  GET /v1/packages/{package_id}/versions/{version_id}
