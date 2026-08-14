package main

import (
	"fmt"
	"os"

	"nyeki.dev/nye/commands"

	"github.com/alecthomas/kong"
)

var CLI commands.Args

func main() {
	kongCtx := kong.Parse(&CLI)
	packageCtx, err := CLI.GetContext()
	if err != nil {
		fmt.Println("An error occurred:", err)
		os.Exit(1)
	}

	err = kongCtx.Run(packageCtx)

	if err != nil {
		fmt.Println("An error occurred:", err)
		os.Exit(1)
	}
}
