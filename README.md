# git-branches-overview

Visualize branches 'ahead' and 'behind' commits compared to a base revision or their upstream.

## Installation

Use [Cargo](https://doc.rust-lang.org/cargo/index.html).

```
$ cargo install git-branches-overview
```

You may need to add `$HOME/.cargo/bin` in your `PATH`.

## Usage

```
git-branches-overview [FLAGS] [OPTIONS] [--] [base_revision]

FLAGS:
    -l                 Show local branches (default)
    -r                 Show remote branches
    -a                 Show all branches
    -u, --upstreams    Compare branches with their respective upstream instead of the default branch
    -h, --help         Prints help information
    -V, --version      Prints version information

OPTIONS:
        --remote <remote_name>...    Only list branches from those remotes;  can be specified multiple times;  implies
                                     '-r'
        --repo-dir <path>            Repository path [default: .]

ARGS:
    <base_revision>    Revision to use as a base [default: HEAD]
```

## Screenshot

![Screenshot of git-branches-overview](https://raw.githubusercontent.com/BenoitZugmeyer/git-branches-overview/master/git-branches-overview.png)
