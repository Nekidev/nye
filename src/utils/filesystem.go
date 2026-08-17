package utils

import (
	"errors"
	"fmt"
	"io"
	"os"
)

// Checks if a file or a directory exists.
func Exists(path string) (bool, error) {
	_, err := os.Lstat(path)

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

// Checks if a directory is empty.
func IsEmpty(name string) (bool, error) {
	f, err := os.Open(name)
	if err != nil {
		return false, err
	}
	defer f.Close()

	// Read at most 1 entry from the directory
	names, err := f.Readdirnames(1)

	if errors.Is(err, io.EOF) {
		return true, nil
	}

	return len(names) == 0, err
}
