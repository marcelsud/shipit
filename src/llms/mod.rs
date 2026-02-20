use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct Topic {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<&'static str>,
}

const INDEX_CONTENT: &str = include_str!("../../docs/llms/index.md");

const TOPICS: &[Topic] = &[
    Topic {
        slug: "quickstart",
        title: "Quickstart",
        description: "Getting started with init, setup, and first deploy",
        content: Some(include_str!("../../docs/llms/quickstart.md")),
    },
    Topic {
        slug: "configuration",
        title: "Configuration",
        description: "shipit.toml format and all available options",
        content: Some(include_str!("../../docs/llms/configuration.md")),
    },
    Topic {
        slug: "deploy",
        title: "Deploy Pipeline",
        description: "The 12-step deploy process with zero-downtime",
        content: Some(include_str!("../../docs/llms/deploy.md")),
    },
    Topic {
        slug: "setup",
        title: "Server Setup",
        description: "What shipit setup installs on target VMs",
        content: Some(include_str!("../../docs/llms/setup.md")),
    },
    Topic {
        slug: "secrets",
        title: "Secrets Management",
        description: "Age-encrypted secrets workflow",
        content: Some(include_str!("../../docs/llms/secrets.md")),
    },
    Topic {
        slug: "rollback",
        title: "Rollback",
        description: "How to rollback to a previous release",
        content: Some(include_str!("../../docs/llms/rollback.md")),
    },
    Topic {
        slug: "health-check",
        title: "Health Check",
        description: "Docker HEALTHCHECK configuration",
        content: Some(include_str!("../../docs/llms/health-check.md")),
    },
    Topic {
        slug: "local",
        title: "Local Development",
        description: "Multipass VMs for local testing",
        content: Some(include_str!("../../docs/llms/local.md")),
    },
    Topic {
        slug: "traefik",
        title: "Traefik Integration",
        description: "Docker network, labels, TLS",
        content: Some(include_str!("../../docs/llms/traefik.md")),
    },
    Topic {
        slug: "accessories",
        title: "Accessories",
        description: "Postgres, Redis, and other auxiliary services",
        content: Some(include_str!("../../docs/llms/accessories.md")),
    },
];

pub fn index() -> &'static str {
    INDEX_CONTENT
}

pub fn get(slug: &str) -> Result<&'static Topic> {
    TOPICS
        .iter()
        .find(|t| t.slug == slug)
        .ok_or_else(|| anyhow::anyhow!("Topic '{}' not found. Run `shipit llms index` to see available topics.", slug))
}

pub fn full() -> String {
    let mut out = String::from(INDEX_CONTENT);
    for topic in TOPICS {
        out.push_str("\n---\n\n");
        out.push_str(topic.content.unwrap_or_default());
    }
    out
}

#[derive(Serialize)]
struct IndexJson {
    name: &'static str,
    summary: &'static str,
    topics: Vec<TopicSummary>,
}

#[derive(Serialize)]
struct TopicSummary {
    slug: &'static str,
    title: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
struct TopicJson {
    slug: &'static str,
    title: &'static str,
    content: &'static str,
}

#[derive(Serialize)]
struct FullJson {
    name: &'static str,
    summary: &'static str,
    topics: Vec<TopicContentJson>,
}

#[derive(Serialize)]
struct TopicContentJson {
    slug: &'static str,
    title: &'static str,
    content: &'static str,
}

pub fn index_json() -> Result<String> {
    let data = IndexJson {
        name: "Shipit",
        summary: "CLI tool for deploying apps to VMs via Docker Compose + Traefik. Zero-downtime, encrypted secrets, rollback support.",
        topics: TOPICS
            .iter()
            .map(|t| TopicSummary {
                slug: t.slug,
                title: t.title,
                description: t.description,
            })
            .collect(),
    };
    Ok(serde_json::to_string_pretty(&data)?)
}

pub fn get_json(slug: &str) -> Result<String> {
    let topic = get(slug)?;
    let data = TopicJson {
        slug: topic.slug,
        title: topic.title,
        content: topic.content.unwrap_or_default(),
    };
    Ok(serde_json::to_string_pretty(&data)?)
}

pub fn full_json() -> Result<String> {
    let data = FullJson {
        name: "Shipit",
        summary: "CLI tool for deploying apps to VMs via Docker Compose + Traefik. Zero-downtime, encrypted secrets, rollback support.",
        topics: TOPICS
            .iter()
            .map(|t| TopicContentJson {
                slug: t.slug,
                title: t.title,
                content: t.content.unwrap_or_default(),
            })
            .collect(),
    };
    Ok(serde_json::to_string_pretty(&data)?)
}
