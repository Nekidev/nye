package packages

import (
	"fmt"
	"strings"

	"github.com/lithammer/dedent"
)

// A shell script wrapper for binaries.
//
// It allows to declare env variables at runtime.
//
// Call `GetContents()` to generate the contents of the script.
type BinaryWrapper struct {
	Path            string            // The absolute path to the binary being wrapped.
	DefinedEnvVars  map[string]string // Environment variables that will be defined when running the binary.
	ConsumedEnvVars map[string]string // Environment variables that will be consumed when running the binary. Maps variable names to separator.
}

// Generates the shell script's contents from the wrapper's configuration.
func (wrapper BinaryWrapper) GetContents(ctx Context) string {
	var contents strings.Builder

	contents.WriteString("#!/bin/sh\n")

	if len(wrapper.DefinedEnvVars) > 0 {
		contents.WriteString("\n")

		for name, value := range wrapper.DefinedEnvVars {
			contents.WriteString(getEnvVarDeclaration(name, value))
			contents.WriteString("\n")
		}
	}

	if len(wrapper.ConsumedEnvVars) > 0 {
		contents.WriteString(dedent.Dedent(`
			if ! command -v envsubst >/dev/null 2>&1; then
				echo "error: envsubst is required to run this package" >&2
				exit 127
			fi

			compose() {
				namespace="$1"
				var="$2"
				separator="$3"
				result=""

				for package in "$namespace/pkg/env/$var/"*; do
					[ -d "$package" ] || continue

					for version in "$package"/*; do
						[ -f "$version" ] || continue

						value="$(envsubst < "$version")"

						if [ -n "$result" ]; then
							result="$result$separator$value"
						else
							result="$value"
						fi
					done
				done

				printf '%s' "$result"
			}
		`))
		contents.WriteString("\n")

		for name, separator := range wrapper.ConsumedEnvVars {
			contents.WriteString(getEnvVarCompose(ctx.Path, name, separator))
			contents.WriteString("\n")
		}
	}

	fmt.Fprintf(&contents, "\nexec %v \"$@\"\n", escapeShellString(wrapper.Path))

	return contents.String()
}

func getEnvVarDeclaration(name, value string) string {
	return fmt.Sprintf("export %v=%v", escapeShellString(name), escapeShellString(value))
}

func getEnvVarCompose(namespace, name, separator string) string {
	return fmt.Sprintf("export %v=\"$(compose %v %v %v)\"", escapeShellString(name), escapeShellString(namespace), escapeShellString(name), escapeShellString(separator))
}

// Escapes a shell string's value to keep the meaning without breaking shell syntax.
//
// For example:
// * `\` -> `"\\"`: The last slash would escape the last double quote.
// * `"hi"` -> `"\"hi\""`: Escapes double quotes.
// * `'hi'` -> `"'hi'"`: Single quotes don't need to be escaped.
// * `/bin:$PATH` -> `"/bin:$PATH"`: Meaning is kept.
func escapeShellString(value string) string {
	value = strings.ReplaceAll(value, "\"", "\\\"")

	if strings.HasSuffix(value, "\\") {
		value += "\\"
	}

	return "\"" + value + "\""
}
