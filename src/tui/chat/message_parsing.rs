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
    let colons: Vec<_> = input_text.match_indices(':').collect();
    let num_colons = colons.len();

    if num_colons == 0 {
        return false;
    }

    if let Some((last_colon_idx, _)) = colons.last() {
        let after_last_colon_idx = last_colon_idx + 1;
        if after_last_colon_idx < input_text.len() {
            if let Some(char_after_colon) = input_text.chars().nth(after_last_colon_idx) {
                if char_after_colon.is_whitespace() {
                    return false;
                }
            }
        } else {
            return true;
        }
    }

    num_colons % 2 != 0
}
