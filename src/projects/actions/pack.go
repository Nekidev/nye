package actions

import (
	"archive/zip"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/pelletier/go-toml/v2"
	"nyeki.dev/nye/packages"
	"nyeki.dev/nye/projects"
)

// Packs a project into a target-specific package.
//
// Arguments:
// * `ctx` - The project to pack's context.
// * `targetString` - The target to pack the project for.
//
// Returns:
// * `string` - The path to the packed zip file.
// * `error` - An error, if any occurred.
func PackProject(ctx projects.Context, targetString string) (string, error) {
	target, ok := ctx.Manifest.Targets[targetString]
	if !ok {
		return "", fmt.Errorf("the specified target %v is not supported by this package", targetString)
	}

	zipper, zipperName, err := createZipper(ctx, targetString)
	if err != nil {
		return "", fmt.Errorf("could not create package file: %v", err)
	}
	defer zipper.Close()

	manifest := packages.FromProjectManifest(ctx.Manifest, targetString)
	manifestBytes, err := toml.Marshal(&manifest)
	if err != nil {
		return "", fmt.Errorf("could not marshall package manifest: %v", err)
	}

	err = writeStringToZipper(zipper, string(manifestBytes), "package.toml")
	if err != nil {
		return "", fmt.Errorf("could not write manifest to package file: %v", err)
	}

	err = writeDirectoryToZipper(zipper, target.Source)
	if err != nil {
		return "", fmt.Errorf("could not write source files to package file: %v", err)
	}

	return zipperName, nil
}

// Creates a zip file.
//
// Arguments:
// * `ctx` - The packages context.
// * `targetString` - The target to create the zip file for.
//
// Returns:
// * `*zip.Writer` - The zip file writer.
// * `string` - The path to the created zip file.
// * `error` - An error, if occurred while creating the zip file.
func createZipper(ctx projects.Context, targetString string) (*zip.Writer, string, error) {
	distDir := filepath.Join(ctx.Path, "dist")
	err := os.MkdirAll(distDir, 0o755)
	if err != nil {
		return nil, "", fmt.Errorf("could not create dist dir: %v", err)
	}

	zipName := filepath.Join(distDir, fmt.Sprintf("nye-%v-v%v-for-%v-pack.zip", ctx.Manifest.Package.Name, ctx.Manifest.Package.Version, targetString))
	zipFile, err := os.Create(zipName)
	if err != nil {
		return nil, "", fmt.Errorf("could not create pack file: %v", err)
	}

	zipWriter := zip.NewWriter(zipFile)
	return zipWriter, zipName, nil
}

// Writes a file into a zip file.
//
// Arguments:
// * `file` - The zip writer to write the file into.
// * `input` - The path to the file to write in the zip file.
// * `output` - The path as which the file should be written in the zip.
func writeFileToZipper(file *zip.Writer, input string, output string) error {
	inputFile, err := os.Open(input)
	if err != nil {
		return fmt.Errorf("could not read zip's input file: %v", err)
	}
	defer inputFile.Close()

	outputEntry, err := file.Create(output)
	if err != nil {
		return fmt.Errorf("could not create entry `%v` in zip file for input path `%v`: %v", output, input, err)
	}

	_, err = io.Copy(outputEntry, inputFile)
	if err != nil {
		return fmt.Errorf("could not insert input file `%v` into zip file for output path `%v`: %v", input, output, err)
	}

	return nil
}

// Writes a string to a file in a zip file.
//
// Arguments:
// * `file` - The zip writer to write the file into.
// * `input` - The file's contents.
// * `output` - The path as which the file should be written in the zip.
func writeStringToZipper(file *zip.Writer, input string, output string) error {
	outputEntry, err := file.Create(output)
	if err != nil {
		return fmt.Errorf("could not create entry `%v` in zip file for input path `%v`: %v", output, input, err)
	}

	_, err = outputEntry.Write([]byte(input))
	if err != nil {
		return fmt.Errorf("could not write string to to zip file for output path `%v`: %v", output, err)
	}

	return nil
}

// Writes all of a directory's contents, recursively, to a zip file.
//
// Arguments:
// * `file` - The zip writer to write the directory in.
// * `input` - The path to the directory to write.
func writeDirectoryToZipper(file *zip.Writer, input string) error {
	return filepath.WalkDir(input, func(path string, dir os.DirEntry, err error) error {
		if err != nil {
			return err
		}

		info, err := os.Stat(path)
		if err != nil {
			return fmt.Errorf("could not get information about path in src directory: %v", err)
		}

		if info.IsDir() {
			return nil
		}

		rel, err := filepath.Rel(input, path)
		if err != nil {
			return fmt.Errorf("could not get relative path for file to pack: %v", err)
		}

		err = writeFileToZipper(file, path, rel)
		if err != nil {
			return fmt.Errorf("could not put file to pack in zip: %v", err)
		}

		return nil
	})
}
