package packages

import (
	"fmt"
	"os"

	"github.com/pelletier/go-toml/v2"
)

// A Nye.toml manifest.
type Manifest struct {
	Package ManifestPackage `toml:"package"`
	Exposes ManifestExposes `toml:"exposes"`
}

type ManifestPackage struct {
	Name string `toml:"name"`
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

func GetManifest(path string) (Manifest, error) {
	contents, err := os.ReadFile(path)
	if err != nil {
		return Manifest{}, fmt.Errorf("could not read manifest file: %v", err)
	}

	manifest := Manifest{}
	err = toml.Unmarshal(contents, &manifest)
	if err != nil {
		return Manifest{}, fmt.Errorf("could not parse manifest file: %v", err)
	}

	return manifest, nil
}

func SetManifest(path string, content Manifest) error {
	bytes, err := toml.Marshal(content)
	if err != nil {
		return fmt.Errorf("could not marshal manifest to TOML: %v", err)
	}

	err = os.WriteFile(path, bytes, 0o644)
	if err != nil {
		return fmt.Errorf("could not write manifest to path %v: %v", path, err)
	}

	return nil
}
