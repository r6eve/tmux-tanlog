tmux-tanlog
===========
[![Build Status][]][CI Results]

`tmux-tanlog` is a tmux/zsh version of [tanlog][] (It's original GNU Screen
version).

This tool saves console outputs into text files in `/tmp/tanlog/`. It depends
on `tmux` and `zsh`.

## Installation

### From source codes

```console
$ git clone https://github.com/r6eve/tmux-tanlog.git
$ cd tmux-tanlog
$ cargo install --path .
```

### From executable binaries

See [Releases][]. `tmux-tanlog-x86_64-unknown-linux-musl` is statically linked
binary.

#### Arch Linux

```console
yay -S tmux-tanlog-bin
```

## Settings

And add the following to your .zshrc

```sh
tanlog_begin() { export TANLOG_LOGFILE=$(tmux-tanlog start "$1") }
tanlog_end() { tmux-tanlog end $TANLOG_LOGFILE }
typeset -Uga preexec_functions
typeset -Uga precmd_functions
preexec_functions+=tanlog_begin
precmd_functions+=tanlog_end
```

Output directory defaults to `/tmp/tanlog`. Set the environment variable
`TANLOG_DIR` if you want to change it.

[Build Status]: https://github.com/r6eve/tmux-tanlog/workflows/main/badge.svg
[CI Results]: https://github.com/r6eve/tmux-tanlog/actions
[tanlog]: http://shinh.hatenablog.com/entry/2017/02/12/031105
[Releases]: https://github.com/r6eve/tmux-tanlog/releases
