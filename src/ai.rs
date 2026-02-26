use std::path::Path;

#[cfg(feature = "ai")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ai")]
use reqwest::blocking::Client;
#[cfg(feature = "ai")]
use std::time::Duration;

#[cfg(feature = "ai")]
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: i32,
}

#[cfg(feature = "ai")]
#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[cfg(feature = "ai")]
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[cfg(feature = "ai")]
#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[allow(dead_code)]
pub struct AiOracle {
    #[cfg(feature = "ai")]
    client: Client,
    api_base: String,
    model: String,
}

impl AiOracle {
    #[cfg(feature = "ai")]
    pub fn new(api_base: String, model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            api_base,
            model,
        }
    }

    #[cfg(not(feature = "ai"))]
    pub fn new(api_base: String, model: String) -> Self {
        Self {
            api_base,
            model,
        }
    }

    /// Ask the local LLM if a file matches a certain prompt.
    /// Returns true if the LLM responds with something like "YES".
    #[cfg(feature = "ai")]
    pub fn matches_prompt<F>(&self, file_name: &str, content_snippet: Option<&str>, prompt: &str, reporter: Option<F>) -> bool 
    where F: Fn(&str)
    {
        let system_msg = "Extract a binary classification (YES/NO) for the file. 
Example 1: Crit: 'Invoice', File: 'scan_01.jpg', Content: 'Bill to John...' -> YES
Example 2: Crit: 'Invoice', File: 'vacation.png', Content: 'Sunny beach...' -> NO
Only answer YES or NO.";
        
        let mut user_msg = format!("Crit: '{}', File: '{}'", prompt, file_name);
        if let Some(snippet) = content_snippet {
            user_msg.push_str(&format!(", Content: '{}...'", snippet.chars().take(200).collect::<String>()));
        }
        user_msg.push_str(" -> ");

        if let Some(ref cb) = reporter {
            cb(&format!("[AI-Match] Querying for '{}'...", file_name));
        } else {
            println!("  [AI] Query: '{}' for file '{}'...", prompt, file_name);
        }
        
        let body = ChatRequest {
            model: self.model.clone(),
            temperature: 0.1, // Keep it low for consistency
            max_tokens: 10,
            messages: vec![
                Message { role: "system".into(), content: system_msg.into() },
                Message { role: "user".into(), content: user_msg.into() },
            ],
        };

        let url = format!("{}/chat/completions", self.api_base);
        match self.client.post(url).json(&body).send() {
            Ok(res) => {
                if let Ok(data) = res.json::<ChatResponse>() {
                    let ans = data.choices.get(0).map(|c| c.message.content.trim().to_uppercase());
                    if let Some(a) = ans {
                        if a.contains("YES") {
                            if let Some(ref cb) = reporter { cb(&format!("[AI-Match] YES ({})", file_name)); } else { println!("  [AI] Result: YES"); }
                            return true;
                        } else {
                            if let Some(ref cb) = reporter { cb(&format!("[AI-Match] NO ({})", file_name)); } else { println!("  [AI] Result: NO ({})", a); }
                        }
                    } else {
                        if let Some(ref cb) = reporter { cb("[AI-Match] Invalid response"); } else { println!("  [AI] Result: INVALID RESPONSE"); }
                    }
                } else {
                    if let Some(ref cb) = reporter { cb("[AI-Match] Parse error"); } else { println!("  [AI] Result: JSON PARSE ERROR"); }
                }
                false
            }
            Err(e) => {
                if let Some(ref cb) = reporter { cb(&format!("[AI-Match] Error: {}", e)); } else { println!("  [AI] Error: {}", e); }
                false
            } // Fallback to false if API fails
        }
    }

    #[cfg(not(feature = "ai"))]
    pub fn matches_prompt<F>(&self, file_name: &str, _content_snippet: Option<&str>, _prompt: &str, reporter: Option<F>) -> bool 
    where F: Fn(&str)
    {
        if let Some(ref cb) = reporter {
            cb(&format!("[AI-Match] Skipped (AI feature disabled) for '{}'", file_name));
        }
        false
    }

    /// Suggest a new name for the file based on its content and a description of the goal.
    /// Returns the suggested name without extension.
    #[cfg(feature = "ai")]
    pub fn suggest_name<F>(&self, file_name: &str, content_snippet: Option<&str>, prompt: &str, reporter: Option<F>) -> String 
    where F: Fn(&str)
    {
        if let Some(ref cb) = reporter {
            cb(&format!("[AI-Rename] Analyzing '{}'...", file_name));
        }
        let system_msg = "You are a professional file naming assistant. 
Analyze the file content and context provided. 
Generate a short, descriptive, and safe file name (slug) in Chinese or English. 
Rules: 
1. No spaces (use underscores or hyphens). 
2. NO extension in the output. 
3. Answer ONLY with the generated name.";
        
        let mut user_msg = format!("Goal: '{}', Current Name: '{}'", prompt, file_name);
        if let Some(snippet) = content_snippet {
            user_msg.push_str(&format!(", Context: '{}...'", snippet.chars().take(200).collect::<String>()));
        }
        user_msg.push_str(" -> Suggest Name:");

        let body = ChatRequest {
            model: self.model.clone(),
            temperature: 0.3,
            max_tokens: 30,
            messages: vec![
                Message { role: "system".into(), content: system_msg.into() },
                Message { role: "user".into(), content: user_msg.into() },
            ],
        };

        let url = format!("{}/chat/completions", self.api_base);
        match self.client.post(url).json(&body).send() {
            Ok(res) => {
                if let Ok(data) = res.json::<ChatResponse>() {
                    let ans = data.choices.get(0).map(|c| c.message.content.trim().to_string());
                    if let Some(a) = ans {
                        // Clean up the name a bit
                        let mut name = a.replace(" ", "_");
                        
                        // If model included an extension (e.g. .txt), remove it
                        if let Some(pos) = name.rfind('.') {
                            name = name[..pos].to_string();
                        }
                        
                        let clean_name = name.replace(".", "_")
                                          .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                                          .to_string();
                        if !clean_name.is_empty() {
                            if let Some(ref cb) = reporter { cb(&format!("[AI-Rename] Done: '{}' -> '{}'", file_name, clean_name)); } else { println!("  [AI-Rename] Suggested: '{}'", clean_name); }
                            return clean_name;
                        }
                    }
                }
            }
            Err(e) => {
                if let Some(ref cb) = reporter { cb(&format!("[AI-Rename] Error: {}", e)); } else { println!("  [AI-Rename] Error: {}", e); }
            }
        }
        // Fallback to original stem if AI fails
        Path::new(file_name).file_stem().unwrap_or_default().to_string_lossy().to_string()
    }

    #[cfg(not(feature = "ai"))]
    pub fn suggest_name<F>(&self, file_name: &str, _content_snippet: Option<&str>, _prompt: &str, reporter: Option<F>) -> String 
    where F: Fn(&str)
    {
        if let Some(ref cb) = reporter {
            cb(&format!("[AI-Rename] Skipped (AI feature disabled) for '{}'", file_name));
        }
        Path::new(file_name).file_stem().unwrap_or_default().to_string_lossy().to_string()
    }
}
