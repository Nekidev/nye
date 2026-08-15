package utils

import (
	"errors"
	"fmt"
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
