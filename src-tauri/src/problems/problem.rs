use serde::{Deserialize, Serialize, Deserializer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub id: String,
    pub topic: String,
    pub difficulty: f32,
    pub statement: String,
    #[serde(deserialize_with = "deserialize_solution_sketch")]
    pub solution_sketch: String,
}

// Custom deserializer that handles both string and structured formats
fn deserialize_solution_sketch<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct SolutionSketchVisitor;

    impl<'de> Visitor<'de> for SolutionSketchVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or structured object/array")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut parts = Vec::new();
            while let Some(item) = seq.next_element::<serde_json::Value>()? {
                if let Some(obj) = item.as_object() {
                    // Handle object with step1, step2, etc.
                    let mut steps: Vec<String> = obj
                        .iter()
                        .filter_map(|(k, v)| {
                            if let Some(s) = v.as_str() {
                                Some(format!("{}: {}", k, s))
                            } else {
                                None
                            }
                        })
                        .collect();
                    steps.sort(); // Sort by key for consistent output
                    parts.extend(steps);
                } else if let Some(s) = item.as_str() {
                    parts.push(s.to_string());
                } else {
                    parts.push(item.to_string());
                }
            }
            Ok(parts.join("\n"))
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            let obj: serde_json::Map<String, serde_json::Value> = 
                Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
            
            let mut parts: Vec<String> = obj
                .iter()
                .filter_map(|(k, v)| {
                    if let Some(s) = v.as_str() {
                        Some(format!("{}: {}", k, s))
                    } else {
                        None
                    }
                })
                .collect();
            parts.sort();
            Ok(parts.join("\n"))
        }
    }

    deserializer.deserialize_any(SolutionSketchVisitor)
}

impl Problem {
    pub fn load_all() -> Result<Vec<Problem>, Box<dyn std::error::Error>> {
        // Build list of possible paths to check
        let mut possible_paths = Vec::new();
        
        // 1. FIRST: Try app data directory (where problems should be after initialization)
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = std::env::var_os("HOME") {
                let mut dir = std::path::PathBuf::from(home);
                dir.push("Library/Application Support/com.zacnwo.zos");
                dir.push("problems");
                possible_paths.push(dir);
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                let mut dir = std::path::PathBuf::from(appdata);
                dir.push("com.zacnwo.zos");
                dir.push("problems");
                possible_paths.push(dir);
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            if let Some(home) = std::env::var_os("HOME") {
                let mut dir = std::path::PathBuf::from(home);
                dir.push(".local/share/com.zacnwo.zos");
                dir.push("problems");
                possible_paths.push(dir);
            }
        }
        
        // 2. Try relative to current working directory (development)
        possible_paths.push(std::path::PathBuf::from("problems"));
        possible_paths.push(std::path::PathBuf::from("../problems"));
        possible_paths.push(std::path::PathBuf::from("./problems"));
        
        // 3. Try relative to executable (for built apps - check Resources first)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // For macOS app bundles, Resources is at: MyApp.app/Contents/Resources
                possible_paths.push(exe_dir.join("../../Resources/problems"));
                possible_paths.push(exe_dir.join("../../../Resources/problems"));
                possible_paths.push(exe_dir.join("problems"));
                possible_paths.push(exe_dir.join("../problems"));
                possible_paths.push(exe_dir.join("../../problems"));
                possible_paths.push(exe_dir.join("../../../problems"));
            }
        }

        // Find the first existing problems directory
        let mut problems_dir = None;
        for path in &possible_paths {
            if path.exists() && path.is_dir() {
                problems_dir = Some(path.clone());
                break;
            }
        }

        let problems_dir = match problems_dir {
            Some(dir) => dir,
            None => {
                // If no problems directory found, return empty (will trigger problem generation)
                eprintln!("Warning: No problems directory found. Searched: {:?}", possible_paths);
                return Ok(Vec::new());
            },
        };

        let mut problems = Vec::new();

        // Load from main problems directory
        let problems_dir_clone = problems_dir.clone();
        for entry in std::fs::read_dir(&problems_dir_clone)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = std::fs::read_to_string(&path)?;
                let problem: Problem = serde_json::from_str(&content)?;
                problems.push(problem);
            }
        }

        // Also load from autogen subdirectory if it exists
        let autogen_dir = problems_dir.join("autogen");
        if autogen_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&autogen_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(problem) = serde_json::from_str::<Problem>(&content) {
                                problems.push(problem);
                            }
                        }
                    }
                }
            }
        }

        Ok(problems)
    }
    
    /// Initialize problems directory by copying from source if needed
    pub fn initialize_problems_dir() {
        let app_data_problems = get_app_data_problems_dir();
        
        // Check if app data directory has problems
        let has_problems = app_data_problems.exists() && 
            std::fs::read_dir(&app_data_problems)
                .map(|d| d.count())
                .unwrap_or(0) > 0;
        
        if !has_problems {
            eprintln!("App data problems directory empty or missing: {:?}", app_data_problems);
            
            // Try to find source problems directory
            let mut source_paths = Vec::new();
            
            // Check current working directory
            source_paths.push(std::path::PathBuf::from("problems"));
            source_paths.push(std::path::PathBuf::from("../problems"));
            source_paths.push(std::path::PathBuf::from("../../problems"));
            
            // Check relative to executable (for built apps)
            if let Ok(exe_path) = std::env::current_exe() {
                eprintln!("Executable path: {:?}", exe_path);
                if let Some(exe_dir) = exe_path.parent() {
                    eprintln!("Executable directory: {:?}", exe_dir);
                    
                    // For macOS app bundles, the structure is:
                    // MyApp.app/Contents/MacOS/myapp (executable)
                    // We need to go to: MyApp.app/Contents/Resources/problems
                    source_paths.push(exe_dir.join("problems"));
                    source_paths.push(exe_dir.join("../problems"));
                    source_paths.push(exe_dir.join("../../problems"));
                    source_paths.push(exe_dir.join("../../../problems"));
                    source_paths.push(exe_dir.join("../../Resources/problems"));
                    source_paths.push(exe_dir.join("../../../Resources/problems"));
                }
            }
            
            eprintln!("Searching for source problems in: {:?}", source_paths);
            
            for source_path in source_paths {
                if source_path.exists() && source_path.is_dir() {
                    eprintln!("Found source problems directory: {:?}", source_path);
                    
                    // Create app data directory
                    if let Some(parent) = app_data_problems.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!("Failed to create app data directory: {}", e);
                            continue;
                        }
                    }
                    
                    // Copy problems directory
                    match copy_dir_all(&source_path, &app_data_problems) {
                        Ok(_) => {
                            eprintln!("Successfully copied problems to: {:?}", app_data_problems);
                            return;
                        }
                        Err(e) => {
                            eprintln!("Failed to copy problems directory: {}", e);
                        }
                    }
                }
            }
            
            eprintln!("ERROR: Could not find source problems directory to copy!");
        } else {
            eprintln!("Problems directory already exists at: {:?}", app_data_problems);
        }
    }
}

fn get_app_data_problems_dir() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = std::path::PathBuf::from(home);
            dir.push("Library/Application Support/com.zacnwo.zos");
            dir.push("problems");
            return dir;
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut dir = std::path::PathBuf::from(appdata);
            dir.push("com.zacnwo.zos");
            dir.push("problems");
            return dir;
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = std::path::PathBuf::from(home);
            dir.push(".local/share/com.zacnwo.zos");
            dir.push("problems");
            return dir;
        }
    }
    
    // Fallback
    std::path::PathBuf::from("problems")
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dst)?;
    
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        
        if path.is_dir() {
            copy_dir_all(&path, &dst_path)?;
        } else {
            std::fs::copy(&path, &dst_path)?;
        }
    }
    
    Ok(())
}

