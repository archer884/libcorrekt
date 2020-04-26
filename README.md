# libcorrekt

Supports spellcheck logic for `correkt`.

## Operating system support

In theory, libcorrekt supports recent Windows versions and anything where you can get hunspell to build.

### Windows

On Windows machines, libcorrekt uses the OS spellchecker. The API is tough but fair---or, at least, I was able to implement it. Kind of.

### macOS/Unix

Five minutes into researching how to implement support for Apple's Cocoa/AppKit nonsense, I wanted to kill myself and all my unborn children. Rather than commit quantum, four-dimensional suicide, I decided to make use of hunspell-sys on both macOS and *nix platforms.

The API presented by Hunspell is decidedly less nonpessimal than `ISpellChecker` on Windows and, as far as I can tell, does not even kind of play well with the idea of laziness. libcorrekt pretends not to care about this and, therefore, continues to present suggestions as an iterator even when they come from an array of character arrays. Or pointers.

Whatever, just shoot me now.

## Building libcorrekt

On Windows systems, you're done. If you're on macOS or some posix-like operating system, keep reading.

### dotenv

To make this work, it is necessary for you to include a `.env` file pointing to the location on your system where appropriate hunspell dictionaries are accessible. I haven't included my dotenv file because it would be useless to you, but yours should wind up looking something like this, assuming you want to spellcheck in English:

```
DICTIONARY_PATH=/path/to/dictionaries/en
```

This path should point to a directory containing both the aff and dic files, because the following code is used to actually pull the files:

```rust
fn build_path(path: impl AsRef<Path>) -> PathBuf {
    let root: &Path = DICTIONARY_PATH.as_ref();
    root.join(path)
}
```

```rust
let aff = build_path("index.aff");
let dic = build_path("index.dic");
```

If I'm honest, I'm not totally sure the .env file is necessary; it's possible the program will work if you just have the actual environment variables defined when you build it, or when you run it; one of these days I'm going to look that up. In the meantime, you can [read all about it](https://crates.io/crates/dotenv).

### Hunspell

The real sticking point here is hunspell itself. If already have hunspell installed on your system, it is possible that building this will be a non-issue for you. If not, the best advice probably comes from the good people over at [hunspell](https://github.com/hunspell/hunspell).

Unfortunately, I didn't go about this scientifically, but I believe that their advice to run the following brew commands is what saved my bacon:

```shell
brew install autoconf automake libtool gettext
brew link gettext --force
```

My build didn't immediately start working after that; it seemed like it took a few consecutive calls to `cargo build` to prime the pump, so to speak.

Good luck.
