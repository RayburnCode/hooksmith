use serde::Serialize;

use crate::WebhookError;

// ---------------------------------------------------------------------------
// Public helper
// ---------------------------------------------------------------------------

/// Format any serializable value as a Discord JSON code block.
///
/// The result looks like:
/// ````text
/// ```json
/// {
///   "key": "value"
/// }
/// ```
/// ````
///
/// Useful when you want to compose the string yourself before passing it to
/// [`WebhookMessageBuilder::content`] or [`EmbedBuilder::description`].
pub fn json_code_block(value: &impl Serialize) -> Result<String, WebhookError> {
    let pretty = serde_json::to_string_pretty(value)?;
    Ok(format!("```json\n{pretty}\n```"))
}

// ---------------------------------------------------------------------------
// AllowedMentions
// ---------------------------------------------------------------------------

/// Controls which mentions Discord will actually notify.
///
/// For user-submitted content (e.g. a lead form) you should call
/// [`AllowedMentions::none()`] to prevent accidental `@everyone` or role pings.
#[derive(Serialize, Debug, Clone, Default)]
pub struct AllowedMentions {
    /// Parse types to auto-detect and notify.  Empty = no auto-parsing.
    pub parse: Vec<AllowedMentionType>,
    /// Explicit role IDs whose mentions should be notified.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Explicit user IDs whose mentions should be notified.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<String>,
}

impl AllowedMentions {
    /// Allow no mentions at all — safe default for user-generated content.
    pub fn none() -> Self {
        Self { parse: vec![], roles: vec![], users: vec![] }
    }

    /// Allow mentions for specific user IDs only.
    pub fn users(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self { parse: vec![], roles: vec![], users: ids.into_iter().map(Into::into).collect() }
    }

    /// Allow `@everyone` / `@here`, role mentions, and user mentions.
    pub fn all() -> Self {
        Self {
            parse: vec![
                AllowedMentionType::Everyone,
                AllowedMentionType::Roles,
                AllowedMentionType::Users,
            ],
            roles: vec![],
            users: vec![],
        }
    }
}

/// The categories of mentions Discord can auto-parse from message text.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AllowedMentionType {
    Roles,
    Users,
    Everyone,
}

// ---------------------------------------------------------------------------
// Message flags
// ---------------------------------------------------------------------------

/// Bitfield constants for the `flags` field on [`WebhookMessage`].
///
/// # Example
///
/// ```rust
/// use discord_hook::{WebhookMessage, flags};
///
/// let msg = WebhookMessage::builder()
///     .content("Quiet alert")
///     .flag(flags::SUPPRESS_NOTIFICATIONS)
///     .build()
///     .unwrap();
/// ```
pub mod flags {
    /// Do not include any embeds when serialising this message.
    pub const SUPPRESS_EMBEDS: u64 = 1 << 2; // 4
    /// Send the message without triggering a push / desktop notification.
    pub const SUPPRESS_NOTIFICATIONS: u64 = 1 << 12; // 4096
    /// Use Components V2 layout. When set, `content`, `embeds`, `files`, and
    /// `poll` must all be absent — the message body is driven entirely by
    /// `components`. Can only be set, never unset after creation.
    pub const IS_COMPONENTS_V2: u64 = 1 << 15; // 32768
}

// ---------------------------------------------------------------------------
// Embed sub-types
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug, Clone)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline: Option<bool>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EmbedFooter {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EmbedAuthor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EmbedImage {
    pub url: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct EmbedThumbnail {
    pub url: String,
}

// ---------------------------------------------------------------------------
// Embed
// ---------------------------------------------------------------------------

/// A Discord rich embed.  Build one with [`Embed::builder()`].
#[derive(Serialize, Debug, Clone, Default)]
pub struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Colour as a 24-bit integer, e.g. `0xFF5733`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<EmbedFooter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<EmbedThumbnail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<EmbedImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<EmbedAuthor>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<EmbedField>,
    /// ISO 8601 timestamp string, e.g. `"2024-01-01T00:00:00.000Z"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

impl Embed {
    pub fn builder() -> EmbedBuilder {
        EmbedBuilder::default()
    }
}

#[derive(Default)]
pub struct EmbedBuilder {
    inner: Embed,
}

impl EmbedBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.inner.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.inner.description = Some(description.into());
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.inner.url = Some(url.into());
        self
    }

    /// Set the sidebar colour as a 24-bit integer, e.g. `0xFF5733`.
    pub fn color(mut self, color: u32) -> Self {
        self.inner.color = Some(color);
        self
    }

    pub fn footer(mut self, text: impl Into<String>) -> Self {
        self.inner.footer = Some(EmbedFooter { text: text.into(), icon_url: None });
        self
    }

    pub fn footer_with_icon(mut self, text: impl Into<String>, icon_url: impl Into<String>) -> Self {
        self.inner.footer = Some(EmbedFooter {
            text: text.into(),
            icon_url: Some(icon_url.into()),
        });
        self
    }

    pub fn thumbnail(mut self, url: impl Into<String>) -> Self {
        self.inner.thumbnail = Some(EmbedThumbnail { url: url.into() });
        self
    }

    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.inner.image = Some(EmbedImage { url: url.into() });
        self
    }

    pub fn author(mut self, name: impl Into<String>) -> Self {
        self.inner.author = Some(EmbedAuthor { name: name.into(), url: None, icon_url: None });
        self
    }

    pub fn author_full(
        mut self,
        name: impl Into<String>,
        url: Option<impl Into<String>>,
        icon_url: Option<impl Into<String>>,
    ) -> Self {
        self.inner.author = Some(EmbedAuthor {
            name: name.into(),
            url: url.map(Into::into),
            icon_url: icon_url.map(Into::into),
        });
        self
    }

    /// Add a field.  Pass `inline: true` to render side-by-side with adjacent fields.
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>, inline: bool) -> Self {
        self.inner.fields.push(EmbedField {
            name: name.into(),
            value: value.into(),
            inline: Some(inline),
        });
        self
    }

    /// ISO 8601 timestamp shown in the embed footer, e.g. `"2024-01-01T00:00:00.000Z"`.
    pub fn timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.inner.timestamp = Some(timestamp.into());
        self
    }

    /// Set the embed description to a pretty-printed JSON code block.
    ///
    /// ```rust
    /// # use discord_hook::Embed;
    /// # use serde::Serialize;
    /// #[derive(Serialize)]
    /// struct Event { kind: String, code: u32 }
    ///
    /// let embed = Embed::builder()
    ///     .title("Event payload")
    ///     .json_description(&Event { kind: "deploy".into(), code: 0 })
    ///     .expect("serialization failed")
    ///     .build();
    /// ```
    pub fn json_description(mut self, value: &impl Serialize) -> Result<Self, WebhookError> {
        self.inner.description = Some(json_code_block(value)?);
        Ok(self)
    }

    /// Add a field whose value is a pretty-printed JSON code block.
    ///
    /// Discord field values are capped at 1 024 characters; make sure your
    /// serialized value fits within that limit.
    pub fn json_field(
        mut self,
        name: impl Into<String>,
        value: &impl Serialize,
        inline: bool,
    ) -> Result<Self, WebhookError> {
        self.inner.fields.push(EmbedField {
            name: name.into(),
            value: json_code_block(value)?,
            inline: Some(inline),
        });
        Ok(self)
    }

    pub fn build(self) -> Embed {
        self.inner
    }
}

// ---------------------------------------------------------------------------
// WebhookMessage
// ---------------------------------------------------------------------------

/// The top-level payload sent to a Discord webhook.  Build one with
/// [`WebhookMessage::builder()`].
#[derive(Serialize, Debug, Clone, Default)]
pub struct WebhookMessage {
    /// Plain-text message content (up to 2000 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Override the webhook's display name for this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Override the webhook's avatar URL for this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// Send as a text-to-speech message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
    /// Rich embeds (up to 10 per message).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
    /// Controls which `@mentions` in `content` actually ping someone.
    /// Set to [`AllowedMentions::none()`] when passing user-submitted text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,
    /// Message flags bitfield.  Use constants from [`flags`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    /// Create a new thread with this name and post the message into it.
    /// Only valid when the webhook targets a forum or media channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_name: Option<String>,
    /// Tag IDs to apply to the newly-created thread.
    /// Only valid alongside `thread_name` in forum or media channels.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub applied_tags: Vec<String>,
}

impl WebhookMessage {
    pub fn builder() -> WebhookMessageBuilder {
        WebhookMessageBuilder::default()
    }
}

#[derive(Default)]
pub struct WebhookMessageBuilder {
    inner: WebhookMessage,
}

impl WebhookMessageBuilder {
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.inner.content = Some(content.into());
        self
    }

    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.inner.username = Some(username.into());
        self
    }

    pub fn avatar_url(mut self, url: impl Into<String>) -> Self {
        self.inner.avatar_url = Some(url.into());
        self
    }

    pub fn tts(mut self, tts: bool) -> Self {
        self.inner.tts = Some(tts);
        self
    }

    /// Set the allowed mentions for this message.
    ///
    /// **Always call `.allowed_mentions(AllowedMentions::none())` when the
    /// message content includes any user-submitted text**, to prevent
    /// accidental `@everyone` or role pings.
    pub fn allowed_mentions(mut self, allowed_mentions: AllowedMentions) -> Self {
        self.inner.allowed_mentions = Some(allowed_mentions);
        self
    }

    /// Apply a message flag from the [`flags`] module.  Can be called multiple
    /// times to OR flags together.
    ///
    /// ```rust
    /// use discord_hook::{WebhookMessage, flags};
    ///
    /// let msg = WebhookMessage::builder()
    ///     .content("Silent alert")
    ///     .flag(flags::SUPPRESS_NOTIFICATIONS)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn flag(mut self, flag: u64) -> Self {
        self.inner.flags = Some(self.inner.flags.unwrap_or(0) | flag);
        self
    }

    /// Create a new thread with this name and post the message into it.
    /// Only valid when the webhook targets a forum or media channel.
    pub fn thread_name(mut self, name: impl Into<String>) -> Self {
        self.inner.thread_name = Some(name.into());
        self
    }

    /// Apply forum/media channel tag IDs to the thread created by `thread_name`.
    /// Call multiple times or pass multiple IDs to apply several tags.
    pub fn applied_tag(mut self, tag_id: impl Into<String>) -> Self {
        self.inner.applied_tags.push(tag_id.into());
        self
    }

    /// Set the message content to a pretty-printed JSON code block.
    ///
    /// If [`content`](Self::content) was already called, the code block is
    /// appended on a new line after the existing text — so you can combine a
    /// human-readable description with the raw payload:
    ///
    /// ```rust
    /// # use discord_hook::WebhookMessage;
    /// # use serde::Serialize;
    /// #[derive(Serialize)]
    /// struct Build { branch: String, passed: bool }
    ///
    /// let msg = WebhookMessage::builder()
    ///     .content("Build result:")
    ///     .json_content(&Build { branch: "main".into(), passed: true })
    ///     .expect("serialization failed")
    ///     .build()
    ///     .expect("valid message");
    /// ```
    pub fn json_content(mut self, value: &impl Serialize) -> Result<Self, WebhookError> {
        let block = json_code_block(value)?;
        self.inner.content = Some(match self.inner.content.take() {
            Some(existing) => format!("{existing}\n{block}"),
            None => block,
        });
        Ok(self)
    }

    /// Attach an embed.  Up to 10 embeds are allowed per message.
    pub fn embed(mut self, embed: Embed) -> Self {
        self.inner.embeds.push(embed);
        self
    }

    /// Validate and return the finished message.
    ///
    /// Returns [`WebhookError::EmptyMessage`] if neither `content` nor any
    /// embeds were provided.
    pub fn build(self) -> Result<WebhookMessage, WebhookError> {
        if self.inner.content.is_none() && self.inner.embeds.is_empty() {
            return Err(WebhookError::EmptyMessage);
        }
        Ok(self.inner)
    }
}
