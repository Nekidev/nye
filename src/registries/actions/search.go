package actions

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"

	"nyeki.dev/nye/utils"
)

type SearchResult struct {
	PackageId     string `json:"package_id"`
	PackageName   string `json:"package_name"`
	BundleVersion string `json:"bundle_version"`
	BundleTarget  string `json:"bundle_target"`
	BundleSize    uint64 `json:"bundle_size"`
	BundleURL     string `json:"bundle_url"`
}

type searchResultPage struct {
	Items []SearchResult `json:"items"`
	Total uint           `json:"total"`
}

func SearchPackages(registryURL string, names []string) ([]SearchResult, error) {
	queryString := "target=" + utils.GetCurrentTarget()

	for _, name := range names {
		queryString += "&query=" + url.QueryEscape(name)
	}

	url, err := url.JoinPath(registryURL, "/v1/search")
	if err != nil {
		return nil, fmt.Errorf("could not join url parts to search packages from registry: %v", err)
	}

	response, err := http.Get(url + "?" + queryString)
	if err != nil {
		return nil, fmt.Errorf("could not get search results: %v", err)
	}

	body, err := io.ReadAll(response.Body)
	if err != nil {
		return nil, fmt.Errorf("could not read response from registry when getting search results: %v", err)
	}

	var page searchResultPage
	err = json.Unmarshal(body, &page)
	if err != nil {
		return nil, fmt.Errorf("could not decode search results page: %v", err)
	}

	return page.Items, nil

}
