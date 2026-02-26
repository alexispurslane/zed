//! Agent Skills discovery and management.
//!
//! Skills are located in `~/.config/zed/skills/<skill-name>/` with the following structure:
//! - `SKILL.md` - Main instructions (YAML frontmatter + Markdown body)
//! - `scripts/` - Executable scripts
//! - `references/` - Additional documentation
//! - `assets/` - Templates, data files, images

use crate::{SkillContext, SkillsPromptTemplate, Template, Templates};
use anyhow::{Result, anyhow};
use collections::HashMap;
use futures::StreamExt;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The name of the main skill file.
pub const SKILL_FILE_NAME: &str = "SKILL.md";

/// Metadata extracted from a skill's YAML frontmatter.
#[derive(Clone, Debug, Deserialize)]
pub struct SkillMetadata {
    /// Required: lowercase alphanumeric + hyphens, max 64 chars, must match directory
    pub name: String,
    /// Required: max 1024 chars, explains what & when to use
    pub description: String,
    /// Optional: license information
    pub license: Option<String>,
    /// Optional: environment requirements
    pub compatibility: Option<String>,
    /// Optional: key-value map of additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Optional: space-delimited tool list (experimental)
    pub allowed_tools: Option<String>,
}

impl SkillMetadata {
    /// Validates that the skill name matches the expected format.
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(anyhow!("skill name cannot be empty"));
        }
        if self.name.len() > 64 {
            return Err(anyhow!(
                "skill name exceeds 64 characters: {}",
                self.name.len()
            ));
        }
        if !self
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(anyhow!(
                "skill name must be lowercase alphanumeric + hyphens only: {}",
                self.name
            ));
        }
        if self.description.len() > 1024 {
            return Err(anyhow!(
                "skill description exceeds 1024 characters: {}",
                self.description.len()
            ));
        }
        Ok(())
    }
}

/// A discovered skill with its metadata and content.
#[derive(Clone, Debug)]
pub struct Skill {
    /// The skill metadata parsed from frontmatter.
    pub metadata: SkillMetadata,
    /// The markdown body (without frontmatter).
    pub body: String,
    /// The absolute path to the skill directory.
    pub path: PathBuf,
}

impl Skill {
    /// Returns the skill name.
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Returns the skill description.
    pub fn description(&self) -> &str {
        &self.metadata.description
    }

    /// Resolves a file path relative to the skill directory.
    /// Returns an error if the resolved path would escape the skill directory.
    pub fn resolve_path(&self, relative_path: &str) -> Result<PathBuf> {
        // Reject paths with parent directory components to prevent traversal attacks
        if relative_path.contains("..") {
            return Err(anyhow!("path traversal not allowed: {}", relative_path));
        }

        let resolved = self.path.join(relative_path);
        let canonical_resolved = resolved.canonicalize().unwrap_or(resolved);

        // Ensure the resolved path is within the skill directory
        let canonical_skill_dir = self
            .path
            .canonicalize()
            .unwrap_or_else(|_| self.path.clone());
        if !canonical_resolved.starts_with(&canonical_skill_dir) {
            return Err(anyhow!("path escapes skill directory: {}", relative_path));
        }

        Ok(canonical_resolved)
    }
}

/// Parses YAML frontmatter from a markdown file.
/// Returns the parsed metadata and the markdown body.
fn parse_skill_file(content: &str) -> Result<(SkillMetadata, String)> {
    let content = content.trim_start();

    // Check if content starts with frontmatter delimiter
    if !content.starts_with("---") {
        return Err(anyhow!("SKILL.md must start with YAML frontmatter (---)"));
    }

    // Find the end of frontmatter
    let end_marker = content[3..].find("---");
    let (yaml_part, body) = match end_marker {
        Some(end) => {
            let yaml_end = 3 + end;
            let yaml = content[3..yaml_end].trim().to_string();
            let body_start = yaml_end + 3;
            let body = content[body_start..].trim_start().to_string();
            (yaml, body)
        }
        None => return Err(anyhow!("YAML frontmatter not properly closed with ---")),
    };

    let metadata: SkillMetadata = serde_yaml::from_str(&yaml_part)
        .map_err(|e| anyhow!("failed to parse YAML frontmatter: {}", e))?;

    metadata.validate()?;

    Ok((metadata, body))
}

/// Discovers all skills in the given directory (async version).
/// Returns a map of skill name to Skill.
pub async fn discover_skills(
    fs: &dyn fs::Fs,
    skills_dir: &Path,
) -> Result<HashMap<String, Arc<Skill>>> {
    let mut skills = HashMap::default();

    if !fs.is_dir(skills_dir).await {
        // Skills directory doesn't exist yet - that's ok, just return empty
        return Ok(skills);
    }

    let mut entries = fs.read_dir(skills_dir).await?;

    while let Some(entry) = entries.next().await {
        let path = entry?;

        if !fs.is_dir(&path).await {
            continue;
        }

        let skill_file = path.join(SKILL_FILE_NAME);
        if !fs.is_file(&skill_file).await {
            log::debug!("skipping directory without SKILL.md: {:?}", path);
            continue;
        }

        let content = match fs.load(&skill_file).await {
            Ok(content) => content,
            Err(e) => {
                log::warn!("failed to read {:?}: {}", skill_file, e);
                continue;
            }
        };

        let (metadata, body) = match parse_skill_file(&content) {
            Ok(result) => result,
            Err(e) => {
                log::warn!("failed to parse {:?}: {}", skill_file, e);
                continue;
            }
        };

        // Verify the skill name matches the directory name
        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if metadata.name != dir_name {
            log::warn!(
                "skill name '{}' doesn't match directory name '{}', skipping",
                metadata.name,
                dir_name
            );
            continue;
        }

        let skill = Arc::new(Skill {
            metadata,
            body,
            path: path.clone(),
        });

        skills.insert(skill.name().to_string(), skill);
    }

    Ok(skills)
}

/// Synchronous version of skill discovery for use in synchronous contexts.
/// Returns a map of skill name to Skill.
pub fn discover_skills_sync(skills_dir: &Path) -> HashMap<String, Arc<Skill>> {
    let mut skills = HashMap::default();

    if !skills_dir.exists() || !skills_dir.is_dir() {
        return skills;
    }

    let entries = match std::fs::read_dir(skills_dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("failed to read skills directory: {}", e);
            return skills;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let skill_file = path.join(SKILL_FILE_NAME);
        if !skill_file.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&skill_file) {
            Ok(content) => content,
            Err(e) => {
                log::warn!("failed to read {:?}: {}", skill_file, e);
                continue;
            }
        };

        let (metadata, body) = match parse_skill_file(&content) {
            Ok(result) => result,
            Err(e) => {
                log::warn!("failed to parse {:?}: {}", skill_file, e);
                continue;
            }
        };

        // Verify the skill name matches the directory name
        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if metadata.name != dir_name {
            log::warn!(
                "skill name '{}' doesn't match directory name '{}', skipping",
                metadata.name,
                dir_name
            );
            continue;
        }

        let skill = Arc::new(Skill {
            metadata,
            body,
            path: path.clone(),
        });

        skills.insert(skill.name().to_string(), skill);
    }

    skills
}

/// Returns the default global skills directory path (~/.config/zed/skills).
pub fn global_skills_dir() -> PathBuf {
    paths::config_dir().join("skills")
}

/// Returns the worktree-relative skills directory path (.agents/skills).
pub fn worktree_skills_dir(worktree_root: &Path) -> PathBuf {
    worktree_root.join(".agents").join("skills")
}

/// Discovers skills from both global and worktree locations.
/// Worktree skills take precedence over global skills with the same name.
/// Later worktrees in the slice take precedence over earlier ones.
pub fn discover_all_skills_sync(worktree_roots: &[PathBuf]) -> HashMap<String, Arc<Skill>> {
    // Start with global skills
    let mut all_skills = discover_skills_sync(&global_skills_dir());

    // Merge in worktree skills (later worktrees override earlier ones)
    for worktree in worktree_roots {
        let worktree_skills = discover_skills_sync(&worktree_skills_dir(worktree));
        // Worktree skills override global skills and earlier worktree skills
        for (name, skill) in worktree_skills {
            all_skills.insert(name, skill);
        }
    }

    all_skills
}

/// Format skills for display in the system prompt using handlebars templating.
pub fn format_skills_for_prompt(
    skills: &HashMap<String, Arc<Skill>>,
    templates: &Templates,
) -> String {
    // Sort skills by name for consistent ordering
    let mut skill_list: Vec<_> = skills.values().collect();
    skill_list.sort_by(|a, b| a.name().cmp(b.name()));

    let skill_contexts: Vec<SkillContext> = skill_list
        .into_iter()
        .map(|skill| SkillContext {
            name: skill.name().to_string(),
            description: if skill.description().len() > 80 {
                format!("{}...", &skill.description()[..77])
            } else {
                skill.description().to_string()
            },
        })
        .collect();

    let template = SkillsPromptTemplate {
        has_skills: !skill_contexts.is_empty(),
        skills: skill_contexts,
    };

    template.render(templates).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_file_valid() {
        let content = r#"---
name: pdf-processing
description: Extract text and tables from PDF files
license: MIT
---
# PDF Processing

This skill helps you work with PDF files.
"#;

        let (metadata, body) = parse_skill_file(content).unwrap();
        assert_eq!(metadata.name, "pdf-processing");
        assert_eq!(
            metadata.description,
            "Extract text and tables from PDF files"
        );
        assert_eq!(metadata.license, Some("MIT".to_string()));
        assert!(body.starts_with("# PDF Processing"));
    }

    #[test]
    fn test_parse_skill_file_no_frontmatter() {
        let content = "# Just markdown";
        assert!(parse_skill_file(content).is_err());
    }

    #[test]
    fn test_skill_metadata_validation() {
        let metadata = SkillMetadata {
            name: "valid-skill".to_string(),
            description: "A valid description".to_string(),
            license: None,
            compatibility: None,
            metadata: HashMap::default(),
            allowed_tools: None,
        };
        assert!(metadata.validate().is_ok());

        let metadata = SkillMetadata {
            name: "Invalid_Name".to_string(),
            description: "A valid description".to_string(),
            license: None,
            compatibility: None,
            metadata: HashMap::default(),
            allowed_tools: None,
        };
        assert!(metadata.validate().is_err());

        let metadata = SkillMetadata {
            name: "a".repeat(65),
            description: "A valid description".to_string(),
            license: None,
            compatibility: None,
            metadata: HashMap::default(),
            allowed_tools: None,
        };
        assert!(metadata.validate().is_err());
    }

    #[test]
    fn test_skill_resolve_path() {
        let skill = Skill {
            metadata: SkillMetadata {
                name: "test".to_string(),
                description: "test".to_string(),
                license: None,
                compatibility: None,
                metadata: HashMap::default(),
                allowed_tools: None,
            },
            body: String::new(),
            path: PathBuf::from("/home/user/.config/zed/skills/test"),
        };

        assert!(skill.resolve_path("scripts/run.sh").is_ok());
        assert!(skill.resolve_path("references/doc.md").is_ok());
        assert!(skill.resolve_path("../etc/passwd").is_err());
        assert!(skill.resolve_path("scripts/../../../etc/passwd").is_err());
    }
}
