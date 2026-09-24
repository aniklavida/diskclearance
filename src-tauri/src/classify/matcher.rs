use std::path::{Path, PathBuf};

use crate::classify::evidence::{Confidence, Recoverability};
use crate::platform::{EntryType, FileIdentity};

/// Context provided to rule matchers during classification.
#[derive(Debug, Clone)]
pub struct MatchContext<'a> {
    pub original_path: &'a Path,
    pub canonical_path: &'a Path,
    pub normalized_path: &'a Path,
    pub entry_type: EntryType,
    pub identity: FileIdentity,
    pub apparent_size: u64,
    pub allocated_size: u64,
    pub modified_ms: Option<u64>,
    pub is_symlink: bool,
    pub symlink_target_canonical: Option<PathBuf>,
    pub home_dir: Option<&'a Path>,
    pub app_support_dir: Option<&'a Path>,
    pub caches_dir: Option<&'a Path>,
    pub crosses_mount_boundary: bool,
    pub inside_git_repo: bool,
    pub is_git_internal: bool,
}

/// Result of evaluating a rule matcher against a filesystem item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub matched: bool,
    pub explanation: String,
    pub confidence: Confidence,
    pub regenerator: Option<String>,
    pub recoverability: Recoverability,
}

impl MatchResult {
    pub fn matched(
        explanation: impl Into<String>,
        confidence: Confidence,
        regenerator: Option<String>,
        recoverability: Recoverability,
    ) -> Self {
        Self {
            matched: true,
            explanation: explanation.into(),
            confidence,
            regenerator,
            recoverability,
        }
    }

    pub fn no_match() -> Self {
        Self {
            matched: false,
            explanation: String::new(),
            confidence: Confidence::Definite,
            regenerator: None,
            recoverability: Recoverability::Irrecoverable,
        }
    }
}

/// Declarative matcher for filesystem rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleMatcher {
    /// Exact canonical path match.
    ExactPath(PathBuf),
    /// Canonical path starts with this prefix (including all subpaths).
    PrefixPath(PathBuf),
    /// Path relative to user home directory.
    HomeRelativePrefix(PathBuf),
    /// Path relative to user Caches directory.
    CachesRelativePrefix(PathBuf),
    /// Path relative to user Application Support directory.
    AppSupportRelativePrefix(PathBuf),
    /// Matches any file or directory inside a .git directory.
    GitInternal,
    /// Matches user working tree items inside a git repository (excluding .git).
    GitWorkingTreeUserFile,
    /// Matches entries where traversal crosses a filesystem mount boundary.
    MountBoundaryCrossed,
    /// Matches a symlink pointing into a protected location or outside allowed boundaries.
    SymlinkPointingToProtected,
    /// Matches durable local AI model storage directories.
    DurableModelStorage,
    /// Matches conventional project build output directories.
    BuildOutput,
    /// Composite matcher requiring all inner matchers to match.
    All(Vec<RuleMatcher>),
    /// Composite matcher requiring at least one inner matcher to match.
    Any(Vec<RuleMatcher>),
}

impl RuleMatcher {
    /// Evaluates whether this matcher matches the given item context.
    pub fn evaluate(&self, ctx: &MatchContext<'_>) -> MatchResult {
        match self {
            Self::ExactPath(expected) => {
                let matches = ctx.canonical_path == expected || ctx.normalized_path == expected;
                if matches {
                    MatchResult::matched(
                        format!("Exact canonical path matches {}", expected.display()),
                        Confidence::Definite,
                        None,
                        Recoverability::Irrecoverable,
                    )
                } else {
                    MatchResult::no_match()
                }
            }
            Self::PrefixPath(prefix) => {
                let matches = ctx.canonical_path.starts_with(prefix)
                    || ctx.normalized_path.starts_with(prefix);
                if matches {
                    MatchResult::matched(
                        format!("Canonical path sits inside {}", prefix.display()),
                        Confidence::Definite,
                        None,
                        Recoverability::Irrecoverable,
                    )
                } else {
                    MatchResult::no_match()
                }
            }
            Self::HomeRelativePrefix(rel) => {
                if let Some(home) = ctx.home_dir {
                    let full = home.join(rel);
                    let matches = ctx.canonical_path.starts_with(&full)
                        || ctx.normalized_path.starts_with(&full);
                    if matches {
                        return MatchResult::matched(
                            format!("Path sits inside user directory ~/{}", rel.display()),
                            Confidence::Definite,
                            None,
                            Recoverability::Irrecoverable,
                        );
                    }
                }
                MatchResult::no_match()
            }
            Self::CachesRelativePrefix(rel) => {
                if let Some(caches) = ctx.caches_dir {
                    let full = caches.join(rel);
                    let matches = ctx.canonical_path.starts_with(&full)
                        || ctx.normalized_path.starts_with(&full);
                    if matches {
                        return MatchResult::matched(
                            format!("Path sits inside cache directory {}", rel.display()),
                            Confidence::Definite,
                            None,
                            Recoverability::TrashRecoverable,
                        );
                    }
                }
                MatchResult::no_match()
            }
            Self::AppSupportRelativePrefix(rel) => {
                if let Some(app_sup) = ctx.app_support_dir {
                    let full = app_sup.join(rel);
                    let matches = ctx.canonical_path.starts_with(&full)
                        || ctx.normalized_path.starts_with(&full);
                    if matches {
                        return MatchResult::matched(
                            format!(
                                "Path sits inside Application Support directory {}",
                                rel.display()
                            ),
                            Confidence::Definite,
                            None,
                            Recoverability::TrashRecoverable,
                        );
                    }
                }
                MatchResult::no_match()
            }
            Self::GitInternal => {
                if ctx.is_git_internal {
                    MatchResult::matched(
                        "Internal Git repository state (.git directory)",
                        Confidence::Definite,
                        None,
                        Recoverability::Irrecoverable,
                    )
                } else {
                    MatchResult::no_match()
                }
            }
            Self::GitWorkingTreeUserFile => {
                if ctx.inside_git_repo && !ctx.is_git_internal {
                    MatchResult::matched(
                        "User working tree file inside source-controlled git repository",
                        Confidence::Definite,
                        None,
                        Recoverability::SourceControlled { remote: None },
                    )
                } else {
                    MatchResult::no_match()
                }
            }
            Self::MountBoundaryCrossed => {
                if ctx.crosses_mount_boundary {
                    MatchResult::matched(
                        "Filesystem mount boundary crossed during traversal",
                        Confidence::Definite,
                        None,
                        Recoverability::Irrecoverable,
                    )
                } else {
                    MatchResult::no_match()
                }
            }
            Self::SymlinkPointingToProtected => {
                if ctx.is_symlink {
                    if let Some(target) = &ctx.symlink_target_canonical {
                        MatchResult::matched(
                            format!(
                                "Symlink points to external or protected path: {}",
                                target.display()
                            ),
                            Confidence::Definite,
                            None,
                            Recoverability::Irrecoverable,
                        )
                    } else {
                        MatchResult::no_match()
                    }
                } else {
                    MatchResult::no_match()
                }
            }
            Self::BuildOutput => {
                let matches = ctx.entry_type == EntryType::Directory
                    && ctx
                        .canonical_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            ["target", "build", "dist", "out", ".next"].contains(&name)
                        });
                if matches {
                    MatchResult::matched(
                        "Conventional project build output directory",
                        Confidence::Definite,
                        None,
                        Recoverability::RebuildableByTool {
                            command: "project build command".to_string(),
                        },
                    )
                } else {
                    MatchResult::no_match()
                }
            }
            Self::DurableModelStorage => {
                let matches_home_model = if let Some(home) = ctx.home_dir {
                    let ollama_models = home.join(".ollama/models");
                    let user_models = home.join("models");
                    let huggingface_hub = home.join(".cache/huggingface/hub");
                    ctx.canonical_path.starts_with(&ollama_models)
                        || ctx.normalized_path.starts_with(&ollama_models)
                        || ctx.canonical_path.starts_with(&user_models)
                        || ctx.normalized_path.starts_with(&user_models)
                        || ctx.canonical_path.starts_with(&huggingface_hub)
                        || ctx.normalized_path.starts_with(&huggingface_hub)
                } else {
                    false
                };

                if matches_home_model {
                    MatchResult::matched(
                        "Durable machine learning model weights or local checkpoint store",
                        Confidence::Definite,
                        None,
                        Recoverability::Irrecoverable,
                    )
                } else {
                    MatchResult::no_match()
                }
            }
            Self::All(matchers) => {
                for m in matchers {
                    let res = m.evaluate(ctx);
                    if !res.matched {
                        return MatchResult::no_match();
                    }
                }
                MatchResult::matched(
                    "All rule conditions satisfied",
                    Confidence::Definite,
                    None,
                    Recoverability::TrashRecoverable,
                )
            }
            Self::Any(matchers) => {
                for m in matchers {
                    let res = m.evaluate(ctx);
                    if res.matched {
                        return res;
                    }
                }
                MatchResult::no_match()
            }
        }
    }
}
