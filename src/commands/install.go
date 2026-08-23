package commands

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/fatih/color"
	"github.com/vbauerster/mpb/v8"
	"github.com/vbauerster/mpb/v8/decor"
	"nyeki.dev/nye/packages"
	pactions "nyeki.dev/nye/packages/actions"
	"nyeki.dev/nye/registries"
	ractions "nyeki.dev/nye/registries/actions"
	"nyeki.dev/nye/utils"
)

type InstallCommand struct {
	Names []string "arg:\"\" help:\"The names of the packages to download and install. They can be prefixed by `registry-name/` to specify a registry, otherwise the default registry will be used.\""
	Paths []string `flag:"path" short:"p" help:"The path to the package's zip file." type:"existingPath"`
}

func (cmd *InstallCommand) Run(ctx packages.Context) error {
	config, err := registries.GetConfig()
	if err != nil {
		return fmt.Errorf("could not read registries configuration: %v", err)
	}

	packagesPerRegistry, err := organizeReferences(config, cmd.Names)
	if err != nil {
		return fmt.Errorf("could not organize references: %v", err)
	}

	// Use pkg/tmp instead of tmp not to place big files in tmpfs (RAM)
	dirPath := filepath.Join(ctx.Path, "pkg", "tmp")
	err = os.MkdirAll(dirPath, 0o700)
	if err != nil {
		return fmt.Errorf("could not create temporary directory for downloads at `%v`: %v", dirPath, err)
	}
	defer os.RemoveAll(dirPath)

	results, err := search(config, packagesPerRegistry)
	if err != nil {
		return fmt.Errorf("an error occurred while searching for the specified packages: %v", err)
	}

	temporaryPaths, err := downloadAll(results, dirPath)
	if err != nil {
		return fmt.Errorf("an error occurred while downloading files from a registry: %v", err)
	}

	manifests, err := installAll(ctx, append(temporaryPaths, cmd.Paths...))
	if err != nil {
		return fmt.Errorf("could not install a package: %v", err)
	}

	index := 1
	blue := color.New(color.FgBlue)
	fmt.Println("Done! The following packages have been installed:")
	for _, manifest := range manifests {
		fmt.Printf("%v. %v\n", index, blue.Sprintf("%v v%v", manifest.Package.Name, manifest.Package.Version))
		index += 1
	}

	return nil
}

func organizeReferences(config registries.Config, references []string) (map[string][]string, error) {
	packagesPerRegistry := map[string][]string{}

	for _, reference := range references {
		slashes := strings.Count(reference, "/")

		var name string
		var registry string

		switch slashes {
		case 0:
			if config.Default == "" {
				return nil, fmt.Errorf("the package reference `%v` did not specify a registry to use and there's no default registry set up", reference)
			}

			name = reference
			registry = config.Default
		case 1:
			parts := strings.SplitN(reference, "/", 2)

			registry = parts[0]
			name = parts[1]

			_, ok := config.Registries[registry]
			if !ok {
				return nil, fmt.Errorf("the specified registry `%v` is not defined in `registries.toml`, did you make a typo?", registry)
			}
		default:
			return nil, fmt.Errorf("the package reference `%v` is not valid, only 1 slash is allowed", reference)
		}

		ppr, ok := packagesPerRegistry[registry]
		if !ok {
			packagesPerRegistry[registry] = []string{name}
		} else {
			packagesPerRegistry[registry] = append(ppr, name)
		}
	}

	return packagesPerRegistry, nil
}

type searchResult struct {
	Results      []ractions.SearchResult
	Error        error
	RegistryName string
}

func search(config registries.Config, packagesPerRegistry map[string][]string) ([]ractions.SearchResult, error) {
	channel := make(chan searchResult, len(packagesPerRegistry))

	for registryName, packages := range packagesPerRegistry {
		registry := config.GetRegistry(registryName)
		if registry == nil {
			return nil, fmt.Errorf("the provided package registry name `%v` is not a configured registry", registryName)
		}

		go searchInRegistry(registryName, registry.URL, packages, channel)
	}

	collection := []ractions.SearchResult{}

	for range packagesPerRegistry {
		result := <-channel

		if result.Error != nil {
			return nil, fmt.Errorf("could not search for packages in registry `%v`: %v", result.RegistryName, result.Error)
		}

		collection = append(collection, result.Results...)
	}

	return collection, nil
}

func searchInRegistry(registryName, registryURL string, names []string, channel chan searchResult) {
	results, err := ractions.SearchPackages(registryURL, names)
	channel <- searchResult{
		Results:      results,
		Error:        err,
		RegistryName: registryName,
	}
}

type downloadResult struct {
	Result string // The path to the downloaded file.
	Error  error
}

// Downloads all the search results into temporary files in the specified download dir.
//
// Returns:
// * `[]string` - An array of paths to each of the downloaded files.
// * `error` - If an error occurred during any of the downloads.
func downloadAll(searchResults []ractions.SearchResult, downloadDir string) ([]string, error) {
	// TODO: Allow concurrency limits to be customized.
	semaphore := make(chan struct{}, 5)
	progress := mpb.New(mpb.WithWidth(50))
	downloadResults := make(chan downloadResult, len(searchResults))

	for _, result := range searchResults {
		go func() {
			// Backpressure, only 5 can be in the channel's queue at the same time, making this
			// behave like a semaphore throttling concurrent downloads.
			semaphore <- struct{}{}
			defer func() { <-semaphore }()

			bar := getDownloadBar(progress, int64(result.BundleSize), result.PackageName)

			file, err := os.CreateTemp(downloadDir, "*.zip")
			if err != nil {
				downloadResults <- downloadResult{
					Error: fmt.Errorf("could not create temporary file for download: %v", err),
				}
				return
			}
			defer file.Close()

			err = download(result, bar.ProxyWriter(file))
			if err != nil {
				defer os.Remove(file.Name())
				downloadResults <- downloadResult{
					Error: fmt.Errorf("could not download package file for `%v`: %v", result.PackageName, err),
				}
				return
			}
			bar.SetTotal(int64(result.BundleSize), true)

			downloadResults <- downloadResult{
				Result: file.Name(),
			}
		}()
	}

	results := []string{}
	for range searchResults {
		result := <-downloadResults

		if result.Error != nil {
			return nil, fmt.Errorf("could not download package file: %v", result.Error)
		}

		results = append(results, result.Result)
	}

	progress.Wait()
	utils.ClearLines(len(searchResults))

	return results, nil
}

func download(result ractions.SearchResult, output io.Writer) error {
	response, err := http.Get(result.BundleURL)
	if err != nil {
		return fmt.Errorf("could not download file `%v`: %v", result.BundleURL, err)
	}

	if response.StatusCode != 200 {
		return fmt.Errorf("downloading file at `%v` returned status code `%v`", result.BundleURL, response.Status)
	}

	_, err = io.Copy(output, response.Body)
	if err != nil {
		return fmt.Errorf("could not write response to file: %v", err)
	}

	return nil
}

func getDownloadBar(progress *mpb.Progress, size int64, name string) *mpb.Bar {
	blue := color.New(color.FgBlue)
	black := color.New(color.FgHiBlack)

	return progress.New(size,
		mpb.BarStyle().
			Rbound(" ").
			Lbound(" ").
			Filler("━").
			Refiller("━").
			RefillerMeta(func(s string) string { return black.Sprint(s) }).
			Tip("╺"),
		mpb.PrependDecorators(
			decor.OnComplete(
				decor.Name(
					fmt.Sprintf("Downloading %v...", blue.Sprint(name)),
				),
				fmt.Sprintf("Downloaded %v!", blue.Sprint(name)),
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
}

func installAll(ctx packages.Context, paths []string) ([]packages.Manifest, error) {
	results := []packages.Manifest{}

	for _, path := range paths {
		manifest, err := pactions.InstallPackage(ctx, path)
		if err != nil {
			return nil, fmt.Errorf("could not install package at `%v`: %v", path, err)
		}
		results = append(results, manifest)
	}

	return results, nil
}
