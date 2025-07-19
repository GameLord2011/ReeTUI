use crate::tui::chat::message_parsing::{replace_shortcodes_with_emojis, should_show_emoji_popup, should_show_mention_popup};

#[test]
fn test_replace_shortcodes_with_emojis() {
    assert_eq!(replace_shortcodes_with_emojis("Hello :smile: world"), "Hello 😄 world");
    assert_eq!(replace_shortcodes_with_emojis(":+1:"), "👍");
    assert_eq!(replace_shortcodes_with_emojis("No shortcode here"), "No shortcode here");
    assert_eq!(replace_shortcodes_with_emojis("Multiple :smile: :+1: emojis"), "Multiple 😄 👍 emojis");
    assert_eq!(replace_shortcodes_with_emojis("Invalid :not_an_emoji: shortcode"), "Invalid :not_an_emoji: shortcode");
    assert_eq!(replace_shortcodes_with_emojis("Colon at end:"), "Colon at end:");
    assert_eq!(replace_shortcodes_with_emojis("Colon in middle:test"), "Colon in middle:test");
}

#[test]
fn test_should_show_emoji_popup() {
    // Test cases where popup should be shown
    assert_eq!(should_show_emoji_popup("hello :"), true, "Should show for trailing colon");
    assert_eq!(should_show_emoji_popup("hello :s"), true, "Should show for unclosed shortcode");
    assert_eq!(should_show_emoji_popup("hello :smile"), true, "Should show for unclosed shortcode");

    // Test cases where popup should NOT be shown
    assert_eq!(should_show_emoji_popup("hello :smile:"), false, "Should not show for closed shortcode");
    assert_eq!(should_show_emoji_popup("hello :smile: "), false, "Should not show for closed shortcode followed by space");
    assert_eq!(should_show_emoji_popup("no colon"), false, "Should not show if no colon");
    assert_eq!(should_show_emoji_popup("two colons::"), false, "Should not show for double colon");
    assert_eq!(should_show_emoji_popup("two colons:a:"), false, "Should not show for closed shortcode");
    assert_eq!(should_show_emoji_popup("one : two"), false, "Should not show for colon followed by space");
    assert_eq!(should_show_emoji_popup("invalid : shortcode"), false, "Should not show for space in shortcode");
    assert_eq!(should_show_emoji_popup("invalid :s:hortcode"), false, "Should not show for invalid shortcode format");
    assert_eq!(should_show_emoji_popup("invalid :s: "), false, "Should not show for invalid shortcode format");
}

#[test]
fn test_should_show_mention_popup() {
    // Test cases where popup should be shown
    assert_eq!(should_show_mention_popup("hello @"), true, "Should show for trailing @");
    assert_eq!(should_show_mention_popup("hello @u"), true, "Should show for unclosed mention");
    assert_eq!(should_show_mention_popup("hello @user"), true, "Should show for unclosed mention");

    // Test cases where popup should NOT be shown
    assert_eq!(should_show_mention_popup("hello @user "), false, "Should not show for mention followed by space");
    assert_eq!(should_show_mention_popup("no at symbol"), false, "Should not show if no @");
    assert_eq!(should_show_mention_popup("@@"), false, "Should not show for double @");
    assert_eq!(should_show_mention_popup("hello @user@"), false, "Should not show for multiple @");
    assert_eq!(should_show_mention_popup("hello @user: "), false, "Should not show for invalid mention format");
}
