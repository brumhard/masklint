# masklint 🥸

Lint your [mask](https://github.com/jacobdeichert/mask) targets.

<details>
<summary><h2>Installation</h3></summary>

### From source

If you have `cargo` installed you can just run the following.
Make sure that you have added Cargo's bin directory (e.g. `~/.cargo/bin`) to your `PATH`.

```shell
cargo install --git https://github.com/brumhard/maskfile.git --tag latest
```

### Released binaries/packages

Download the desired version for your operating system and processor architecture from the [releases](https://github.com/brumhard/maskfile/releases).
Make the file executable and place it in a directory available in your `$PATH`.

### Use with nix

```shell
nix run github:brumhard/maskfile/latest
```

or

```nix
{
    inputs.maskfile.url = "github:brumhard/maskfile/latest";

    outputs = { maskfile, ... }: {
        packages.x86_64-linux = [maskfile.packages.x86_64-linux.rl];
    };
}
```

### Homebrew

```shell
brew install brumhard/tap/maskfile
```

</details>

## Features

```shell
masklint run # lints all supported script blocks in the maskfile.md
masklint run --maskfile /path/to/some/file # lints maskfile in another dir
masklint dump -o ./test # dumps all targets as seperate files to ./test
```

Supported languages and used linters:

- `bash`, `sh`, `zsh` using [shellcheck](https://github.com/koalaman/shellcheck)
- `python` using [pylint](https://github.com/pylint-dev/pylint)
- `ruby` using [rubocop](https://github.com/rubocop/rubocop)

> **Warning**
> The linters are not bundled so make sure that the needed ones are installed and in the `PATH`

## Example

The [testing `maskfile`](test/maskfile.md) produces the following outputs:

```shell
$ masklint run --maskfile test/maskfile.md
bash
In line 2:
mkdir $unset
      ^----^ SC2154 (warning): unset is referenced but not assigned (for output from commands, use "$(unset ...)" ).
      ^----^ SC2086 (info): Double quote to prevent globbing and word splitting.

Did you mean: 
mkdir "$unset"

For more information:
  https://www.shellcheck.net/wiki/SC2154 -- unset is referenced but not assig...
  https://www.shellcheck.net/wiki/SC2086 -- Double quote to prevent globbing ...

python
line 2:0: W0301: Unnecessary semicolon (unnecessary-semicolon)

ruby
line 1:1: C: [Correctable] Style/FrozenStringLiteralComment: Missing frozen string literal comment.
name = ENV["name"] || "WORLD"
^
line 1:12: C: [Correctable] Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols. (https://rubystyle.guide#consistent-string-literals)
name = ENV["name"] || "WORLD"
           ^^^^^^
line 1:23: C: [Correctable] Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols. (https://rubystyle.guide#consistent-string-literals)
name = ENV["name"] || "WORLD"
                      ^^^^^^^
```
