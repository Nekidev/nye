package main

import (
	"fmt"
	"os"

	"nyeki.dev/nye/commands"

	"github.com/alecthomas/kong"
)

var CLI commands.Args

func main() {
	ctx := kong.Parse(&CLI)
	err := ctx.Run()

	if err != nil {
		fmt.Println("An error occurred:", err)
		os.Exit(1)
	}
}
