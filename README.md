<p align="center">
<svg width="500" height="500" viewBox="0 0 500 500" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="500" height="500" rx="250" fill="#2B303B"/>
<rect x="171.232" y="129" width="26.2087" height="241.495" fill="#0A8754"/>
<rect x="183" y="129" width="198.437" height="26.2087" transform="rotate(39.3349 183 129)" fill="#00A6ED"/>
<rect x="161" y="349.154" width="198.437" height="26.2087" transform="rotate(-34.213 161 349.154)" fill="#EDAE49"/>
</svg>

<h3 align="center">Danivim - the only sane editor</h3>
</p>


<p align="center">
	<img src="https://github.com/vhakulinen/gnvim/wiki/completionmenu.png" alt="Logo">
</p>

For more screenshots, see [the wiki](https://github.com/vhakulinen/gnvim/wiki).

TL;DR to get started on Ubuntu 18.04 after cloning this repo and assuming
you have [rust tool chain](https://rustup.rs/) installed:

```
$ sudo apt install libgtk-3-dev libwebkit2gtk-4.0-dev
$ # Run (unoptimized version) without installing
$ GNVIM_RUNTIME_PATH=/path/to/gnvim/runtime cargo run
$ # Install
$ make && sudo make install
```

## macOS (without webkit2gtk)

Webkit2gtk isn't really available for macOS. GNvim is available without said
dependency, but such builds won't have the cursor tooltip feature. 

To install all dependencies and build without webkit2gtk (`gtk+3` required for 
building, `librsvg` is a runtime dependency for showing LSP icons in completion):

```bash
$ brew install gtk+3 librsvg
$ make NOWEBKIT2GTK=1
$ # or with cargo
$ cargo build --no-default-features
```

## Features

* No electron (!), build on GTK.
* Ligatures
* Animated cursor
* Custom cursor tooltip feature to display markdown documents.
  Useful for implementing features like hover information or signature help
  (see [gnvim-lsp](https://github.com/vhakulinen/gnvim-lsp)).
* A lot of the nvim external features implemented
    - Popupmenu
        * Own view for `preview` (`:h completeopt`).
    - Tabline
    - Cmdline
    - Wildmenu

More externalized features will follow as they are implemented for neovim.

## Requirements

GNvim requires

* Stable rust to compile
* Latest nvim release or master
* Gtk version 3.18 or higher

On some systems, Gtk packages doesn't include development files. On Ubuntu
18.04, you'll need the following ones:

```
$ sudo apt install libgtk-3-dev libwebkit2gtk-4.0-dev
```

For other systems, see requirements listed by gtk-rs project [here](https://gtk-rs.org/docs-src/requirements.html).
Note that you'll need the `libwebkit2gtk-4.0-dev` package too.

There are some benchmarks for internal data structures, but to run those you'll
need nightly rust. To run those benchmarks, use `cargo bench --features=unstable`
command.

# Install

You're required to have rust tool chain available. Once you have that, clone
this repo and run `make build` followed by `sudo make install`.

# Running

TL;DR: Without installing:

```
GNVIM_RUNTIME_PATH=/path/to/gnvim/runtime cargo run
```

GNvim requires some runtime files to be present and loaded by Neovim to work
properly. By default, GNvim will look for these files in `/usr/local/share/gnvim/runtime`,
but this can be changed by specifying the `GNVIM_RUNTIME_PATH` environment variable.

GNvim will use `nvim` to run Neovim by default. If you want to change that,
you can use `--nvim` flag (e.g. `gnvim --nvim=/path/to/nvim`).

For debugging purposes, there is `--print-nvim-cmd` flag to tell GNvim to print
the executed nvim command.

See `gnvim --help` for all the cli arguments.
