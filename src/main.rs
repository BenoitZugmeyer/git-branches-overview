use git2::{Branch, BranchType, Oid, Repository};
use prettytable::{format::TableFormat, Cell, Row, Table};
use std::{fmt::Write, iter::repeat, path::PathBuf};
use structopt::{clap::AppSettings, StructOpt};

/// Visualize branches 'ahead' and 'behind' commits compared to a base revision or their upstream.
#[derive(StructOpt, Debug)]
#[structopt(
    author = "",
    after_help = "\
EXAMPLES:

    # Compare all branches with development
    git-branches-overview -a development

    # Compare local branches with their upstreams
    git-branches-overview -u
    ",
    raw(global_settings = "&[AppSettings::DeriveDisplayOrder, AppSettings::ColoredHelp]")
)]
struct Opt {
    /// Revision to use as a base
    #[structopt(name = "base_revision", default_value = "HEAD")]
    base_revision: String,

    /// Show local branches (default)
    #[structopt(short = "l")]
    local_branches: bool,

    /// Show remote branches
    #[structopt(short = "r")]
    remote_branches: bool,

    /// Show all branches
    #[structopt(short = "a")]
    all_branches: bool,

    /// Compare branches with their respective upstream instead of the default branch
    #[structopt(short = "u", long = "--upstreams")]
    compare_with_upstream_branches: bool,

    /// Only list branches from those remotes;  can be specified multiple times;  implies '-r'
    #[structopt(long = "remote", name = "remote_name", number_of_values = 1)]
    remotes: Vec<String>,

    /// Repository path
    #[structopt(
        long = "repo-dir",
        name = "path",
        default_value = ".",
        parse(from_os_str)
    )]
    repo_path: PathBuf,
}

const BRANCH_CHARACTERS_COUNT: usize = 16;

fn number_size(mut n: usize) -> usize {
    let mut result = 1;
    while n >= 10 {
        result += 1;
        n /= 10;
    }
    result
}

fn branch_size(commits_count: usize, max_commits_count: usize) -> (usize, bool) {
    let ratio = commits_count as f64 / max_commits_count as f64;
    let floating_size =
        (ratio * std::f64::consts::PI / 2.).sin().sqrt() * BRANCH_CHARACTERS_COUNT as f64;
    // let floating_size = (1. - (1. - ratio).powf(4.)) * BRANCH_CHARACTERS_COUNT as f64;
    let floating_part = floating_size - floating_size.floor();
    (
        floating_size.ceil() as usize,
        floating_part > 0. && floating_part <= 0.5,
    )
}

struct FormatedBranch {
    last_commit_time: i64,
    name: String,
    remote: Option<String>,
    behind: usize,
    ahead: usize,
}

impl FormatedBranch {
    fn from_branch(
        repo: &Repository,
        branch: &Branch,
        opt: &Opt,
        default_target: Oid,
    ) -> Option<Self> {
        let full_name = branch.get().name()?;

        let (name, remote) = if full_name.starts_with("refs/remotes/") {
            let mut parts = full_name.splitn(4, '/');
            let remote_name = parts.nth(2)?.into();

            // Only keep selected remotes, if needed
            if !opt.remotes.is_empty() && !opt.remotes.contains(&remote_name) {
                return None;
            }

            (parts.next()?.into(), Some(remote_name))
        } else if full_name.starts_with("refs/heads/") {
            (full_name[11..].into(), None)
        } else {
            return None;
        };

        let target = if opt.compare_with_upstream_branches {
            branch.upstream().ok()?.get().target()?
        } else {
            default_target
        };

        let (ahead, behind) = repo
            .graph_ahead_behind(branch.get().target()?, target)
            .ok()?;

        Some(Self {
            last_commit_time: branch
                .get()
                .peel_to_commit()
                .ok()?
                .author()
                .when()
                .seconds(),
            remote,
            name,
            behind,
            ahead,
        })
    }

    fn format_chart_line(&self, max: usize) -> String {
        let mut result = String::new();

        // First half
        {
            let (behind_size, behind_half) = branch_size(self.behind, max);

            result.extend(repeat(' ').take(
                BRANCH_CHARACTERS_COUNT + number_size(max) - number_size(self.behind) - behind_size,
            ));

            write!(result, "{} ", self.behind).unwrap();

            if behind_half {
                result.push('╺');
                result.extend(repeat('━').take(behind_size - 1));
            } else {
                result.extend(repeat('━').take(behind_size));
            }
        }

        // Middle bar
        result.push(if self.behind == 0 && self.ahead == 0 {
            '│'
        } else if self.behind == 0 {
            '┝'
        } else if self.ahead == 0 {
            '┥'
        } else {
            '┿'
        });

        // Second half
        {
            let (ahead_size, ahead_half) = branch_size(self.ahead, max);

            if ahead_half {
                result.extend(repeat('━').take(ahead_size - 1));
                result.push('╸');
            } else {
                result.extend(repeat('━').take(ahead_size));
            }

            write!(result, " {}", self.ahead).unwrap();

            result.extend(repeat(' ').take(
                number_size(max) - number_size(self.ahead) + BRANCH_CHARACTERS_COUNT - ahead_size,
            ));
        }

        result
    }
}

fn compare_branches(a: &FormatedBranch, b: &FormatedBranch) -> std::cmp::Ordering {
    // Compare commit authoring date
    b.last_commit_time
        .cmp(&a.last_commit_time)
        // Compare remotes
        .then_with(|| match (a.remote.as_ref(), b.remote.as_ref()) {
            (Some(remote_a), Some(remote_b)) => remote_a.cmp(remote_b),
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
        // Compare names
        .then_with(|| a.name.cmp(&b.name))
}

#[derive(Debug)]
enum CliError {
    GitError(git2::Error),
}

impl From<git2::Error> for CliError {
    fn from(error: git2::Error) -> Self {
        CliError::GitError(error)
    }
}

fn run() -> Result<(), CliError> {
    let mut opt = Opt::from_args();

    if !opt.remotes.is_empty() {
        opt.remote_branches = true;
    }

    let repo = Repository::open(&opt.repo_path)?;
    let default_target = repo.revparse_single(&opt.base_revision)?.id();

    let mut branches: Vec<_> = repo
        .branches(
            if opt.all_branches || (opt.remote_branches && opt.local_branches) {
                None
            } else if opt.remote_branches {
                Some(BranchType::Remote)
            } else {
                Some(BranchType::Local)
            },
        )?
        .filter_map(|result| {
            FormatedBranch::from_branch(&repo, &result.ok()?.0, &opt, default_target)
        })
        .collect();

    branches.sort_by(compare_branches);

    let mut table = Table::new();
    let mut format = TableFormat::new();
    format.padding(1, 1);
    format.column_separator('·');
    table.set_format(format);

    let max = branches
        .iter()
        .map(|branch| branch.ahead.max(branch.behind))
        .max()
        .unwrap()
        .max(1);

    for branch in branches.iter() {
        let mut row = Vec::new();

        if opt.all_branches || opt.remote_branches {
            row.push(
                Cell::new(branch.remote.as_ref().map_or("local", |remote| remote)).style_spec(
                    if branch.remote.is_none() {
                        "Fgb"
                    } else {
                        "Frb"
                    },
                ),
            );
        }
        row.push(Cell::new(&branch.name));
        row.push(Cell::new(&branch.format_chart_line(max)));

        table.add_row(Row::new(row));
    }

    table.printstd();
    Ok(())
}

fn main() {
    run().unwrap_or_else(|error: CliError| {
        let message = match error {
            CliError::GitError(error) => error.message().to_string(),
        };
        println!("Error: {}", message);
    });
}
