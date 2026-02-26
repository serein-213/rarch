#[cfg(feature = "ai-native")]
use anyhow::Result;
#[cfg(feature = "ai-native")]
use std::path::PathBuf;
#[cfg(feature = "ai-native")]
use std::sync::Mutex;
#[cfg(feature = "ai-native")]
use candle_transformers::models::quantized_qwen2::ModelWeights;
#[cfg(feature = "ai-native")]
use candle_core::quantized::gguf_file;
#[cfg(feature = "ai-native")]
use candle_core::{Device, Tensor};
#[cfg(feature = "ai-native")]
use tokenizers::Tokenizer;
#[cfg(feature = "ai-native")]
use candle_transformers::generation::LogitsProcessor;

#[cfg(feature = "ai-native")]
pub struct NativeModel {
    model: Mutex<ModelWeights>,
    tokenizer: Tokenizer,
}

#[cfg(feature = "ai-native")]
impl NativeModel {
    pub fn new(model_path: Option<String>, tokenizer_path: Option<String>) -> Result<Self> {
        let (m_path, t_path) = if let (Some(m), Some(t)) = (model_path, tokenizer_path) {
            (PathBuf::from(m), PathBuf::from(t))
        } else {
            Self::download_default_model()?
        };

        println!("Loading native model from: {:?}", m_path);
        println!("Loading tokenizer from: {:?}", t_path);

        let mut file = std::fs::File::open(&m_path)?;
        let content = gguf_file::Content::read(&mut file)?;
        let model = ModelWeights::from_gguf(content, &mut file, &Device::Cpu)?;
        
        let tokenizer = Tokenizer::from_file(&t_path).map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self {
            model: Mutex::new(model),
            tokenizer,
        })
    }

    fn download_default_model() -> Result<(PathBuf, PathBuf)> {
        println!("No local model specified. Downloading default Qwen2-0.5B-Instruct from HuggingFace...");
        use hf_hub::api::sync::Api;
        let api = Api::new()?;
        
        // Fetch tokenizer
        let tokenizer_repo = api.model("Qwen/Qwen2-0.5B-Instruct".to_string());
        let t_path = tokenizer_repo.get("tokenizer.json")?;
        
        // Fetch GGUF model
        let model_repo = api.model("Qwen/Qwen2-0.5B-Instruct-GGUF".to_string());
        let m_path = model_repo.get("qwen2-0_5b-instruct-q4_k_m.gguf")?;
        
        Ok((m_path, t_path))
    }

    pub fn generate(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let mut model = self.model.lock().unwrap();
        
        // Format prompt for Qwen2 Instruct
        let formatted_prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            system_prompt, user_prompt
        );
        // println!("--- PROMPT ---\n{}\n--- END PROMPT ---", formatted_prompt);
        
        let tokens = self.tokenizer.encode(formatted_prompt, true).map_err(|e| anyhow::anyhow!(e))?;
        let mut tokens = tokens.get_ids().to_vec();
        
        let mut generated_tokens = Vec::new();
        let mut lp = LogitsProcessor::new(299792458, None, None); // ArgMax for deterministic output
        
        let mut index_pos = 0;
        
        for _ in 0..200 { // max tokens
            let input = Tensor::new(&tokens[index_pos..], &Device::Cpu)?.unsqueeze(0)?;
            let logits = model.forward(&input, index_pos)?;
            let logits = logits.squeeze(0)?;
            let logits = if logits.rank() == 2 {
                logits.get(logits.dim(0)? - 1)?
            } else {
                logits
            };
            
            let next_token = lp.sample(&logits)?;
            tokens.push(next_token);
            generated_tokens.push(next_token);
            
            index_pos += input.dim(1)?;
            
            // Qwen2 EOS tokens
            if next_token == 151645 || next_token == 151643 {
                break;
            }
        }
        
        let generated = self.tokenizer.decode(&generated_tokens, true).map_err(|e| anyhow::anyhow!(e))?;
        Ok(generated.trim().to_string())
    }
}
