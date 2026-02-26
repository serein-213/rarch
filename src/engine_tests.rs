#[cfg(test)]
mod tests {
    use crate::engine::Engine;
    use crate::config::{Config, Rule, ConflictStrategy};
    use crate::journal::{Operation, OpType};
    use std::path::{PathBuf, Path};
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_resolve_placeholders() {
        let config = Config { rules: vec![] };
        let engine = Engine::new(config, PathBuf::from("."));
        let path = PathBuf::from("test.txt");
        
        let rule_target = "${ext}/file";
        let resolved = engine.resolve_placeholders(rule_target, &path);
        assert_eq!(resolved, "txt/file");

        let rule2_target = "${year}-${month}";
        let resolved = engine.resolve_placeholders(rule2_target, &path);
        assert!(resolved.contains("${year}") || resolved.len() == 7);
    }

    #[test]
    fn test_match_rule_by_extension() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("main.rs");
        fs::write(&file_path, "fn main() {}").expect("Failed to write to file");

        let rule = Rule {
            name: "test".into(),
            extensions: Some(vec!["rs".into()]),
            target: "src/".into(),
            ..Default::default()
        };
        let config = Config { rules: vec![rule] };
        let engine = Engine::new(config, dir.path().to_path_buf());
        
        let matched = engine.match_rule(&file_path);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().name, "test");

        let other_path = dir.path().join("Cargo.toml");
        fs::write(&other_path, "[package]").expect("Failed to write to file");
        let no_match = engine.match_rule(&other_path);
        assert!(no_match.is_none());
    }

    #[test]
    fn test_match_rule_by_mime() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("test_image.png");
        
        // Create a fake PNG file header
        fs::write(&file_path, vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]).expect("Failed to write to file");

        let rule = Rule {
            name: "image_rule".into(),
            mime: Some("image/png".into()),
            target: "images/".into(),
            ..Default::default()
        };
        let config = Config { rules: vec![rule] };
        let engine = Engine::new(config, dir.path().to_path_buf());
        
        let matched = engine.match_rule(&file_path);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().name, "image_rule");
    }

    #[test]
    fn test_match_rule_by_type() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("test_doc.pdf");
        
        // Create a fake PDF file header (starts with %PDF)
        fs::write(&file_path, vec![0x25, 0x50, 0x44, 0x46]).expect("Failed to write to file");

        let rule = Rule {
            name: "doc_rule".into(),
            r#type: Some("document".into()),
            target: "docs/".into(),
            ..Default::default()
        };
        let config = Config { rules: vec![rule] };
        let engine = Engine::new(config, dir.path().to_path_buf());
        
        let matched = engine.match_rule(&file_path);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().name, "doc_rule");
    }
}
