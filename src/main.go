package main

import (
	"fmt"
	"os"

	"nyeki.dev/nye/commands"

	"github.com/alecthomas/kong"
)

var CLI commands.Args

func main() {
	kongCtx := kong.Parse(&CLI,
		kong.Name("nye"),
		kong.Description("Nyeki's package manager."),
		kong.UsageOnError(),
	)

	packageCtx, err := CLI.GetContext()
	if err != nil {
		fmt.Println("An error occurred:", err)
		os.Exit(1)
	}

	err = kongCtx.Run(packageCtx, kongCtx)

	if err != nil {
		fmt.Println("An error occurred:", err)
		os.Exit(1)
	}
}
