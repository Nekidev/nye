package main

import (
	"fmt"
	"os"

	"nyeki.dev/nye/commands"
	"nyeki.dev/nye/utils"

	"github.com/alecthomas/kong"
	"github.com/fatih/color"
)

var CLI commands.Args

func main() {
	description := commands.PackageDescription
	currentTarget := utils.GetCurrentTarget()

	if !utils.IsSupportedTarget(currentTarget) {
		red := color.New(color.FgRed)

		description += "\n\n"
		description += red.Sprintf("Your current target, %v, is not supported by nye. You can use nye to create packages, you will not be able to install them (trying to do so will potentially break your system). Use an adequate system for better development guarantees.", currentTarget)
	}

	kongCtx := kong.Parse(&CLI,
		kong.Name("nye"),
		kong.Description(description),
		kong.UsageOnError(),
		kong.Vars{
			"current_target": currentTarget,
		},
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
