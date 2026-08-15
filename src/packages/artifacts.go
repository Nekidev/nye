package packages

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// The kind of an artifact.
//
// This may be either:
// * `manifest` - A package's manifest
// * `bin` - A binary
type ArtifactKind string

const (
	ArtifactKindBin      ArtifactKind = "bin"      // A binary artifact
	ArtifactKindManifest ArtifactKind = "manifest" // A package manifest
)

type ArtifactMetadata struct {
	Name    string                  // The artifact's file name
	Path    string                  // The path to the artifact's location
	Kind    ArtifactKind            // The kind of artifact
	Package ArtifactPackageMetadata // The artifact's package metadata
}

type ArtifactPackageMetadata struct {
	Name                 string // The package's name
	Version              string // The package's version
	Path                 string // The package's installation path
	IsInstalledForUser   string // If the package is installed for a user, their username
	IsInstalledForSystem bool   // If the package is installed for the system
}

// Gets a package artifact's metadata from a system path.
//
// The path may be either:
// * A path to an exposed symlink
// * A path to the artifact
//
// The metadata is extracted from the path, the manifest is not read.
//
// Returns:
// * `ArtifactMetadata` - The artifact's metadata, empty if an error occurred.
// * `error` - An error, if any.
func GetArtifactMetadataFromPath(path string) (ArtifactMetadata, error) {
	stat, err := os.Stat(path)
	if err != nil {
		return ArtifactMetadata{}, fmt.Errorf("could not get information about the specified path: %v", err)
	}
	if !filepath.IsAbs(stat.Name()) {
		return ArtifactMetadata{}, errors.New("the path of the artifact (or the path the symlink points to, if a symlink) must be absolute")
	}

	invalidPathError := errors.New("the specified path (or the path the symlink points to, if a symlink) did not point to an installed package's artifact")

	path = stat.Name()
	original := strings.Split(path, "/")
	segments := original[1:] // Absolute path (`/pkg` => ["", "pkg"]), skip the first empty string.

	isInstalledForUser := ""

	if segments[0] == "usr" && segments[2] == "pkg" {
		// The path points to a user installation.

		isInstalledForUser = segments[1]

		// Strip the /usr/username prefix, leave the segments starting by the "pkg" part.
		segments = segments[2:]
	} else if segments[0] == "pkg" {
		// The path points to a system installation.
	} else {
		return ArtifactMetadata{}, invalidPathError
	}

	// pkg/packages/{package-name}/{package-version}/artifact
	if len(segments) < 5 || segments[1] != "packages" {
		return ArtifactMetadata{}, invalidPathError
	}

	packageName := segments[2]
	packageVersion := segments[3]
	packagePath := ""

	if isInstalledForUser != "" {
		// (0)/usr(1)/{username}(2)/pkg(3)/packages(4)/{package-name}(5)/{package-version}(6)
		packagePath = filepath.Join(original[:7]...)
	} else {
		// (0)/pkg(1)/packages(2)/{package-name}(3)/{package-version}(4)
		packagePath = filepath.Join(original[:5]...)
	}

	artifactName := ""
	artifactPath := ""
	artifactKind := ArtifactKindBin // Default value, will be overriden below

	switch segments[4] {
	case "package.toml":
		artifactName = "package.toml"
		artifactPath = "package.toml"
		artifactKind = ArtifactKindManifest
	case "bin":
		artifactName = segments[len(segments)-1]
		artifactPath = filepath.Join(segments[5:]...)
		artifactKind = ArtifactKindBin
	default:
		return ArtifactMetadata{}, fmt.Errorf("the artifact was not placed in a valid location")
	}

	return ArtifactMetadata{
		Name: artifactName,
		Path: artifactPath,
		Kind: artifactKind,
		Package: ArtifactPackageMetadata{
			Name:                 packageName,
			Version:              packageVersion,
			Path:                 packagePath,
			IsInstalledForUser:   isInstalledForUser,
			IsInstalledForSystem: isInstalledForUser == "",
		},
	}, nil
}
