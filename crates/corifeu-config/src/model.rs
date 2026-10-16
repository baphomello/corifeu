//! The configuration model.
//!
//! Mirrors `docs/config.md` section for section. Every field that can sensibly
//! default does, so the minimum valid configuration stays small; what cannot
//! default is required and reported by [`crate::validate`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub project: Project,
    #[serde(default)]
    pub providers: BTreeMap<String, Provider>,
    #[serde(default)]
    pub privacy: Privacy,
    pub roles: BTreeMap<String, Role>,
    #[serde(default = "FileClass::defaults")]
    pub file_classes: Vec<FileClass>,
    #[serde(default)]
    pub ownership: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub oracles: BTreeMap<String, Oracle>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub autonomy: Autonomy,
    #[serde(default)]
    pub visibility: Visibility,
    #[serde(default)]
    pub git: Git,
    #[serde(default)]
    pub fallback: BTreeMap<String, Fallback>,
    #[serde(default)]
    pub execution: Execution,
    #[serde(default)]
    pub retention: Retention,
    #[serde(default)]
    pub memory: Memory,
    #[serde(default)]
    pub metering: Metering,
    #[serde(default)]
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub domain: Domain,
}

/// MVP 1 targets software development only (ADR-0006). The enum exists so that
/// widening the domain later is an added variant rather than a core change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Domain {
    #[default]
    Software,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    pub kind: ProviderKind,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub min_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    /// An agent CLI driven as a subprocess (ADR-0007).
    Cli,
    Ollama,
    LlamaCpp,
    /// Supplied by an out-of-process plugin (ADR-0032).
    Plugin,
}

impl ProviderKind {
    /// Whether work sent to this provider leaves the machine. Consumed by
    /// [`Privacy::allow_remote_providers`] (ADR-0041).
    pub fn is_local(self) -> bool {
        matches!(self, Self::Ollama | Self::LlamaCpp)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Privacy {
    /// `false` refuses any non-local provider in any role. This replaces the
    /// prototype's undefined `private` mode with a checkable property (ADR-0041).
    #[serde(default = "yes")]
    pub allow_remote_providers: bool,
}

impl Default for Privacy {
    fn default() -> Self {
        Self {
            allow_remote_providers: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub category: Category,
    pub provider: String,
    pub model: String,
    /// Required roles make a configuration invalid by their absence (ADR-0012).
    #[serde(default)]
    pub required: bool,
    /// File classes this role may write, not paths (ADR-0035).
    #[serde(default)]
    pub writes: Vec<String>,
    #[serde(default)]
    pub reads_only: bool,
    #[serde(default)]
    pub contract: Contract,
}

/// The five categories the core understands. Roles are configuration;
/// categories are not (ADR-0010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Lead,
    Planners,
    Researchers,
    Doers,
    Checkers,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::Planners => "planners",
            Self::Researchers => "researchers",
            Self::Doers => "doers",
            Self::Checkers => "checkers",
        }
    }
}

/// What a role's assembled view contains. Enforced by assembly, never by
/// instruction (ADR-0014).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    #[serde(default = "yes")]
    pub task: bool,
    #[serde(default = "yes")]
    pub acceptance: bool,
    #[serde(default)]
    pub files: Vec<serde_json::Value>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub messages: Option<ContractMessages>,
    #[serde(default)]
    pub memory: Vec<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            task: true,
            acceptance: true,
            files: Vec::new(),
            artifacts: Vec::new(),
            messages: None,
            memory: Vec::new(),
            max_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContractMessages {
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub last: Option<u32>,
}

/// Ordered glob rules. First match wins, which is what lets a sibling test file
/// classify as a test rather than as source (ADR-0035).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileClass {
    pub name: String,
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl FileClass {
    fn of(name: &str, include: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            include: include.iter().map(|s| (*s).to_string()).collect(),
            exclude: Vec::new(),
        }
    }

    /// The default classes, covering the common layouts across ecosystems.
    ///
    /// **`test` is declared first on purpose.** First match wins, so
    /// `src/bank/deposit_test.ts` classifies as a test while its sibling
    /// `deposit.ts` falls through to `source` — the case a directory prefix
    /// cannot express, and the reason this model exists (ADR-0035).
    pub fn defaults() -> Vec<Self> {
        vec![
            Self::of(
                "test",
                &[
                    "**/*_test.*",
                    "**/*.test.*",
                    "**/*_spec.*",
                    "**/*.spec.*",
                    "tests/**",
                    "spec/**",
                ],
            ),
            Self::of(
                "source",
                &[
                    "src/**",
                    "lib/**",
                    "app/**",
                    "internal/**",
                    "pkg/**",
                    "cmd/**",
                ],
            ),
            Self::of(
                "config",
                &[
                    "*.json",
                    "*.toml",
                    "*.yaml",
                    "*.yml",
                    ".github/**",
                    "Dockerfile*",
                ],
            ),
            Self::of("docs", &["**/*.md", "docs/**"]),
        ]
    }
}

/// A declared pair permitted to exchange direct messages (ADR-0015).
pub type Channel = [String; 2];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Oracle {
    /// An argument vector, never a shell string (ADR-0037).
    pub command: Vec<String>,
    #[serde(default = "default_oracle_timeout")]
    pub timeout_s: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default = "default_rounds")]
    pub rounds_per_pair: u32,
    #[serde(default)]
    pub steps: Steps,
    #[serde(default)]
    pub task_budget_tokens: Option<u64>,
    #[serde(default)]
    pub run_budget_tokens: Option<u64>,
    #[serde(default = "default_role_timeout")]
    pub role_timeout_s: u32,
    #[serde(default = "yes")]
    pub loop_detection: bool,
    /// Consecutive repeats of one action before a role's turn stops
    /// ([ADR-0050]). One stops at the first repeat.
    ///
    /// Six is what three measurement rounds ran with. The fifth round priced
    /// it: across three cells and two languages 54 slices repeated an action,
    /// 2 reached green, and the steps spent after a first repeat were 30% of
    /// everything the round spent. The right number is a measurement and this
    /// is where it is taken.
    ///
    /// [ADR-0050]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0050-loop-detection-warns-before-it-stops.md
    #[serde(default = "default_repeats")]
    pub repeats: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            rounds_per_pair: default_rounds(),
            steps: Steps::default(),
            task_budget_tokens: None,
            run_budget_tokens: None,
            role_timeout_s: default_role_timeout(),
            loop_detection: true,
            repeats: default_repeats(),
        }
    }
}

/// How many steps one role gets on one slice, and what makes that grow
/// (ADR-0062).
///
/// A flat ceiling is either too high for work that finishes or a guess about
/// work nobody has measured. `floor` is what a turn starts with; it grows by
/// `raise_by`, never past `ceiling`, and only after `patience` turns have spent
/// every step they had. Anything that reaches its goal puts it back at the
/// floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    #[serde(default = "default_steps_floor")]
    pub floor: u32,
    #[serde(default = "default_steps_ceiling")]
    pub ceiling: u32,
    #[serde(default = "default_steps_patience")]
    pub patience: u32,
    #[serde(default = "default_steps_raise_by")]
    pub raise_by: u32,
}

impl Default for Steps {
    fn default() -> Self {
        Self {
            floor: default_steps_floor(),
            ceiling: default_steps_ceiling(),
            patience: default_steps_patience(),
            raise_by: default_steps_raise_by(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Autonomy {
    #[default]
    Supervised,
    SemiAutonomous,
    Autonomous,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Visibility {
    #[serde(default)]
    pub peer_conversations: PeerVisibility,
    #[serde(default = "yes")]
    pub show_token_usage: bool,
}

impl Default for Visibility {
    fn default() -> Self {
        Self {
            peer_conversations: PeerVisibility::default(),
            show_token_usage: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PeerVisibility {
    #[default]
    Summary,
    Live,
    Hidden,
}

/// Every git operation is decided separately, and each defaults to its most
/// conservative option (ADR-0025).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Git {
    #[serde(default)]
    pub branch: GitBranch,
    #[serde(default)]
    pub commit: GitCommit,
    #[serde(default)]
    pub push: GitPush,
    #[serde(default)]
    pub pull_request: GitPullRequest,
    #[serde(default)]
    pub merge: GitGate,
    #[serde(default)]
    pub delete_branch: GitGate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitBranch {
    #[serde(default)]
    pub strategy: BranchStrategy,
    #[serde(default = "default_branch_pattern")]
    pub pattern: String,
}

impl Default for GitBranch {
    fn default() -> Self {
        Self {
            strategy: BranchStrategy::default(),
            pattern: default_branch_pattern(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BranchStrategy {
    #[default]
    PerRun,
    PerTask,
    Current,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitCommit {
    #[serde(default)]
    pub cadence: CommitCadence,
    #[serde(default)]
    pub author: CommitAuthor,
    #[serde(default)]
    pub convention: Option<String>,
    #[serde(default)]
    pub require_approval: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommitCadence {
    #[default]
    PerTask,
    PerPhase,
    End,
    Never,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommitAuthor {
    /// The configured git identity. Corifeu never adds a tool attribution.
    #[default]
    User,
}

/// Gates default to `never`, so clicking through the wizard without reading
/// cannot produce a configuration that surprises you.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitGate {
    #[serde(default)]
    pub when: GateWhen,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateWhen {
    #[default]
    Never,
    AfterApproval,
    Automatic,
    AfterMerge,
}

/// Push is the gate that matters most: it is where work leaves the machine.
/// It carries its own option because the owner's flow presents every commit,
/// with its rationale, *before* that gate opens (ADR-0025).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPush {
    #[serde(default)]
    pub when: GateWhen,
    /// Show every commit and why it exists before pushing.
    #[serde(default)]
    pub present_commits: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPullRequest {
    #[serde(default)]
    pub when: GateWhen,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fallback {
    pub strategy: FallbackStrategy,
    #[serde(default)]
    pub attempts: Option<u32>,
    #[serde(default)]
    pub backoff: Option<String>,
    #[serde(default)]
    pub max_s: Option<u32>,
    #[serde(default)]
    pub escalate_to: Option<String>,
    #[serde(default)]
    pub budget_cap: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FallbackStrategy {
    Stop,
    Retry,
    Escalate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    #[serde(default)]
    pub sandbox: Sandbox,
    /// Commands run as argument vectors. `true` is an explicit, documented
    /// choice to reintroduce shell interpretation (ADR-0037).
    #[serde(default)]
    pub shell: bool,
    #[serde(default)]
    pub allowed_commands: Vec<Vec<String>>,
}

impl Default for Execution {
    fn default() -> Self {
        Self {
            sandbox: Sandbox::Auto,
            shell: false,
            allowed_commands: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sandbox {
    #[default]
    Auto,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Retention {
    #[serde(default = "default_full_runs")]
    pub full_runs: u32,
    /// The limit that protects against the tail; run sizes vary by an order of
    /// magnitude and a count alone controls only the mean (ADR-0022).
    #[serde(default = "default_max_bytes")]
    pub max_bytes: String,
    #[serde(default = "default_always_keep")]
    pub always_keep: u32,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            full_runs: default_full_runs(),
            max_bytes: default_max_bytes(),
            always_keep: default_always_keep(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Memory {
    #[serde(default = "default_memory_dir")]
    pub dir: String,
    #[serde(default)]
    pub documents: Vec<String>,
    #[serde(default)]
    pub agent_writes: MemoryWrites,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            dir: default_memory_dir(),
            documents: Vec::new(),
            agent_writes: MemoryWrites::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryWrites {
    #[default]
    Propose,
    Direct,
    Never,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metering {
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Billing mode per provider. Decides whether savings are reported in money
    /// or as a share of quota (ADR-0031).
    #[serde(default)]
    pub billing: BTreeMap<String, Billing>,
    #[serde(default)]
    pub report: MeteringReport,
}

impl Default for Metering {
    fn default() -> Self {
        Self {
            enabled: true,
            billing: BTreeMap::new(),
            report: MeteringReport::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Billing {
    Api,
    Subscription,
    Free,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeteringReport {
    #[default]
    EndOfRun,
    PerTask,
    Never,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub config: serde_json::Value,
}

fn yes() -> bool {
    true
}
/// Twelve: the slowest of the fourth run's 57 green slices. Thirty-eight of
/// them closed in one step.
/// ADR-0050's measured default. Three killed the winning Task B round at step
/// 26: it hit four dead ends in a row before finding the fix, which is
/// searching rather than looping.
fn default_repeats() -> u32 {
    6
}

fn default_steps_floor() -> u32 {
    12
}

/// Thirty: what the fourth run ran with, kept as the place growth stops.
fn default_steps_ceiling() -> u32 {
    30
}

fn default_steps_patience() -> u32 {
    2
}

fn default_steps_raise_by() -> u32 {
    6
}

fn default_rounds() -> u32 {
    3
}
fn default_role_timeout() -> u32 {
    900
}
fn default_oracle_timeout() -> u32 {
    300
}
fn default_full_runs() -> u32 {
    10
}
fn default_always_keep() -> u32 {
    3
}
fn default_max_bytes() -> String {
    "500MB".to_string()
}
fn default_memory_dir() -> String {
    ".corifeu/memory".to_string()
}
fn default_branch_pattern() -> String {
    "corifeu/{run-slug}".to_string()
}
