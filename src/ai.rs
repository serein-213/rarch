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
        let system_msg = "You are a strict binary classifier. Your task is to determine if a file matches a specific criteria based on its name and content snippet.
Rules:
1. You MUST answer with ONLY 'YES' or 'NO'.
2. Do not provide any explanations or additional text.
3. If the content snippet is insufficient but the filename strongly suggests a match, answer 'YES'.
4. If you are completely unsure, answer 'NO'.

Examples:
Criteria: 'Invoice or Receipt'
File: 'scan_01.jpg'
Content: 'Bill to John Doe... Total: $50.00'
Answer: YES

Criteria: 'Invoice or Receipt'
File: 'vacation_photo.png'
Content: 'Sunny beach with palm trees...'
Answer: NO";
        
        let mut user_msg = format!("Criteria: '{}'\nFile: '{}'", prompt, file_name);
        if let Some(snippet) = content_snippet {
            user_msg.push_str(&format!("\nContent: '{}...'", snippet.chars().take(300).collect::<String>()));
        }
        user_msg.push_str("\nAnswer:");

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
                Message { role: "user".into(), content: user_msg },
            ],
        };

        let url = format!("{}/chat/completions", self.api_base);
        match self.client.post(url).json(&body).send() {
            Ok(res) => {
                if let Ok(data) = res.json::<ChatResponse>() {
                    let ans = data.choices.first().map(|c| c.message.content.trim().to_uppercase());
                    if let Some(a) = ans {
                        if a.contains("YES") {
                            if let Some(ref cb) = reporter { cb(&format!("[AI-Match] YES ({})", file_name)); } else { println!("  [AI] Result: YES"); }
                            return true;
                        } else if let Some(ref cb) = reporter { cb(&format!("[AI-Match] NO ({})", file_name)); } else { println!("  [AI] Result: NO ({})", a); }
                    } else if let Some(ref cb) = reporter { cb("[AI-Match] Invalid response"); } else { println!("  [AI] Result: INVALID RESPONSE"); }
                } else if let Some(ref cb) = reporter { cb("[AI-Match] Parse error"); } else { println!("  [AI] Result: JSON PARSE ERROR"); }
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
        let system_msg = "You are a professional file naming assistant. Your task is to generate a highly descriptive, concise, and safe file name based on the provided context.
Rules:
1. Output ONLY the generated file name. No explanations, no quotes, no markdown.
2. DO NOT include the file extension (e.g., output 'annual_report' instead of 'annual_report.pdf').
3. Use ONLY alphanumeric characters, underscores (_), or hyphens (-). Replace spaces with underscores.
4. Keep it concise but descriptive (ideally 2-5 words).
5. If the language of the content is Chinese, you may output a Chinese filename (but still follow rule 3 for separators).";
        
        let mut user_msg = format!("Naming Goal: '{}'\nCurrent Name: '{}'", prompt, file_name);
        if let Some(snippet) = content_snippet {
            user_msg.push_str(&format!("\nFile Content Snippet: '{}...'", snippet.chars().take(300).collect::<String>()));
        }
        user_msg.push_str("\nSuggested Name:");

        let body = ChatRequest {
            model: self.model.clone(),
            temperature: 0.3,
            max_tokens: 30,
            messages: vec![
                Message { role: "system".into(), content: system_msg.into() },
                Message { role: "user".into(), content: user_msg },
            ],
        };

        let url = format!("{}/chat/completions", self.api_base);
        match self.client.post(url).json(&body).send() {
            Ok(res) => {
                if let Ok(data) = res.json::<ChatResponse>() {
                    let ans = data.choices.first().map(|c| c.message.content.trim().to_string());
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

    /// Extract structured information from the file content based on a prompt.
    /// Returns the extracted string, or None if extraction fails.
    #[cfg(feature = "ai")]
    pub fn extract_info<F>(&self, file_name: &str, content_snippet: Option<&str>, prompt: &str, reporter: Option<F>) -> Option<String> 
    where F: Fn(&str)
    {
        if let Some(ref cb) = reporter {
            cb(&format!("[AI-Extract] Analyzing '{}'...", file_name));
        }
        let system_msg = "You are a precise data extraction assistant. Your task is to extract specific information from the provided file context based on the user's goal.
Rules:
1. Output ONLY the extracted value. No explanations, no quotes, no markdown.
2. If the information cannot be found or inferred, output exactly 'UNKNOWN'.
3. Keep the output as concise as possible.";
        
        let mut user_msg = format!("Extraction Goal: '{}'\nFile Name: '{}'", prompt, file_name);
        if let Some(snippet) = content_snippet {
            user_msg.push_str(&format!("\nFile Content Snippet: '{}...'", snippet.chars().take(400).collect::<String>()));
        }
        user_msg.push_str("\nExtracted Value:");

        let body = ChatRequest {
            model: self.model.clone(),
            temperature: 0.1,
            max_tokens: 50,
            messages: vec![
                Message { role: "system".into(), content: system_msg.into() },
                Message { role: "user".into(), content: user_msg },
            ],
        };

        let url = format!("{}/chat/completions", self.api_base);
        match self.client.post(url).json(&body).send() {
            Ok(res) => {
                if let Ok(data) = res.json::<ChatResponse>() {
                    let ans = data.choices.first().map(|c| c.message.content.trim().to_string());
                    if let Some(a) = ans {
                        if a == "UNKNOWN" || a.is_empty() {
                            if let Some(ref cb) = reporter { cb(&format!("[AI-Extract] Failed to extract for '{}'", file_name)); }
                            return None;
                        }
                        
                        // Clean up the extracted value
                        let clean_val = a.replace("\n", " ").trim().to_string();
                        if let Some(ref cb) = reporter { cb(&format!("[AI-Extract] Done: '{}' -> '{}'", file_name, clean_val)); }
                        return Some(clean_val);
                    }
                }
            }
            Err(e) => {
                if let Some(ref cb) = reporter { cb(&format!("[AI-Extract] Error: {}", e)); }
            }
        }
        None
    }

    #[cfg(not(feature = "ai"))]
    pub fn extract_info<F>(&self, file_name: &str, _content_snippet: Option<&str>, _prompt: &str, reporter: Option<F>) -> Option<String> 
    where F: Fn(&str)
    {
        if let Some(ref cb) = reporter {
            cb(&format!("[AI-Extract] Skipped (AI feature disabled) for '{}'", file_name));
        }
        None
    }
}
