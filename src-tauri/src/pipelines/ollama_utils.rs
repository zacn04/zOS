/// Utility functions for cleaning and parsing Ollama responses

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
pub fn extract_json(text: &str) -> anyhow::Result<String> {
    // Single-pass optimization: find the best JSON candidate in one scan
    let mut json_start = None;
    let mut json_end = None;
    let mut in_code_block = false;
    let mut brace_count = 0;
    let mut best_start = None;
    let mut best_end = None;
    
    let bytes = text.as_bytes();
    let mut i = 0;
    
    while i < bytes.len() {
        // Check for code block markers
        if i + 3 <= bytes.len() && &bytes[i..i+3] == b"```" {
            if in_code_block {
                // End of code block
                if json_start.is_some() {
                    if brace_count == 0 && json_end.is_none() {
                        json_end = Some(i);
                    }
                }
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                if i + 7 <= bytes.len() && &bytes[i..i+7] == b"```json" {
                    i += 7;
                    // Skip whitespace
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    json_start = Some(i);
                    brace_count = 0;
                    continue;
                } else {
                    i += 3;
                    // Skip whitespace
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == b'{' {
                        json_start = Some(i);
                        brace_count = 0;
                    }
                    continue;
                }
            }
            i += 3;
            continue;
        }
        
        // Track braces for JSON boundaries
        if let Some(start) = json_start {
            match bytes[i] {
                b'{' => {
                    if brace_count == 0 {
                        best_start = Some(start);
                    }
                    brace_count += 1;
                }
                b'}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        best_end = Some(i + 1);
                        break; // Found complete JSON object
                    }
                }
                _ => {}
            }
        } else if bytes[i] == b'{' {
            // Found JSON start outside code block
            json_start = Some(i);
            brace_count = 1;
            best_start = Some(i);
        }
        
        i += 1;
    }
    
    // Use best match found
    let json_str = if let (Some(start_idx), Some(end_idx)) = (best_start, best_end) {
        text[start_idx..end_idx].to_string()
    } else if let Some(start_idx) = best_start {
        // Incomplete JSON, try to find end
        let mut brace_count = 0;
        let mut end = start_idx;
        for (i, ch) in text[start_idx..].char_indices() {
            match ch {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end = start_idx + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if brace_count == 0 {
            text[start_idx..end].to_string()
        } else {
            text.trim().to_string()
        }
    } else {
        text.trim().to_string()
    };
    
    // Remove trailing commas
    let cleaned = remove_trailing_commas(&json_str);
    
    // Strategy 1: Validate structure
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
    if let Some(start) = cleaned.find('{') {
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
    
    // Strategy 4: Last resort - return cleaned string and let caller handle
    anyhow::bail!(
        "Failed to extract valid JSON from response. Text length: {}, Preview: {}",
        text.len(),
        text.chars().take(200).collect::<String>()
    )
}

