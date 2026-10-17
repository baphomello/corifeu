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

/// One file inside a checkpoint: where it lives in the workspace, and where
/// its content was copied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub path: String,
    pub stored: PathBuf,
}

/// A saved state of everything a slice is about, and what each check thought
/// of it.
///
/// The unit is the slice rather than the file, because a case count is a
/// property of the combination. A component and its test that change together
/// have states that are only meaningful in pairs: restoring the test to a
/// state that scored nine, beside a component it never saw, produces a
/// combination nothing ever measured (ADR-0051, scoped per slice).
///
/// Scores are kept per check rather than one per state, because a count only
/// means something next to the check that produced it. Nine cases passing under
/// one slice's oracle says nothing about five passing under another's, and
/// comparing them reads every smaller check as a regression.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: usize,
    pub files: Vec<Saved>,
    /// Keyed by the check that produced each score.
    pub scores: BTreeMap<String, Score>,
}

impl Checkpoint {
    pub fn score(&self, scope: &str) -> Option<Score> {
        self.scores.get(scope).copied()
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(|f| f.path.as_str())
    }

    pub fn holds(&self, path: &str) -> bool {
        self.files.iter().any(|f| f.path == path)
    }
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
    pub paths: Vec<String>,
    pub from: usize,
    /// What each check thought of the state that came back.
    pub regained: BTreeMap<String, Score>,
    /// What each check thought of the state that was replaced.
    pub lost: BTreeMap<String, Score>,
}

impl Restored {
    pub fn regained(&self, scope: &str) -> Option<Score> {
        self.regained.get(scope).copied()
    }

    pub fn lost(&self, scope: &str) -> Option<Score> {
        self.lost.get(scope).copied()
    }

    /// The files, as a doer should be told about them.
    pub fn describe(&self) -> String {
        self.paths.join(", ")
    }
}

pub struct Workspace {
    root: PathBuf,
    store: PathBuf,
    checkpoints: Vec<Checkpoint>,
    /// The files the current slice is about, saved and restored together.
    /// Empty until a caller stages one, and a write outside it is checkpointed
    /// on its own — which is what a role writing a file the slice does not
    /// name is doing.
    slice: Vec<String>,
    /// Scores of what is on disk now, keyed by the check that produced each,
    /// held until the next write turns that content into a checkpoint. A
    /// checkpoint is taken *before* a write, so the state being scored is not
    /// a checkpoint yet when the check runs.
    ///
    /// Not keyed by path: the state being scored is the slice's, and the check
    /// that produced the score already identifies which slice that is.
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
            slice: Vec::new(),
            pending: BTreeMap::new(),
        })
    }

    /// The directory writes resolve against, which is also where a doer's
    /// commands run.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Declare the files the next slice is about. A write to any of them
    /// checkpoints all of them, so a rollback restores the combination that
    /// was scored rather than one file out of it.
    ///
    /// Scores recorded for the previous slice are dropped, because they
    /// describe content this slice is about to replace.
    pub fn stage(&mut self, paths: &[String]) {
        self.slice = paths.to_vec();
        self.pending.clear();
    }

    /// The files that travel with `path`: the staged set when it is one of
    /// them, and `path` alone otherwise.
    fn group(&self, path: &str) -> Vec<String> {
        if self.slice.iter().any(|p| p == path) {
            self.slice.clone()
        } else {
            vec![path.to_string()]
        }
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

        // Every file the slice holds, not only the one being written: the
        // score this checkpoint carries was measured on the combination.
        self.checkpoint(&self.group(path))?;
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(&full, content).map_err(|source| Error::Io { path: full, source })
    }

    /// Save every named file that exists. A set where none of them exists yet
    /// is not a state worth keeping, so nothing is pushed and the ids stay
    /// dense.
    fn checkpoint(&mut self, paths: &[String]) -> Result<Option<usize>, Error> {
        let id = self.checkpoints.len();
        let mut files = Vec::new();
        for (n, path) in paths.iter().enumerate() {
            let full = self.resolve(path)?;
            if !full.exists() {
                continue;
            }
            let stored = self.store.join(format!("{id:05}-{n}-{}", flatten(path)));
            std::fs::copy(&full, &stored).map_err(|source| Error::Io {
                path: stored.clone(),
                source,
            })?;
            files.push(Saved {
                path: path.clone(),
                stored,
            });
        }
        if files.is_empty() {
            return Ok(None);
        }
        self.checkpoints.push(Checkpoint {
            id,
            files,
            scores: std::mem::take(&mut self.pending),
        });
        Ok(Some(id))
    }

    /// Record what a check said about the state currently on disk. It becomes
    /// that state's score when the next write checkpoints it.
    pub fn score(&mut self, scope: &str, score: Score) {
        self.pending.insert(scope.to_string(), score);
    }

    /// The best state this check has scored.
    ///
    /// Checkpoints from other slices are filtered out by the scope itself: a
    /// slice's oracle is what names it, so a state it never scored is not a
    /// state it can be compared against.
    pub fn best(&self, scope: &str) -> Option<&Checkpoint> {
        self.checkpoints
            .iter()
            .filter(|c| c.score(scope).is_some())
            .max_by_key(|c| c.score(scope))
    }

    /// ADR-0051 §3: a state that scores worse than one already saved is a
    /// regression, and the better state is what to go back to. Where no check
    /// has run there is nothing to compare and the write stands.
    pub fn regression(&self, scope: &str, now: Score) -> Option<&Checkpoint> {
        self.best(scope)
            .filter(|best| best.score(scope).is_some_and(|saved| saved > now))
    }

    /// Put a checkpoint back on disk. The current content is checkpointed
    /// first, so a restore is never itself a loss.
    pub fn restore(&mut self, id: usize) -> Result<Restored, Error> {
        let checkpoint = self
            .checkpoints
            .get(id)
            .ok_or(Error::NoSuchCheckpoint(id))?
            .clone();
        let lost = std::mem::take(&mut self.pending);

        // The content being replaced is checkpointed first, so a restore is
        // never itself a loss. Its own files are the group, not the staged
        // set, because that is the combination that scored.
        let paths: Vec<String> = checkpoint.paths().map(String::from).collect();
        self.checkpoint(&paths)?;

        for file in &checkpoint.files {
            let full = self.resolve(&file.path)?;
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).map_err(|source| Error::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            std::fs::copy(&file.stored, &full)
                .map_err(|source| Error::Io { path: full, source })?;
        }
        // What is on disk is now the restored state, so its scores are known.
        self.pending = checkpoint.scores.clone();

        Ok(Restored {
            paths,
            from: id,
            regained: checkpoint.scores,
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
