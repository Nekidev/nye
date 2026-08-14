package packages

// A Nye.toml manifest.
type Manifest struct {
	Package ManifestPackage `toml:"package"`
	Exposes ManifestExposes `toml:"exposes"`
}

type ManifestPackage struct {
	Name    string `toml:"name"`
	Version string `toml:"version"`
}

type ManifestExposes struct {
	// Exposed symlinks on installation to the package's bundled binaries
	Bin []ManifestExposesBinary `toml:"bin"`
}

type ManifestExposesBinary struct {
	Name string `toml:"name"`
	Path string `toml:"path"`
}
