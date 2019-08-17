tmux-tanlog
===========
[![Build Status][]][CI Results]

`tmux-tanlog` is a tmux/zsh version of [tanlog](http://shinh.hatenablog.com/entry/2017/02/12/031105).

## Installation

```console
$ git clone https://github.com/r6eve/tmux-tanlog.git
$ cd tmux-tanlog
$ cargo install
```

And add the following to your .zshrc

```sh
tanlog_begin() { export TANLOG_LOGFILE=$(tmux-tanlog start "$1") }
tanlog_end() { tmux-tanlog end $TANLOG_LOGFILE }
typeset -Uga preexec_functions
typeset -Uga precmd_functions
preexec_functions+=tanlog_begin
precmd_functions+=tanlog_end
```

[Build Status]: https://travis-ci.org/r6eve/tmux-tanlog.svg?branch=master
[CI Results]: https://travis-ci.org/r6eve/tmux-tanlog
