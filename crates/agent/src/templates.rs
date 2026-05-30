use anyhow::Result;
use gpui::SharedString;
use handlebars::Handlebars;
use rust_embed::RustEmbed;
use serde::Serialize;
use std::sync::Arc;

#[derive(RustEmbed)]
#[folder = "src/templates"]
#[include = "*.hbs"]
struct Assets;

pub struct Templates(Handlebars<'static>);

impl Templates {
    pub fn new() -> Arc<Self> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_helper("contains", Box::new(contains));
        handlebars.register_embed_templates::<Assets>().unwrap();
        Arc::new(Self(handlebars))
    }
}

pub trait Template: Sized {
    const TEMPLATE_NAME: &'static str;

    fn render(&self, templates: &Templates) -> Result<String>
    where
        Self: Serialize + Sized,
    {
        Ok(templates.0.render(Self::TEMPLATE_NAME, self)?)
    }
}

#[derive(Serialize)]
pub struct SystemPromptTemplate<'a> {
    #[serde(flatten)]
    pub project: &'a prompt_store::ProjectContext,
    pub available_tools: Vec<SharedString>,
    pub model_name: Option<String>,
}

impl Template for SystemPromptTemplate<'_> {
    const TEMPLATE_NAME: &'static str = "system_prompt.hbs";
}

/// Handlebars helper for checking if an item is in a list
fn contains(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let list = h
        .param(0)
        .and_then(|v| v.value().as_array())
        .ok_or_else(|| {
            handlebars::RenderError::new("contains: missing or invalid list parameter")
        })?;
    let query = h.param(1).map(|v| v.value()).ok_or_else(|| {
        handlebars::RenderError::new("contains: missing or invalid query parameter")
    })?;

    if list.contains(query) {
        out.write("true")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_template() {
        let project = prompt_store::ProjectContext::default();
        let template = SystemPromptTemplate {
            project: &project,
            available_tools: vec!["echo".into()],
            model_name: Some("test-model".to_string()),
        };
        let templates = Templates::new();
        let rendered = template.render(&templates).unwrap();
        assert!(rendered.contains("## Fixing Diagnostics"));
        assert!(!rendered.contains("## Planning"));
        assert!(rendered.contains("test-model"));
    }

    #[test]
    fn test_grep_guidance_varies_by_lsp_tool_availability() {
        let project = prompt_store::ProjectContext::default();
        let templates = Templates::new();

        // When LSP tools are available, grep guidance should be scoped to
        // non-symbol search and the LSP section should be present.
        let with_lsp = SystemPromptTemplate {
            project: &project,
            available_tools: vec![
                "grep".into(),
                "find_references".into(),
                "go_to_definition".into(),
                "rename_symbol".into(),
                "get_code_actions".into(),
                "apply_code_action".into(),
            ],
            model_name: None,
        };
        let rendered = with_lsp.render(&templates).unwrap();
        assert!(
            rendered.contains("non-symbol"),
            "with LSP tools: should scope grep to non-symbol search"
        );
        assert!(
            rendered.contains("## Language Server Tools"),
            "with LSP tools: should include LSP section"
        );
        assert!(
            !rendered.contains("When looking for symbols in the project, prefer the `grep` tool"),
            "with LSP tools: should NOT tell model to prefer grep for symbols"
        );

        // When no LSP tools are available, grep guidance should fall back to
        // the original symbol-search advice and the LSP section should be absent.
        let without_lsp = SystemPromptTemplate {
            project: &project,
            available_tools: vec!["grep".into(), "read_file".into()],
            model_name: None,
        };
        let rendered = without_lsp.render(&templates).unwrap();
        assert!(
            rendered.contains("When looking for symbols in the project, prefer the `grep` tool"),
            "without LSP tools: should prefer grep for symbols"
        );
        assert!(
            !rendered.contains("## Language Server Tools"),
            "without LSP tools: should NOT include LSP section"
        );
        assert!(
            !rendered.contains("non-symbol"),
            "without LSP tools: should NOT mention non-symbol scoping"
        );
    }
}
