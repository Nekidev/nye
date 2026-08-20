package dev

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/fatih/color"
	"github.com/vbauerster/mpb/v8"
	"github.com/vbauerster/mpb/v8/decor"

	"nyeki.dev/nye/projects"
	"nyeki.dev/nye/registries"
	"nyeki.dev/nye/registries/actions"
	"nyeki.dev/nye/utils"
)

type DevPublishCommand struct {
	Registry string `short:"r" help:"The name of the registry to publish to. Defaults to your default registry."`
}

func (cmd *DevPublishCommand) Run() error {
	config, err := registries.GetConfig()
	if err != nil {
		return fmt.Errorf("could not read registries config file: %v", err)
	}

	ctx, err := projects.GetContextCwd()
	if err != nil {
		return fmt.Errorf("could not get context for working directory's package: %v", err)
	}

	if len(config.Registries) == 0 {
		return fmt.Errorf(
			"no registries are configured, create a `%v` with a registry first",
			filepath.Join(utils.EnvNyeInstallationEtc, "registries.toml"),
		)
	}

	if cmd.Registry == "" {
		cmd.Registry = config.Default
	}

	var registry *registries.ConfigRegistry
	var registryNames []string

	for _, reg := range config.Registries {
		if reg.Name == cmd.Registry {
			registry = &reg
		}

		registryNames = append(registryNames, reg.Name)
	}

	if registry == nil {
		return fmt.Errorf("no registry named `%v` was found, options are `%v`", cmd.Registry, strings.Join(registryNames, "`, `"))
	}

	bundles := map[string]io.Reader{}
	progress := mpb.New(
		mpb.WithWidth(50),
	)

	blue := color.New(color.FgBlue)
	black := color.New(color.FgBlack)

	for target := range ctx.Manifest.Targets {
		distPath := projects.GetPackDistPath(ctx, target)
		file, err := os.Open(distPath)
		if err != nil {
			return fmt.Errorf("could not open package file at `%v`: %v", distPath, err)
		}

		stat, err := os.Stat(distPath)
		if err != nil {
			return fmt.Errorf("could not get stats for package file at `%v`: %v", distPath, err)
		}

		bar := progress.New(stat.Size(),
			mpb.BarStyle().
				Rbound(" ").
				Lbound(" ").
				Filler("━").
				FillerMeta(func(s string) string { return blue.Sprint(s) }).
				Refiller("━").
				RefillerMeta(func(s string) string { return black.Sprint(s) }).
				Tip("╺").
				TipMeta(func(s string) string { return blue.Sprint(s) }),
			mpb.PrependDecorators(
				decor.OnComplete(
					decor.Name(
						fmt.Sprintf("Uploading %v...", blue.Sprint(target)),
					),
					fmt.Sprintf("Uploaded %v!", blue.Sprint(target)),
				),
			),
			mpb.AppendDecorators(
				decor.EwmaETA(decor.ET_STYLE_GO, 30),
				decor.Name(" "),
				decor.Percentage(decor.WC{W: 5}),
				decor.Name(" "),
				decor.EwmaSpeed(decor.SizeB1024(0), "% .2f", 30),
			),
		)

		bundles[target] = bar.ProxyReader(file)
	}

	err = actions.PublishPackage(registry.Url, ctx.Manifest.Package.Name, ctx.Manifest.Package.Version, bundles)
	if err != nil {
		return fmt.Errorf("could not upload package version files: %v", err)
	}

	progress.Wait()

	fmt.Printf(
		"Done! All packs for %v were published to the registry.\n",
		blue.Sprintf("%v v%v", ctx.Manifest.Package.Name, ctx.Manifest.Package.Version),
	)

	return nil
}
