package projects

import (
	"fmt"
	"maps"
	"os"
	"path/filepath"
	"slices"

	"github.com/pelletier/go-toml/v2"
	"nyeki.dev/nye/utils"
)

// A nye.toml manifest.
type Manifest struct {
	Package ManifestPackage `toml:"package" validate:"required"`
	Targets ManifestTargets `toml:"targets" validate:"required,unique,dive,keys,supported-target,endkeys"`
	Exposes ManifestExposes `toml:"exposes"`
}

type ManifestPackage struct {
	Name    string `toml:"name" validate:"required,min=1,max=32,kebab-case"`
	Version string `toml:"version" validate:"required,semver,max=32"`
}

type ManifestTargets map[string]ManifestTarget

type ManifestTarget struct {
	Source string `toml:"source" validate:"safe-path"`
}

type ManifestExposes struct {
	// Exposed symlinks on installation to the package's bundled binaries
	Bin []ManifestExposesBinary `toml:"bin" validate:"unique=Name"`
}

func (exposes *ManifestExposes) ForTarget(target string) ManifestExposes {
	bins := []ManifestExposesBinary{}

	for _, bin := range exposes.Bin {
		if len(bin.Targets) > 0 {
			if slices.Contains(bin.Targets, target) {
				bins = append(bins, bin)
			}
		} else {
			bins = append(bins, bin)
		}
	}

	return ManifestExposes{Bin: bins}
}

type ManifestExposesBinary struct {
	Name    string   `toml:"name" validate:"required,min=1,max=32,safe-path-segment"`
	Path    string   `toml:"path" validate:"safe-path"`
	Targets []string `toml:"targets" validate:"unique,dive,supported-target"`
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

	err = utils.ValidateStruct(&manifest)
	if err != nil {
		return Manifest{}, fmt.Errorf("manifest was not valid: %v", err)
	}

	err = validateTargets(path, manifest)
	if err != nil {
		return Manifest{}, fmt.Errorf("failed to validate targets: %v", err)
	}

	err = validateExposedBins(path, manifest)
	if err != nil {
		return Manifest{}, fmt.Errorf("failed to validate exposed binaries: %v", err)
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

func validateTargets(path string, manifest Manifest) error {
	for target, meta := range manifest.Targets {
		exists, err := utils.Exists(meta.Source)
		if err != nil {
			return fmt.Errorf("an error occurred while checking if `%v` existed: %v", meta.Source, err)
		}
		if !exists {
			return fmt.Errorf("the source directory for target `%v` (`%v`) does not exist", target, meta.Source)
		}
	}

	return nil
}

func validateExposedBins(path string, manifest Manifest) error {
	dir, _ := filepath.Split(path)

	targets := slices.Collect(maps.Keys(manifest.Targets))

	for _, bin := range manifest.Exposes.Bin {
		if bin.Path == "" {
			bin.Path = bin.Name
		}

		if len(bin.Targets) == 0 {
			bin.Targets = targets
		}

		for _, target := range bin.Targets {
			if !slices.Contains(targets, target) {
				return fmt.Errorf("an exposed binary (`%v`) specifies a target (`%v`) not supported by this package", bin.Name, target)
			}

			sourcePath := filepath.Join(manifest.Targets[target].Source, "bin", bin.Path)
			exists, err := utils.Exists(filepath.Join(dir, sourcePath))
			if err != nil {
				return fmt.Errorf("could not check if exposed binary `%v` existed in `%v`", bin.Name, sourcePath)
			}
			if !exists {
				return fmt.Errorf("the exposed binary `%v` does not exist in `%v`. if intentional, select the binary's targets with `exposes.bin.targets`", bin.Name, sourcePath)
			}
		}
	}

	return nil
}
