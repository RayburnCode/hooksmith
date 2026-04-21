/// Build a [`WebhookMessage`] using key `=` value syntax.
///
/// Each key maps directly to a [`WebhookMessageBuilder`] method. The macro
/// expands to the equivalent builder chain and returns
/// `Result<WebhookMessage, WebhookError>` — the same type as calling
/// `.build()` manually.
///
/// Methods that accept a single argument work out of the box. For multi-argument
/// builder calls (e.g. [`EmbedBuilder::field`]) or fallible methods such as
/// [`WebhookMessageBuilder::json_content`], use the full builder instead.
///
/// # Examples
///
/// ```rust
/// use discord_hook::{discord_message, flags};
///
/// // Minimal — content only.
/// let msg = discord_message!(content = "Deploy succeeded").unwrap();
///
/// // Multiple fields.
/// let msg = discord_message!(
///     content  = "Silent alert",
///     username = "CI Bot",
///     flag     = flags::SUPPRESS_NOTIFICATIONS,
/// ).unwrap();
/// ```
#[macro_export]
macro_rules! discord_message {
    ($($key:ident = $val:expr),+ $(,)?) => {
        $crate::WebhookMessage::builder()
            $(.$key($val))+
            .build()
    };
}

/// Build an [`Embed`] using key `=` value syntax.
///
/// Each key maps directly to an [`EmbedBuilder`] method. The macro expands
/// to the equivalent builder chain and returns the finished [`Embed`] — the
/// builder's `.build()` is infallible.
///
/// Methods that accept a single argument work out of the box. Multi-argument
/// methods (e.g. [`EmbedBuilder::field`], [`EmbedBuilder::author_full`]) and
/// fallible methods (e.g. [`EmbedBuilder::json_description`]) require the full
/// builder.
///
/// # Examples
///
/// ```rust
/// use discord_hook::embed;
///
/// let embed = embed!(
///     title       = "Deployment",
///     description = "Production release complete.",
///     color       = 0x57F287u32,   // green
/// );
///
/// // Can be nested directly in discord_message! via the builder:
/// use discord_hook::{WebhookMessage};
/// let msg = WebhookMessage::builder()
///     .embed(embed!(title = "Alert", color = 0xED4245u32))
///     .build()
///     .unwrap();
/// ```
#[macro_export]
macro_rules! embed {
    ($($key:ident = $val:expr),+ $(,)?) => {
        $crate::Embed::builder()
            $(.$key($val))+
            .build()
    };
}
