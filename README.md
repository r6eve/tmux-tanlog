tmux-tanlog
===========
[![Build Status][]][CI Results]

`tmux-tanlog` is a tmux/zsh version of [tanlog][] (It's original GNU Screen version).

This tool saves console outputs into text files in `/tmp/tanlog/`. It depends on `tmux` and `zsh`.

## Installation

### From source codes

```console
$ git clone https://github.com/r6eve/tmux-tanlog.git
$ cd tmux-tanlog
$ cargo install --path .
```

### From executable binaries

See [Releases][]. There are two binaries; `*-gnu.tar.xz` is dynamically linked and `*-musl.tar.xz` is statically linked.

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

[Build Status]: https://travis-ci.org/r6eve/tmux-tanlog.svg?branch=master
[CI Results]: https://travis-ci.org/r6eve/tmux-tanlog
[tanlog]: http://shinh.hatenablog.com/entry/2017/02/12/031105
[Releases]: https://github.com/r6eve/tmux-tanlog/releases
