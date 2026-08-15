package projects

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"

	"nyeki.dev/nye/utils"
)

type Context struct {
	Path     string   // The project's base path, the directory where the `nye.toml` manifest is located.
	Manifest Manifest // The unmarshaled manifest of the project.
}

// Gets the project context of a directory.
func GetContext(dir string) (Context, error) {
	for {
		guessedManifestPath := filepath.Join(dir, "nye.toml")
		exists, err := utils.Exists(guessedManifestPath)
		if err != nil {
			return Context{}, fmt.Errorf("could not check if `nye.toml` manifest existed in directory: %v", err)
		}

		if exists {
			manifest, err := GetManifest(guessedManifestPath)
			if err != nil {
				return Context{}, fmt.Errorf("could not read `nye.toml` manifest for package context: %v", err)
			}

			return Context{
				Path:     dir,
				Manifest: manifest,
			}, nil
		}

		parent := filepath.Dir(dir)

		if parent == dir {
			break
		}

		dir = parent
	}

	return Context{}, errors.New("no `nye.toml` manifest was found in this directory nor in any of its parents")
}

func GetContextCwd() (Context, error) {
	dir, err := os.Getwd()
	if err != nil {
		return Context{}, fmt.Errorf("could not get current working directory for package context: %v", err)
	}

	return GetContext(dir)
}
