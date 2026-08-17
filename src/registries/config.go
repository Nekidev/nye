package registries

import (
	"fmt"
	"os"
	"slices"

	"github.com/pelletier/go-toml/v2"
	"nyeki.dev/nye/utils"
)

type Config struct {
	DefaultRegistry string           `toml:"default-registry"`
	Registries      []ConfigRegistry `toml:"registries"`
}

type ConfigRegistry struct {
	Url  string `toml:"url" validate:"required,url"`
	Name string `toml:"name" validate:"kebab-case"`
}

// Reads a config file from the specified file path.
//
// If the file path does not exist, an empty configuration is returned instead of an error. If the
// file fails to be read or the config is invalid, an error is returned.
//
// Returns:
// * `Config` - The read config, or an empty config if it does not exist.
// * `error` - An error, if one occurred while reading the config file.
func GetConfig(path string) (Config, error) {
	exists, err := utils.Exists(path)
	if err != nil {
		return Config{}, fmt.Errorf("could not check if config file existed: %v", err)
	}
	if !exists {
		return Config{}, nil
	}

	contents, err := os.ReadFile(path)
	if err != nil {
		return Config{}, fmt.Errorf("could not read config file: %v", err)
	}

	config := Config{}
	err = toml.Unmarshal(contents, &config)
	if err != nil {
		return Config{}, fmt.Errorf("could not parse config file: %v", err)
	}

	err = utils.ValidateStruct(&config)
	if err != nil {
		return Config{}, fmt.Errorf("config was not valid: %v", err)
	}

	err = validateDefultRegistry(&config)
	if err != nil {
		return Config{}, fmt.Errorf("config was not valid: %v", err)
	}

	return config, nil
}

func validateDefultRegistry(config *Config) error {
	if config.DefaultRegistry != "" {
		registryNames := []string{}

		for _, registry := range config.Registries {
			registryNames = append(registryNames, registry.Name)
		}

		if !slices.Contains(registryNames, config.DefaultRegistry) {
			return fmt.Errorf("the default registry, `%v`, was not defined as a registry under `registries`", config.DefaultRegistry)
		}
	}

	return nil
}
