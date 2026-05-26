use gpui::{Context, EventEmitter};

/// An observable entity that tracks the progress of an in-progress (or completed)
/// thread summary generation. Created by the UI layer and passed into the agent's
/// summary generation pipeline so it survives crease deletion in the editor.
pub struct SummaryProgress {
    /// The summary text generated so far (updated incrementally as streaming progresses).
    pub partial_text: String,
    /// A rough count of the number of streaming text deltas received.
    pub output_tokens: usize,
    /// Whether the generation has completed successfully.
    pub is_complete: bool,
    /// An error message if the generation failed.
    pub error: Option<String>,
}

impl SummaryProgress {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        log::info!("[SummaryProgress] Entity created");
        Self {
            partial_text: String::new(),
            output_tokens: 0,
            is_complete: false,
            error: None,
        }
    }

    /// Called when a new text delta is received during streaming.
    pub fn append_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.partial_text.push_str(text);
        self.output_tokens += 1;
        if self.output_tokens == 1 {
            log::info!(
                "[SummaryProgress] First text chunk received! {} chars",
                self.partial_text.len()
            );
        } else if self.output_tokens % 50 == 0 {
            log::debug!(
                "[SummaryProgress] append_text #{}: {} chars total",
                self.output_tokens,
                self.partial_text.len()
            );
        }
        cx.notify();
    }

    /// Called when generation completes successfully.
    pub fn mark_complete(&mut self, cx: &mut Context<Self>) {
        log::debug!(
            "[SummaryProgress] mark_complete: {} tokens, {} chars",
            self.output_tokens,
            self.partial_text.len()
        );
        self.is_complete = true;
        cx.notify();
    }

    /// Called when generation fails.
    pub fn mark_error(&mut self, error: String, cx: &mut Context<Self>) {
        log::error!("[SummaryProgress] mark_error: {}", error);
        self.error = Some(error);
        cx.notify();
    }
}

/// Emitted when the summary generation finishes (either success or failure).
pub enum SummaryProgressEvent {
    Completed,
    Failed(String),
}

impl EventEmitter<SummaryProgressEvent> for SummaryProgress {}
