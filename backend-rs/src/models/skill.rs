//! Skill model — personas with system prompt, tools, and parameters.

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub enabled_tools: Option<Vec<String>>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub format_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCreate {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub enabled_tools: Option<Vec<String>>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub format_tag: String,
}

fn default_icon() -> String {
    "\u{1F916}".to_string() // 🤖
}
fn default_temperature() -> f64 {
    0.7
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_color() -> String {
    "#6366f1".to_string()
}

/// Generate a slug ID from a name.
pub fn slugify(text: &str) -> String {
    let re = Regex::new(r"[^\w\s-]").unwrap();
    let lowered = text.to_lowercase();
    let trimmed = lowered.trim().to_string();
    let cleaned = re.replace_all(&trimmed, "");
    let re2 = Regex::new(r"[\s_]+").unwrap();
    let slug = re2.replace_all(&cleaned, "-");
    let result = if slug.len() > 64 { &slug[..64] } else { &slug };
    if result.is_empty() {
        "imported-skill".to_string()
    } else {
        result.to_string()
    }
}

/// The 5 built-in default skills.
pub const DEFAULT_SKILLS_COUNT: usize = 5;

pub fn default_skills() -> [Skill; DEFAULT_SKILLS_COUNT] {
    [
        Skill {
            id: "default".to_string(),
            name: "Assistant g\u{00e9}n\u{00e9}ral".to_string(),
            description: "Assistant polyvalent avec acc\u{00e8}s \u{00e0} tous les outils".to_string(),
            icon: "\u{1F916}".to_string(),
            system_prompt: "Tu es OllamaStudio, un assistant d\u{00e9}veloppeur expert. \
                Tu as acc\u{00e8}s \u{00e0} des outils pour lire/\u{00e9}crire des fichiers, ex\u{00e9}cuter des commandes shell, \
                faire des recherches dans le code et les documents import\u{00e9}s. \
                R\u{00e9}ponds toujours en fran\u{00e7}ais sauf si l'utilisateur utilise une autre langue. \
                Sois pr\u{00e9}cis, concis et propose du code de qualit\u{00e9} production.".to_string(),
            enabled_tools: None,
            temperature: 0.7,
            max_tokens: 4096,
            color: "#6366f1".to_string(),
            category: String::new(),
            format_tag: String::new(),
        },
        Skill {
            id: "code-review".to_string(),
            name: "Code Review".to_string(),
            description: "Expert en revue de code, s\u{00e9}curit\u{00e9} et bonnes pratiques".to_string(),
            icon: "\u{1F50D}".to_string(),
            system_prompt: "Tu es un expert en revue de code senior. Analyse le code fourni selon ces crit\u{00e8}res : \
                1) S\u{00e9}curit\u{00e9} (OWASP, injections, secrets expos\u{00e9}s) \
                2) Performance et complexit\u{00e9} algorithmique \
                3) Maintenabilit\u{00e9} et respect des principes SOLID \
                4) Couverture de tests \
                Fournis des commentaires structur\u{00e9}s avec exemples de correction.".to_string(),
            enabled_tools: Some(vec![
                "read_file".to_string(),
                "grep_files".to_string(),
                "list_files".to_string(),
            ]),
            temperature: 0.3,
            max_tokens: 8192,
            color: "#f59e0b".to_string(),
            category: String::new(),
            format_tag: String::new(),
        },
        Skill {
            id: "devops".to_string(),
            name: "DevOps & Infrastructure".to_string(),
            description: "Expert Docker, CI/CD, Linux, configuration serveur".to_string(),
            icon: "\u{1F3D7}\u{FE0F}".to_string(),
            system_prompt: "Tu es un ing\u{00e9}nieur DevOps senior sp\u{00e9}cialis\u{00e9} sur Linux (Debian/Fedora), Docker, \
                et les pipelines CI/CD. Tu aides \u{00e0} configurer des environnements, r\u{00e9}diger des Dockerfiles \
                optimis\u{00e9}s et des scripts d'automatisation. Respecte les bonnes pratiques de s\u{00e9}curit\u{00e9} \
                syst\u{00e8}me (ANSSI, CIS Benchmarks) quand applicable.".to_string(),
            enabled_tools: Some(vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "run_command".to_string(),
                "list_files".to_string(),
                "grep_files".to_string(),
                "git_status".to_string(),
            ]),
            temperature: 0.5,
            max_tokens: 4096,
            color: "#10b981".to_string(),
            category: String::new(),
            format_tag: String::new(),
        },
        Skill {
            id: "rust-expert".to_string(),
            name: "Expert Rust".to_string(),
            description: "D\u{00e9}veloppeur Rust avanc\u{00e9} \u{2014} ownership, lifetimes, async, performances".to_string(),
            icon: "\u{1F980}".to_string(),
            system_prompt: "Tu es un expert Rust de niveau avanc\u{00e9}. Tu ma\u{00ee}trises le borrow checker, les lifetimes, \
                les traits, async/await avec Tokio, les FFI et les optimisations de performances. \
                Tu proposes du code idiomatique Rust en suivant les conventions de la communaut\u{00e9}. \
                Tu expliques les concepts complexes avec des exemples concrets.".to_string(),
            enabled_tools: Some(vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "run_command".to_string(),
                "grep_files".to_string(),
                "git_status".to_string(),
            ]),
            temperature: 0.4,
            max_tokens: 8192,
            color: "#ef4444".to_string(),
            category: String::new(),
            format_tag: String::new(),
        },
        Skill {
            id: "security-audit".to_string(),
            name: "Audit S\u{00e9}curit\u{00e9}".to_string(),
            description: "Analyse de s\u{00e9}curit\u{00e9}, conformit\u{00e9} ANSSI/NIS2, hardening".to_string(),
            icon: "\u{1F6E1}\u{FE0F}".to_string(),
            system_prompt: "Tu es un auditeur s\u{00e9}curit\u{00e9} expert (ANSSI, NIS2, ISO 27001). \
                Tu analyses les configurations, les Dockerfiles, les scripts et le code source \
                pour identifier les vuln\u{00e9}rabilit\u{00e9}s. Tu fournis des recommandations de hardening \
                conformes aux r\u{00e9}f\u{00e9}rentiels ANSSI et aux benchmarks CIS. \
                Structure tes rapports : Risque critique / \u{00c9}lev\u{00e9} / Moyen / Faible.".to_string(),
            enabled_tools: Some(vec![
                "read_file".to_string(),
                "grep_files".to_string(),
                "list_files".to_string(),
                "search_documents".to_string(),
            ]),
            temperature: 0.2,
            max_tokens: 8192,
            color: "#8b5cf6".to_string(),
            category: String::new(),
            format_tag: String::new(),
        },
    ]
}
