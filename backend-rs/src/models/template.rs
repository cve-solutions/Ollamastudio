//! Template model — prompt snippets with variable placeholders.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_template_icon")]
    pub icon: String,
    pub content: String,
    #[serde(default)]
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCreate {
    pub id: String,
    pub name: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_template_icon")]
    pub icon: String,
    pub content: String,
    #[serde(default)]
    pub variables: Vec<String>,
}

fn default_category() -> String {
    "G\u{00e9}n\u{00e9}ral".to_string()
}
fn default_template_icon() -> String {
    "\u{1F4DD}".to_string() // 📝
}

pub const DEFAULT_TEMPLATES_COUNT: usize = 8;

pub fn default_templates() -> [Template; DEFAULT_TEMPLATES_COUNT] {
    [
        Template {
            id: "explain-code".to_string(),
            name: "Expliquer ce code".to_string(),
            category: "Code".to_string(),
            icon: "\u{1F4D6}".to_string(),
            content: "Explique ce code de mani\u{00e8}re d\u{00e9}taill\u{00e9}e, ligne par ligne si n\u{00e9}cessaire :\n\n```\n{{code}}\n```".to_string(),
            variables: vec!["code".to_string()],
        },
        Template {
            id: "refactor".to_string(),
            name: "Refactoriser".to_string(),
            category: "Code".to_string(),
            icon: "\u{267B}\u{FE0F}".to_string(),
            content: "Refactorise ce code en respectant les bonnes pratiques (SOLID, DRY, lisibilit\u{00e9}) :\n\n```\n{{code}}\n```\n\nContexte : {{context}}".to_string(),
            variables: vec!["code".to_string(), "context".to_string()],
        },
        Template {
            id: "write-tests".to_string(),
            name: "\u{00c9}crire des tests".to_string(),
            category: "Test".to_string(),
            icon: "\u{1F9EA}".to_string(),
            content: "\u{00c9}cris des tests unitaires complets pour ce code. Couvre les cas nominaux et les cas limites :\n\n```\n{{code}}\n```\n\nFramework de test : {{framework}}".to_string(),
            variables: vec!["code".to_string(), "framework".to_string()],
        },
        Template {
            id: "docker-optimize".to_string(),
            name: "Optimiser Dockerfile".to_string(),
            category: "DevOps".to_string(),
            icon: "\u{1F433}".to_string(),
            content: "Optimise ce Dockerfile (multi-stage build, layer caching, s\u{00e9}curit\u{00e9}, taille d'image) :\n\n```dockerfile\n{{dockerfile}}\n```".to_string(),
            variables: vec!["dockerfile".to_string()],
        },
        Template {
            id: "security-review".to_string(),
            name: "Revue de s\u{00e9}curit\u{00e9}".to_string(),
            category: "S\u{00e9}curit\u{00e9}".to_string(),
            icon: "\u{1F510}".to_string(),
            content: "Effectue une revue de s\u{00e9}curit\u{00e9} de ce code/configuration. Identifie les vuln\u{00e9}rabilit\u{00e9}s selon OWASP et les bonnes pratiques ANSSI :\n\n```\n{{content}}\n```".to_string(),
            variables: vec!["content".to_string()],
        },
        Template {
            id: "commit-message".to_string(),
            name: "Message de commit".to_string(),
            category: "Git".to_string(),
            icon: "\u{1F4DD}".to_string(),
            content: "G\u{00e9}n\u{00e8}re un message de commit conventionnel (Conventional Commits) pour ces changements :\n\n{{changes}}".to_string(),
            variables: vec!["changes".to_string()],
        },
        Template {
            id: "api-doc".to_string(),
            name: "Documenter une API".to_string(),
            category: "Documentation".to_string(),
            icon: "\u{1F4DA}".to_string(),
            content: "G\u{00e9}n\u{00e8}re la documentation OpenAPI/Swagger pour cet endpoint :\n\n```\n{{code}}\n```".to_string(),
            variables: vec!["code".to_string()],
        },
        Template {
            id: "debug-error".to_string(),
            name: "D\u{00e9}boguer une erreur".to_string(),
            category: "Debug".to_string(),
            icon: "\u{1F41B}".to_string(),
            content: "Analyse cette erreur et propose une solution :\n\nErreur :\n```\n{{error}}\n```\n\nCode concern\u{00e9} :\n```\n{{code}}\n```".to_string(),
            variables: vec!["error".to_string(), "code".to_string()],
        },
    ]
}
