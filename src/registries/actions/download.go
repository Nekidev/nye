package actions

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"slices"

	"github.com/Masterminds/semver/v3"
	"nyeki.dev/nye/utils"
)

// Downloads the package file for the latest version (that supports the current target) of a
// package.
//
// Arguments:
// * `registryURL` - The base URL of the registry, without path.
// * `packageName` - The name of the package to download.
// * `output` - The file to write the downloaded package file to.
//
// Returns:
// * `error` - An error, if any occurred.
func DownloadPackage(registryURL, packageName string, output io.Writer) error {
	versions, err := getVersions(registryURL, packageName)
	if err != nil {
		return fmt.Errorf("could not get versions for package `%v`: %v", packageName, err)
	}

	version, err := getLatestSupportedVersion(versions)
	if err != nil {
		return fmt.Errorf("an error occurred while getting the latest supported version for package `%v`: %v", packageName, err)
	}
	if version == nil {
		return fmt.Errorf("could not find a version that supports the current target for `%v`", packageName)
	}

	downloadURL, err := getDownloadURL(registryURL, packageName, version.String())
	if err != nil {
		return fmt.Errorf("could not get download URL for package `%v v%v`: %v", packageName, version, err)
	}

	err = downloadPackageFile(downloadURL, output)
	if err != nil {
		return fmt.Errorf("an error occurred while downloading the package: %v", err)
	}

	return nil
}

type version struct {
	Number  string   `json:"number"`
	Targets []string `json:"targets"`
}

type versionPage struct {
	Items []version `json:"items"`
}

func getVersions(registryURL, packageName string) ([]version, error) {
	url, err := url.JoinPath(registryURL, "/v1/packages", packageName, "versions")
	if err != nil {
		return nil, fmt.Errorf("could not join url parts to get versions for package `%v` from registry: %v", packageName, err)
	}

	response, err := http.Get(url)
	if err != nil {
		return nil, fmt.Errorf("could not get package versions for package `%v`: %v", packageName, err)
	}

	body, err := io.ReadAll(response.Body)
	if err != nil {
		return nil, fmt.Errorf("could not read response from registry when getting versions for package `%v`: %v", packageName, err)
	}

	var page versionPage
	err = json.Unmarshal(body, &page)
	if err != nil {
		return nil, fmt.Errorf("could not decode versions page: %v", err)
	}

	return page.Items, nil
}

func getLatestSupportedVersion(versions []version) (*semver.Version, error) {
	var latestVersion *semver.Version

	for _, version := range versions {
		versionNumber, err := semver.StrictNewVersion(version.Number)
		if err != nil {
			return nil, fmt.Errorf("an error occurred while parsing a version returned by the registry: %v", err)
		}

		if slices.Contains(version.Targets, utils.GetCurrentTarget()) {
			if latestVersion == nil {
				latestVersion = versionNumber
			} else if versionNumber.GreaterThan(latestVersion) {
				latestVersion = versionNumber
			}
		}
	}

	return latestVersion, nil
}

func getDownloadURL(registryURL, packageName, version string) (string, error) {
	URL, err := url.JoinPath(registryURL, "/v1/packages", packageName, "versions", version)
	if err != nil {
		return "", fmt.Errorf("could not join url parts to get download URLs for for package `%v v%v` from registry: %v", packageName, version, err)
	}

	response, err := http.Get(URL)
	if err != nil {
		return "", fmt.Errorf("could not get download URLs for package `%v v%v`: %v", packageName, version, err)
	}

	body, err := io.ReadAll(response.Body)
	if err != nil {
		return "", fmt.Errorf("could not read response from registry when getting download URLs for package `%v v%v`: %v", packageName, version, err)
	}

	var downloadURLs map[string]string
	err = json.Unmarshal(body, &downloadURLs)
	if err != nil {
		return "", fmt.Errorf("could not decode versions page: %v", err)
	}

	downloadURL := downloadURLs[utils.GetCurrentTarget()]

	if downloadURL == "" {
		return "", fmt.Errorf("the specified version (`%v v%v`) did not return a download URL for the current target", packageName, version)
	}

	return downloadURL, nil
}

// Downloads the contents of the specified URL to the specified local path in the system.
//
// Arguments:
//   - `downloadURL` - The URL to download the file from.
//   - `output` - The file to write the downloaded file to.
func downloadPackageFile(downloadURL string, output io.Writer) error {
	response, err := http.Get(downloadURL)
	if err != nil {
		return fmt.Errorf("could not download file `%v`: %v", downloadURL, err)
	}

	if response.StatusCode != 200 {
		return fmt.Errorf("downloading file at `%v` returned status code `%v`", downloadURL, response.Status)
	}

	_, err = io.Copy(output, response.Body)
	if err != nil {
		return fmt.Errorf("could not write response to file: %v", err)
	}

	return nil
}
