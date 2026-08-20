package actions

import (
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"net/url"
)

// A map of target strings to bundle file readers
type Bundles = map[string]io.Reader

func PublishPackage(registryUrl, name, version string, bundles Bundles) error {
	registryUrl, err := url.JoinPath(registryUrl, "/v1/packages/", name, "versions", version)
	if err != nil {
		return fmt.Errorf("could not create package upload URL: %v", err)
	}

	pipeRx, pipeTx := io.Pipe()
	writer := multipart.NewWriter(pipeTx)

	go func() {
		defer pipeTx.Close()
		defer writer.Close()

		for target, reader := range bundles {
			part, err := writer.CreateFormFile("bundles", fmt.Sprintf("%v.zip", target))
			if err != nil {
				pipeTx.CloseWithError(err)
				return
			}

			_, err = io.Copy(part, reader)
			if err != nil {
				pipeTx.CloseWithError(err)
				return
			}
		}
	}()

	response, err := http.Post(registryUrl, writer.FormDataContentType(), pipeRx)
	if err != nil {
		return fmt.Errorf("could not upload package file: %v", err)
	}

	if response.StatusCode != 200 {
		body, err := io.ReadAll(response.Body)
		if err != nil {
			return fmt.Errorf("the package registry returned an unsuccessful status code, `%v`, and the response body could not be read: %v", response.Status, err)
		}

		return fmt.Errorf("the package registry returned an unsuccessful status code, `%v`: %v", response.Status, string(body))
	}

	return nil
}
