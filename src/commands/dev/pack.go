package dev

import (
	"fmt"
	"maps"
	"os"
	"path/filepath"
	"slices"

	"github.com/fatih/color"
	"nyeki.dev/nye/projects"
	"nyeki.dev/nye/projects/actions"
	"nyeki.dev/nye/utils"
)

type DevPackCommand struct {
	Targets []string "short:\"t\" default:\"current\" help:\"The targets for which to pack the package for. May include individual targets, `current`, or `all`.\""
}

func (cmd *DevPackCommand) Validate() error {
	if len(cmd.Targets) == 0 {
		return fmt.Errorf("there must be at least one target specified to pack for")
	}

	for i, target := range cmd.Targets {
		isAll := target == "all"
		isCurrent := target == "current"
		isSupported := utils.IsSupportedTarget(target)

		if !isAll && !isCurrent && !isSupported {
			return fmt.Errorf("the target %v is invalid", target)
		}

		if target == "all" && len(cmd.Targets) != 1 {
			return fmt.Errorf("`--targets=all` cannot be used with other individual targets")
		}

		if slices.Index(cmd.Targets, target) != i {
			return fmt.Errorf("each target can only be specified once, but you specified %v twice or more", target)
		}
	}

	return nil
}

func (cmd *DevPackCommand) Run() error {
	ctx, err := projects.GetContextCwd()
	if err != nil {
		return fmt.Errorf("could not get context for working directory's package: %v", err)
	}

	if cmd.Targets[0] == "all" {
		cmd.Targets = slices.Collect(maps.Keys(ctx.Manifest.Targets))
	}

	packed := []struct{target string; path string}{}

	for _, target := range cmd.Targets {
		if target == "current" {
			target = utils.GetCurrentTarget()
		}

		for _, packed := range packed {
			if packed.target == target {
				continue
			}
		}

		zip, err := actions.PackProject(ctx, target)
		if err != nil {
			return fmt.Errorf("could not pack project for target `%v`: %v", target, err)
		}

		packed = append(packed, struct{target string; path string}{
			target: target,
			path: zip,
		})
	}

	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("could not get current working directory after packing all packages: %v", err)
	}

	fmt.Println("Done! The following packages were generated:")
	blue := color.New(color.FgBlue)

	for i, file := range packed {
		rel, err := filepath.Rel(cwd, file.path)
		if err != nil {
			return fmt.Errorf("could not get relative path for zip file: %v", err)
		}

		fmt.Printf("%v. %v %v\n", i+1, file.target, blue.Sprint(rel))
	}

	return nil
}
