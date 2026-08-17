package utils

import (
	"io/fs"
	"path/filepath"
	"regexp"
	"slices"
	"strings"

	"github.com/Masterminds/semver/v3"
	"github.com/go-playground/validator/v10"
)

// Validates a struct with additional validators.
//
// The additional validators are:
// * `kebab-case` - Enforces kebab-case on a string value.
// * `semver` - Enforces semver compliance on a string value, without `v` prefix.
//
// Arguments:
// * `s` - The struct to validate.
//
// Returns:
// * `error` - If the passed struct is invalid.
// * `nil` - Otherwise.
func ValidateStruct(s interface{}) error {
	validate := validator.New()

	validate.RegisterValidation("kebab-case", IntoValidator(IsKebabCase))
	validate.RegisterValidation("semver", IntoValidator(IsSemver))
	validate.RegisterValidation("safe-path", IntoValidator(IsSafePath))
	validate.RegisterValidation("safe-path-segment", IntoValidator(IsSafePathSegment))
	validate.RegisterValidation("supported-target", IntoValidator(IsSupportedTarget))
	validate.RegisterValidation("env-var-name", IntoValidator(IsEnvVarName))

	return validate.Struct(s)
}

func IntoValidator(f func(string) bool) func(validator.FieldLevel) bool {
	inner := func(field validator.FieldLevel) bool {
		value := field.Field().String()
		return f(value)
	}

	return inner
}

func IsKebabCase(value string) bool {
	re := regexp.MustCompile(`^[a-z0-9]+(?:-[a-z0-9]+)*$`)

	return re.MatchString(value)
}

// Enforces semver compliance on a string value.
//
// This function only allows valid semver v2 semantic versions.
//
// Returns:
// * `true` - If the field value is a valid semver version.
// * `false` -  Otherwise.
func IsSemver(value string) bool {
	if _, err := semver.StrictNewVersion(value); err != nil {
		return false
	}

	return true
}

// Makes sure a path is good for exports.
//
// Rules:
// * Relative paths only.
// * Invalid if normalization is needed. E.g. `../`, `./`, and `//` are not considered valid.
// * A call to `fs.ValidPath` is done at the end to make sure the path is valid.
//
// Returns:
// * `true` - If the path is safe.
// * `false` - Otherwise.
func IsSafePath(value string) bool {
	if filepath.IsAbs(value) {
		return false
	}

	if filepath.Clean(value) != value {
		return false
	}

	if !fs.ValidPath(value) {
		return false
	}

	for _, segment := range strings.Split(value, "/") {
		if !IsSafePathSegment(segment) {
			return false
		}
	}

	return true
}

// Validates a single path segment.
//
// Rules:
// * No `/`
// * Invalid if normalization is needed. E.g. `..` and `.` are not considered valid.
//
// Returns:
// * `true` - If the string is a valid single path segment.
// * `false` - Otherwise.
func IsSafePathSegment(value string) bool {
	if filepath.Clean(value) != value {
		return false
	}

	if strings.ContainsRune(value, '/') {
		return false
	}

	if value == ".." || value == "." {
		return false
	}

	return true
}

func IsSupportedTarget(value string) bool {
	return slices.Contains(SupportedTargets, value)
}

func IsEnvVarName(value string) bool {
	matches, err := regexp.Match("^[a-zA-Z0-9_]+$", []byte(value))
	if err != nil {
		panic(err)
	}

	return matches
}
