pub fn replace_shortcodes_with_emojis(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut current_pos = 0;

    while let Some(colon_start_idx) = text[current_pos..].find(':') {
        let absolute_colon_start_idx = current_pos + colon_start_idx;
        result.push_str(&text[current_pos..absolute_colon_start_idx]);

        let potential_shortcode_start = absolute_colon_start_idx + 1;
        if let Some(colon_end_idx) = text[potential_shortcode_start..].find(':') {
            let absolute_colon_end_idx = potential_shortcode_start + colon_end_idx;
            let shortcode_name = &text[potential_shortcode_start..absolute_colon_end_idx];

            if !shortcode_name.contains(' ') {
                if let Some(emoji) = emojis::get_by_shortcode(shortcode_name) {
                    result.push_str(emoji.as_str());
                    current_pos = absolute_colon_end_idx + 1;
                    continue;
                }
            }
        }

        result.push(':');
        current_pos = absolute_colon_start_idx + 1;
    }
    result.push_str(&text[current_pos..]);
    result
}

pub fn should_show_emoji_popup(input_text: &str) -> bool {
    if let Some(last_colon_idx) = input_text.rfind(':') {
        let potential_shortcode_segment_from_last_colon = &input_text[last_colon_idx..];

        // Rule: If it's a double colon (e.g., "::")
        if potential_shortcode_segment_from_last_colon == ":"
            && last_colon_idx > 0
            && input_text.chars().nth(last_colon_idx - 1).unwrap_or(' ') == ':'
        {
            return false;
        }

        // Rule: If there's a space immediately after the last colon (e.g., ": ")
        if potential_shortcode_segment_from_last_colon.len() > 1
            && potential_shortcode_segment_from_last_colon
                .chars()
                .nth(1)
                .unwrap_or(' ')
                .is_whitespace()
        {
            return false;
        }

        // Rule: If the last colon is part of a closed shortcode like ":smile:"
        // We need to look backwards to find the opening colon.
        if potential_shortcode_segment_from_last_colon.ends_with(':') {
            if let Some(open_colon_idx) = input_text[..last_colon_idx].rfind(':') {
                let segment = &input_text[open_colon_idx..=last_colon_idx];
                if segment.starts_with(':') && segment.ends_with(':') && segment.len() > 1 {
                    // Ensure the content between colons doesn't contain spaces or other colons
                    let inner_content = &segment[1..segment.len() - 1];
                    if !inner_content.contains(' ') && !inner_content.contains(':') {
                        return false; // It's a valid, completed shortcode
                    }
                }
            }
        }

        // Rule: If the last colon is part of a word (e.g., "foo:bar")
        // If the character before the last colon is alphanumeric and the segment is not just a single trailing colon,
        // it's likely not the start of an emoji shortcode.
        if potential_shortcode_segment_from_last_colon.len() > 1
            && last_colon_idx > 0
            && input_text
                .chars()
                .nth(last_colon_idx - 1)
                .unwrap_or(' ')
                .is_alphanumeric()
        {
            return false;
        }

        // If none of the "don't show" rules triggered, then show the popup.
        true
    } else {
        false // No colon found
    }
}

pub fn should_show_mention_popup(input_text: &str) -> bool {
    if let Some(last_at_idx) = input_text.rfind('@') {
        // Rule 1: Double '@' (e.g., "@@")
        // If the current '@' is immediately preceded by another '@'.
        if last_at_idx > 0 && input_text.chars().nth(last_at_idx - 1).unwrap_or(' ') == '@' {
            return false;
        }

        // Determine the "word" segment that contains the last '@'.
        // This segment starts from the last non-whitespace character before the last '@'
        // (or beginning of string) and extends to the end of the string.
        let mut word_start_for_last_at = last_at_idx;
        while word_start_for_last_at > 0
            && !input_text
                .chars()
                .nth(word_start_for_last_at - 1)
                .unwrap_or(' ')
                .is_whitespace()
        {
            word_start_for_last_at -= 1;
        }
        let segment_containing_last_at_word = &input_text[word_start_for_last_at..];

        // Rule 2: If there's a space within the potential mention word (e.g., "@user name" or "@user ")
        // This check needs to apply to the part *after* the initial '@' within the "word".
        // If the segment starts with '@' and contains a space anywhere after that '@', it's invalid.
        if segment_containing_last_at_word.starts_with('@')
            && segment_containing_last_at_word[1..].contains(' ')
        {
            return false;
        }

        // Rule 3: If the "word" containing the last '@' itself contains another '@' symbol
        // after its initial character (e.g., "@user@name" or "@user@").
        // This correctly handles "hello @user@".
        if segment_containing_last_at_word.starts_with('@')
            && segment_containing_last_at_word[1..].contains('@')
        {
            return false;
        }

        // If none of the above "don't show" conditions are met, then show the popup.
        true
    } else {
        // No '@' found in the input string.
        false
    }
}
