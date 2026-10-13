//! Where a role's writes go, and what stops them.
//!
//! Three rules sit on the write path, each from a measured failure:
//!
//! 1. **A file belongs to the first class whose globs claim it** (ADR-0035), so
//!    a test beside its source is a test and not source.
//! 2. **A role writes only the classes it owns** (ADR-0052), enforced where the
//!    write happens rather than at the tool that asked for it — a doer in the
//!    measurement wrote its source file six times by shell heredoc and skipped
//!    every checkpoint, because the checkpoint was attached to the tool.
//! 3. **A write that would reduce a protected block count is refused**
//!    (ADR-0057). Rule 2 has no granularity inside a file, and Rust keeps unit
//!    tests inside the file under test, so a coder deleting all twelve tests in
//!    its slice is using the one tool it legitimately owns.
//!
//! Whatever survives those is checkpointed before it lands (ADR-0051), so a
//! state that scored better can be restored when the next one scores worse.

pub mod classes;

pub use classes::{Class, Classes};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Which classes a role may write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ownership {
    pub role: String,
    pub owns: Vec<String>,
}

impl Ownership {
    pub fn new(role: impl Into<String>, owns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            role: role.into(),
            owns: owns.into_iter().map(Into::into).collect(),
        }
    }
}

/// A literal whose occurrences may not fall in a write (ADR-0057 §3).
///
/// A literal rather than a pattern on purpose: `#[test]`, `func Test`, `def
/// test_` and `it(` are all literals, and a count of them is enough to catch a
/// suite being removed. It is a floor on quantity, not a check on meaning — a
/// test weakened in place keeps the count, which
/// [ADR-0059] covers instead.
///
/// [ADR-0059]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0059-a-test-is-unvalidated-until-green.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protected {
    pub marker: String,
    /// Classes the marker is counted in. A marker counted everywhere would
    /// refuse a legitimate edit to a file that merely mentions it.
    pub in_classes: Vec<String>,
}

impl Protected {
    pub fn new(
        marker: impl Into<String>,
        in_classes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            marker: marker.into(),
            in_classes: in_classes.into_iter().map(Into::into).collect(),
        }
    }

    fn count(&self, text: &str) -> usize {
        text.matches(&self.marker).count()
    }
}

/// What a role may write, and what refuses it.
#[derive(Debug, Clone)]
pub struct Guard {
    pub classes: Classes,
    pub ownership: Ownership,
    pub protected: Option<Protected>,
}

/// Why a write did not happen. ADR-0052: the agent is told the path and the
/// class, because a denial it cannot tell from a bug is a denial it will retry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error(
        "{path} is {}, which {role} does not own. {role} writes: {}.",
        .class.as_deref().map(|c| format!("class `{c}`")).unwrap_or_else(|| "in no class".into()),
        if .owns.is_empty() { "nothing".to_string() } else { .owns.join(", ") }
    )]
    Unowned {
        path: String,
        class: Option<String>,
        role: String,
        owns: Vec<String>,
    },

    #[error(
        "this write drops `{marker}` from {before} to {after} in {path}. Those are not \
         yours to remove. Re-send the whole file with all {before} kept."
    )]
    DropsProtected {
        path: String,
        marker: String,
        before: usize,
        after: usize,
    },
}

impl Guard {
    /// Decide a write without performing it.
    pub fn permits(
        &self,
        path: &str,
        current: Option<&str>,
        incoming: &str,
    ) -> Result<(), Refusal> {
        let class = self.classes.classify(path);
        let Some(class) = class else {
            return Err(Refusal::Unowned {
                path: path.into(),
                class: None,
                role: self.ownership.role.clone(),
                owns: self.ownership.owns.clone(),
            });
        };
        if !self.ownership.owns.iter().any(|o| o == class) {
            return Err(Refusal::Unowned {
                path: path.into(),
                class: Some(class.into()),
                role: self.ownership.role.clone(),
                owns: self.ownership.owns.clone(),
            });
        }

        if let (Some(protected), Some(current)) = (&self.protected, current)
            && protected.in_classes.iter().any(|c| c == class)
        {
            let (before, after) = (protected.count(current), protected.count(incoming));
            if after < before {
                return Err(Refusal::DropsProtected {
                    path: path.into(),
                    marker: protected.marker.clone(),
                    before,
                    after,
                });
            }
        }
        Ok(())
    }
}

/// How good a verified state is (ADR-0051 §2).
///
/// Derived by the caller from a check's verdict rather than defined here, so
/// the write path does not need to know how checks are judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    /// Ordered first: any green state beats any state that is not.
    pub green: bool,
    pub cases_passing: u32,
}

/// A saved state of one file.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: usize,
    pub path: String,
    pub stored: PathBuf,
    /// Attached when a check runs while this state is on disk.
    pub score: Option<Score>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Refused(#[from] Refusal),

    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{0} escapes the workspace root")]
    Escapes(String),

    #[error("no checkpoint {0}")]
    NoSuchCheckpoint(usize),
}

/// What a restore cost, so it can be reported rather than silently applied
/// (ADR-0051 §4). An agent that finds its edit undone with no explanation will
/// simply make it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    pub path: String,
    pub from: usize,
    pub regained: Option<Score>,
    pub lost: Option<Score>,
}

pub struct Workspace {
    root: PathBuf,
    store: PathBuf,
    checkpoints: Vec<Checkpoint>,
    /// The score of what is on disk now, held until the next write turns that
    /// content into a checkpoint. A checkpoint is taken *before* a write, so
    /// the state being scored is not a checkpoint yet when the check runs.
    pending: BTreeMap<String, Score>,
}

impl Workspace {
    pub fn open(root: impl Into<PathBuf>, store: impl Into<PathBuf>) -> Result<Self, Error> {
        let store = store.into();
        std::fs::create_dir_all(&store).map_err(|source| Error::Io {
            path: store.clone(),
            source,
        })?;
        Ok(Self {
            root: root.into(),
            store,
            checkpoints: Vec::new(),
            pending: BTreeMap::new(),
        })
    }

    pub fn read(&self, path: &str) -> Result<Option<String>, Error> {
        let full = self.resolve(path)?;
        match std::fs::read_to_string(&full) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(Error::Io { path: full, source }),
        }
    }

    /// Checkpoint, then write. A refused write leaves both the file and the
    /// checkpoint store untouched.
    pub fn write(&mut self, path: &str, content: &str, guard: &Guard) -> Result<(), Error> {
        let full = self.resolve(path)?;
        let current = self.read(path)?;
        guard.permits(path, current.as_deref(), content)?;

        if current.is_some() {
            self.checkpoint(path, &full)?;
        }
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(&full, content).map_err(|source| Error::Io { path: full, source })
    }

    fn checkpoint(&mut self, path: &str, full: &Path) -> Result<usize, Error> {
        let id = self.checkpoints.len();
        let stored = self.store.join(format!("{id:05}-{}", flatten(path)));
        std::fs::copy(full, &stored).map_err(|source| Error::Io {
            path: stored.clone(),
            source,
        })?;
        self.checkpoints.push(Checkpoint {
            id,
            path: path.to_string(),
            stored,
            score: self.pending.remove(path),
        });
        Ok(id)
    }

    /// Record what a check said about the state currently on disk for `path`.
    /// It becomes that content's score when the next write checkpoints it.
    pub fn score(&mut self, path: &str, score: Score) {
        self.pending.insert(path.to_string(), score);
    }

    /// The best scored state saved for `path`.
    pub fn best(&self, path: &str) -> Option<&Checkpoint> {
        self.checkpoints
            .iter()
            .filter(|c| c.path == path && c.score.is_some())
            .max_by_key(|c| c.score)
    }

    /// ADR-0051 §3: a state that scores worse than one already saved is a
    /// regression, and the better state is what to go back to. Where no check
    /// has run there is nothing to compare and the write stands.
    pub fn regression(&self, path: &str, now: Score) -> Option<&Checkpoint> {
        self.best(path)
            .filter(|best| best.score.is_some_and(|saved| saved > now))
    }

    /// Put a checkpoint back on disk. The current content is checkpointed
    /// first, so a restore is never itself a loss.
    pub fn restore(&mut self, id: usize) -> Result<Restored, Error> {
        let checkpoint = self
            .checkpoints
            .get(id)
            .ok_or(Error::NoSuchCheckpoint(id))?
            .clone();
        let full = self.resolve(&checkpoint.path)?;
        let lost = self.pending.get(&checkpoint.path).copied();

        if full.exists() {
            self.checkpoint(&checkpoint.path, &full)?;
        }
        std::fs::copy(&checkpoint.stored, &full)
            .map_err(|source| Error::Io { path: full, source })?;
        // What is on disk is now the restored content, so its score is known.
        if let Some(score) = checkpoint.score {
            self.pending.insert(checkpoint.path.clone(), score);
        }

        Ok(Restored {
            path: checkpoint.path,
            from: id,
            regained: checkpoint.score,
            lost,
        })
    }

    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    fn resolve(&self, path: &str) -> Result<PathBuf, Error> {
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(Error::Escapes(path.to_string()));
        }
        Ok(self.root.join(relative))
    }
}

fn flatten(path: &str) -> String {
    path.replace(['/', '\\'], "_")
}
