package utils

import "fmt"

// Clears printed lines from the terminal.
//
// It does not clear the current line.
func ClearLines(amount int) {
	for range amount {
		fmt.Print("\033[1A") // Go up one line
		fmt.Print("\033[2K") // Clear full line
	}

	fmt.Print("\r")
}
