package packages

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
)

type Context struct {
	Path     string   // The project's base path, the directory where the `Nye.toml` manifest is located.
	Manifest Manifest // The unmarshaled manifest of the project.
}

// Gets the current working directory's context.
func GetContext() (Context, error) {
	dir, err := os.Getwd()
	if err != nil {
		return Context{}, fmt.Errorf("could not get current working directory for package context: %v", err)
	}

	for {
		guessedManifestPath := filepath.Join(dir, "Nye.toml")
		exists, err := exists(guessedManifestPath)
		if err != nil {
			return Context{}, fmt.Errorf("could not check if `Nye.toml` manifest existed in directory: %v", err)
		}

		if exists {
			manifest, err := GetManifest(guessedManifestPath)
			if err != nil {
				return Context{}, fmt.Errorf("could not read `Nye.toml` manifest for package context: %v", err)
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
	}

	return Context{}, errors.New("no `Nye.toml` manifest was found in this directory nor in any of its parents")
}

// Checks if a file or a directory exists.
func exists(path string) (bool, error) {
	_, err := os.Stat(path)

	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return false, nil
		} else {
			return false, fmt.Errorf("could not check if file or directory existed: %v", err)
		}
	} else {
		return true, nil
	}
}
