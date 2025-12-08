/// Utility functions for cleaning and parsing Ollama responses

/// Check if raw output appears to be truncated
pub fn is_truncated(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    
    // Check if last non-whitespace char is a truncation indicator
    if let Some(last_char) = trimmed.chars().rev().find(|c| !c.is_whitespace()) {
        if matches!(last_char, '{' | '[' | ':' | '"' | ',') {
            return true;
        }
    }
    
    // Check brace/bracket balance
    let mut brace_count = 0;
    let mut bracket_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    
    for ch in trimmed.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => brace_count += 1,
            '}' if !in_string => brace_count -= 1,
            '[' if !in_string => bracket_count += 1,
            ']' if !in_string => bracket_count -= 1,
            _ => {}
        }
    }
    
    // If braces/brackets don't balance or string is unclosed, likely truncated
    brace_count != 0 || bracket_count != 0 || in_string
}

/// Sanitize raw model output before JSON extraction
pub fn sanitize_raw_output(raw: &str) -> String {
    let mut sanitized = raw.to_string();
    
    // Remove markdown code fences
    sanitized = sanitized.replace("```json", "");
    sanitized = sanitized.replace("```", "");
    
    // Remove LaTeX sequences \( ... \) and \[ ... \]
    // Simple regex-like replacement
    let mut result = String::with_capacity(sanitized.len());
    let chars: Vec<char> = sanitized.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '\\' {
            if chars[i + 1] == '(' || chars[i + 1] == ')' || chars[i + 1] == '[' || chars[i + 1] == ']' {
                // Skip LaTeX markers
                i += 2;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    
    // Replace smart quotes
    result = result.replace('\u{201C}', "\""); // Left double quotation mark
    result = result.replace('\u{201D}', "\""); // Right double quotation mark
    result = result.replace('\u{2018}', "'");  // Left single quotation mark
    result = result.replace('\u{2019}', "'");  // Right single quotation mark
    
    // Remove trailing commas before } or ]
    result = remove_trailing_commas(&result);
    
    // Collapse multiple whitespace sequences
    let mut collapsed = String::with_capacity(result.len());
    let mut last_was_whitespace = false;
    for ch in result.chars() {
        if ch.is_whitespace() {
            if !last_was_whitespace {
                collapsed.push(' ');
                last_was_whitespace = true;
            }
        } else {
            collapsed.push(ch);
            last_was_whitespace = false;
        }
    }
    
    collapsed.trim().to_string()
}

/// Fix unescaped backslashes in JSON strings (common issue with LaTeX notation like \(n\))
/// Replaces common LaTeX patterns with escaped versions or plain text alternatives
pub fn fix_unescaped_backslashes(json: &str) -> String {
    // Common LaTeX patterns that need fixing in JSON strings
    // We'll replace them with properly escaped versions
    let result = json.to_string();
    
    // Fix common LaTeX patterns that appear in string values
    // Note: This is a simple approach - replace known problematic patterns
    // Pattern: \( becomes \\( (escaped backslash + paren)
    // Pattern: \) becomes \\) (escaped backslash + paren)
    // Pattern: \pmod{ becomes \pmod{ (but backslash needs escaping)
    
    // Use a more sophisticated approach: find unescaped backslashes in strings
    let mut fixed = String::with_capacity(json.len() * 2);
    let mut in_string = false;
    let mut escape_next = false;
    let mut chars = json.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if escape_next {
            escape_next = false;
            // We just saw a backslash, now processing the escaped char
            match ch {
                // Valid JSON escape sequences - keep as-is
                '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u' => {
                    fixed.push('\\');
                    fixed.push(ch);
                    // If it's 'u', consume the hex digits
                    if ch == 'u' {
                        for _ in 0..4 {
                            if let Some(next) = chars.next() {
                                fixed.push(next);
                            }
                        }
                    }
                }
                // Invalid escape - this backslash should have been escaped
                // Escape the backslash and include the char
                _ => {
                    fixed.push('\\');
                    fixed.push('\\');
                    fixed.push(ch);
                }
            }
            continue;
        }
        
        match ch {
            '\\' if in_string => {
                // Check next char to see if it's a valid escape
                if let Some(&next) = chars.peek() {
                    match next {
                        '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u' => {
                            // Valid escape sequence
                            escape_next = true;
                        }
                        _ => {
                            // Invalid escape - escape the backslash
                            fixed.push('\\');
                            fixed.push('\\');
                        }
                    }
                } else {
                    // Trailing backslash - escape it
                    fixed.push('\\');
                    fixed.push('\\');
                }
            }
            '"' => {
                in_string = !in_string;
                fixed.push(ch);
            }
            _ => {
                fixed.push(ch);
            }
        }
    }
    
    fixed
}

/// Remove trailing commas from JSON (invalid but common in LLM outputs)
pub fn remove_trailing_commas(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let chars: Vec<char> = json.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let ch = chars[i];
        
        if ch == ',' {
            // Look ahead to see if this is a trailing comma
            let mut j = i + 1;
            // Skip whitespace
            while j < chars.len() && matches!(chars[j], ' ' | '\n' | '\t' | '\r') {
                j += 1;
            }
            // Check if next non-whitespace char is } or ]
            if j < chars.len() && matches!(chars[j], '}' | ']') {
                // This is a trailing comma, skip it
                i += 1;
                continue;
            }
        }
        
        result.push(ch);
        i += 1;
    }
    
    result
}

/// Validate JSON structure before parsing
/// Returns true if JSON appears valid (has balanced braces and basic structure)
fn validate_json_structure(json: &str) -> bool {
    let mut brace_count = 0;
    let mut bracket_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    
    for ch in json.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }
        
        match ch {
            '"' if !in_string => in_string = true,
            '"' if in_string => in_string = false,
            '\\' if in_string => escape_next = true,
            '{' if !in_string => brace_count += 1,
            '}' if !in_string => {
                if brace_count == 0 {
                    return false; // Unmatched closing brace
                }
                brace_count -= 1;
            }
            '[' if !in_string => bracket_count += 1,
            ']' if !in_string => {
                if bracket_count == 0 {
                    return false; // Unmatched closing bracket
                }
                bracket_count -= 1;
            }
            _ => {}
        }
    }
    
    brace_count == 0 && bracket_count == 0 && !in_string
}

/// Extract JSON from model response with validation and fallback strategies
/// Optimized single-pass extraction with multiple fallback strategies
/// Note: Input should already be sanitized via sanitize_raw_output
pub fn extract_json(text: &str) -> anyhow::Result<String> {
    // Strategy 0: Try parsing the text directly first (in case it's already clean JSON)
    let trimmed = text.trim();
    if let Ok(_) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return Ok(trimmed.to_string());
    }
    
    // Strategy 0.25: Extract from markdown code block if present (common with DeepSeek)
    if trimmed.starts_with("```json") || trimmed.starts_with("```") {
        if let Some(code_block_start) = trimmed.find("```") {
            let after_marker = &trimmed[code_block_start + 3..];
            // Check if it's ```json
            let json_block_start = if after_marker.starts_with("json") {
                after_marker[4..].trim_start()
            } else {
                after_marker.trim_start()
            };
            
            // Find the closing ```
            if let Some(close_idx) = json_block_start.find("```") {
                let json_content = json_block_start[..close_idx].trim_end();
                if let Ok(_) = serde_json::from_str::<serde_json::Value>(json_content) {
                    tracing::debug!("Strategy 0.25: Successfully extracted JSON from markdown code block");
                    return Ok(json_content.to_string());
                }
            } else {
                // No closing ```, try the whole thing after the opening
                let json_content = json_block_start.trim_end();
                if json_content.starts_with('{') {
                    // Try to find matching closing brace
                    let mut brace_count = 0;
                    let mut in_string = false;
                    let mut escape_next = false;
                    let mut end_pos = None;
                    
                    for (i, ch) in json_content.char_indices() {
                        if escape_next {
                            escape_next = false;
                            continue;
                        }
                        
                        match ch {
                            '\\' if in_string => escape_next = true,
                            '"' => in_string = !in_string,
                            '{' if !in_string => brace_count += 1,
                            '}' if !in_string => {
                                brace_count -= 1;
                                if brace_count == 0 {
                                    end_pos = Some(i + 1);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    
                    if let Some(end) = end_pos {
                        let candidate = &json_content[..end];
                        if let Ok(_) = serde_json::from_str::<serde_json::Value>(candidate) {
                            tracing::debug!("Strategy 0.25: Successfully extracted JSON from incomplete code block");
                            return Ok(candidate.to_string());
                        }
                    }
                }
            }
        }
    }
    
    // Strategy 0.5: If it starts with {, try to find the complete JSON object
    if trimmed.starts_with('{') {
        // Find the matching closing brace accounting for strings and escapes
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut last_valid_end = None;
        
        for (i, ch) in trimmed.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => {
                    brace_count += 1;
                }
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        last_valid_end = Some(i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        
        // Try parsing the complete JSON object we found
        if let Some(end) = last_valid_end {
            let candidate = trimmed[..end].trim();
            if let Ok(_) = serde_json::from_str::<serde_json::Value>(candidate) {
                tracing::debug!("Strategy 0.5: Successfully extracted JSON by finding matching braces");
                return Ok(candidate.to_string());
            }
        }
        
        // Also try the entire trimmed text if it looks complete
        if trimmed.ends_with('}') {
            if let Ok(_) = serde_json::from_str::<serde_json::Value>(trimmed) {
                tracing::debug!("Strategy 0.5: Successfully parsed entire trimmed text");
                return Ok(trimmed.to_string());
            }
        }
    }
    
    // Single-pass optimization: find the best JSON candidate in one scan
    // This version properly tracks string boundaries to handle LaTeX and escaped characters
    // Special handling for markdown code blocks (common with DeepSeek)
    let mut json_start = None;
    let mut json_end = None;
    let mut in_code_block = false;
    let mut code_block_start_idx = None;
    let mut brace_count = 0;
    let mut best_start = None;
    let mut best_end = None;
    let mut in_string = false;
    let mut escape_next = false;
    
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        // Check for code block markers
        if i + 3 <= chars.len() {
            let marker: String = chars[i..i+3].iter().collect();
            if marker == "```" {
                if in_code_block {
                    // End of code block - if we were tracking JSON inside, this is the end
                    if let Some(start) = json_start {
                        // If we found JSON start, use current position as potential end
                        if json_end.is_none() {
                            // Only set end if we haven't found a complete JSON object yet
                            if brace_count > 0 {
                                // Incomplete JSON, but use code block end
                                json_end = Some(i);
                                best_end = Some(i);
                            } else if brace_count == 0 && best_end.is_none() {
                                // Complete JSON, code block ends here
                                json_end = Some(i);
                                best_end = Some(i);
                            }
                        }
                    }
                    in_code_block = false;
                    code_block_start_idx = None;
                } else {
                    // Start of code block
                    in_code_block = true;
                    code_block_start_idx = Some(i);
                    if i + 7 <= chars.len() {
                        let json_marker: String = chars[i..i+7].iter().collect();
                        if json_marker == "```json" {
                            i += 7;
                            // Skip whitespace and newlines
                            while i < chars.len() && (chars[i].is_whitespace() || chars[i] == '\n' || chars[i] == '\r') {
                                i += 1;
                            }
                            if i < chars.len() {
                                json_start = Some(i);
                                best_start = Some(i);
                                brace_count = 0;
                                in_string = false;
                                escape_next = false;
                            }
                            continue;
                        }
                    }
                    // Generic code block (not ```json)
                    i += 3;
                    // Skip whitespace
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == '{' {
                        json_start = Some(i);
                        brace_count = 1;
                        best_start = Some(i);
                        in_string = false;
                        escape_next = false;
                    }
                    continue;
                }
                i += 3;
                continue;
            }
        }
        
        // Track braces for JSON boundaries (accounting for strings and escapes)
        if escape_next {
            escape_next = false;
            i += 1;
            continue;
        }
        
        let ch = chars[i];
        
        if let Some(start) = json_start {
            match ch {
                '\\' if in_string => {
                    escape_next = true;
                }
                '"' => {
                    in_string = !in_string;
                }
                '{' if !in_string => {
                    if brace_count == 0 {
                        best_start = Some(start);
                    }
                    brace_count += 1;
                }
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        best_end = Some(i + 1);
                        // Don't break - keep scanning in case there's a closing code block
                        // But mark that we found complete JSON
                    }
                }
                _ => {}
            }
        } else if ch == '{' && !in_code_block {
            // Found JSON start outside code block
            json_start = Some(i);
            brace_count = 1;
            best_start = Some(i);
            in_string = false;
            escape_next = false;
        }
        
        i += 1;
    }
    
    // If we were in a code block and found JSON start but no end, use end of text or code block
    if in_code_block && json_start.is_some() && best_end.is_none() {
        // JSON might be incomplete, but try to extract what we have
        best_end = Some(chars.len());
    }
    
    // Use best match found
    let json_str = if let (Some(start_idx), Some(end_idx)) = (best_start, best_end) {
        let extracted = &text[start_idx..end_idx];
        // Trim any trailing whitespace/newlines that might be before closing ```
        extracted.trim_end().to_string()
    } else if let Some(start_idx) = best_start {
        // Incomplete JSON, try to find end accounting for strings
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut end = start_idx;
        
        for (i, ch) in text[start_idx..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end = start_idx + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if brace_count == 0 && end > start_idx {
            text[start_idx..end].trim_end().to_string()
        } else if end > start_idx {
            // Incomplete but try to extract anyway
            text[start_idx..end].trim_end().to_string()
        } else {
            text.trim().to_string()
        }
    } else {
        text.trim().to_string()
    };
    
    // Remove trailing commas and fix unescaped backslashes
    let cleaned = remove_trailing_commas(&json_str);
    
    // Strategy 1: Try parsing as-is first
    match serde_json::from_str::<serde_json::Value>(&cleaned) {
        Ok(_) => return Ok(cleaned),
        Err(e) => {
            tracing::debug!(
                error = %e,
                json_preview = &cleaned[..cleaned.len().min(200)],
                "Initial parse failed, trying to fix backslashes"
            );
        }
    }
    
    // Strategy 1.5: Fix unescaped backslashes (common with LaTeX notation)
    let fixed_backslashes = fix_unescaped_backslashes(&cleaned);
    match serde_json::from_str::<serde_json::Value>(&fixed_backslashes) {
        Ok(_) => {
            tracing::debug!("Successfully fixed unescaped backslashes");
            return Ok(fixed_backslashes);
        }
        Err(e) => {
            tracing::debug!(
                error = %e,
                "Backslash fixing didn't help"
            );
        }
    }
    
    // Strategy 1.6: Validate structure and try original
    if validate_json_structure(&cleaned) {
        // Try to parse to ensure it's valid JSON
        match serde_json::from_str::<serde_json::Value>(&cleaned) {
            Ok(_) => return Ok(cleaned),
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    json_preview = &cleaned[..cleaned.len().min(100)],
                    "JSON structure valid but parse failed, trying fallbacks"
                );
            }
        }
    }
    
    // Strategy 2: Try to find JSON object boundaries more aggressively
    // But this time, properly account for strings when finding the closing brace
    if let Some(start) = cleaned.find('{') {
        // Find the matching closing brace, accounting for strings
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut end_pos = None;
        
        for (i, ch) in cleaned[start..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_pos = Some(start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        
        if let Some(end) = end_pos {
            if end > start {
                let candidate = &cleaned[start..end];
                if validate_json_structure(candidate) {
                    match serde_json::from_str::<serde_json::Value>(candidate) {
                        Ok(_) => return Ok(candidate.to_string()),
                        Err(e) => {
                            tracing::debug!(
                                error = %e,
                                "Strategy 2: Found boundaries but parse failed"
                            );
                        }
                    }
                }
            }
        }
        
        // Fallback: try simple start/end if string-aware method failed
        if let Some(end) = cleaned.rfind('}') {
            if end > start {
                let candidate = &cleaned[start..=end];
                if validate_json_structure(candidate) {
                    match serde_json::from_str::<serde_json::Value>(candidate) {
                        Ok(_) => return Ok(candidate.to_string()),
                        Err(_) => {}
                    }
                }
            }
        }
    }
    
    // Strategy 3: Try to fix common issues
    let fixed = cleaned
        .replace(",}", "}")
        .replace(",]", "]")
        .replace(",,", ",");
    
    if validate_json_structure(&fixed) {
        match serde_json::from_str::<serde_json::Value>(&fixed) {
            Ok(_) => return Ok(fixed),
            Err(_) => {}
        }
    }
    
    // Strategy 4: Try parsing the cleaned text directly one more time
    // Sometimes the issue is just whitespace or minor formatting
    let final_attempt = cleaned.trim();
    if let Ok(_) = serde_json::from_str::<serde_json::Value>(final_attempt) {
        return Ok(final_attempt.to_string());
    }
    
    // Strategy 5: If we found JSON boundaries, try the extracted portion even if validation failed
    // (Sometimes the validation is too strict but the JSON is actually valid)
    if let (Some(start_idx), Some(end_idx)) = (best_start, best_end) {
        if end_idx > start_idx && end_idx <= text.len() {
            let candidate = text[start_idx..end_idx].trim();
            if let Ok(_) = serde_json::from_str::<serde_json::Value>(candidate) {
                return Ok(candidate.to_string());
            }
        }
    }
    
    // Strategy 6: Last resort - try to parse with more aggressive cleaning
    let aggressive_clean = cleaned
        .lines()
        .filter(|line| !line.trim().is_empty() || line.trim().starts_with('{') || line.trim().starts_with('}'))
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string();
    
    if let Ok(_) = serde_json::from_str::<serde_json::Value>(&aggressive_clean) {
        return Ok(aggressive_clean);
    }
    
    // Last resort: Try one more time with the raw trimmed text, ignoring validation
    // Sometimes the validation is too strict
    let raw_trimmed = text.trim();
    if raw_trimmed.starts_with('{') {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_trimmed) {
            tracing::debug!("Last resort: Successfully parsed raw trimmed text");
            return Ok(raw_trimmed.to_string());
        }
        
        // Try finding JSON boundaries one more time with simpler logic
        if let Some(start) = raw_trimmed.find('{') {
            let mut brace_count = 0;
            let mut in_string = false;
            let mut escape_next = false;
            
            for (i, ch) in raw_trimmed[start..].char_indices() {
                if escape_next {
                    escape_next = false;
                    continue;
                }
                
                match ch {
                    '\\' if in_string => escape_next = true,
                    '"' => in_string = !in_string,
                    '{' if !in_string => brace_count += 1,
                    '}' if !in_string => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            let candidate = &raw_trimmed[start..start+i+1];
                            if let Ok(_) = serde_json::from_str::<serde_json::Value>(candidate) {
                                tracing::debug!("Last resort: Successfully extracted JSON with simple boundary finding");
                                return Ok(candidate.to_string());
                            }
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Final error with full context
    anyhow::bail!(
        "Failed to extract valid JSON from response. Text length: {}, Preview (first 500 chars): {}, Preview (last 200 chars): {}",
        text.len(),
        text.chars().take(500).collect::<String>(),
        text.chars().rev().take(200).collect::<String>().chars().rev().collect::<String>()
    )
}

