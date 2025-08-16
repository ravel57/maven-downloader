mod dependencies;
mod dependency;
mod project;
mod properties;

use crate::dependency::Dependency;
use crate::dependency::TextOrNode;
use crate::properties::Properties;
use async_recursion::async_recursion;
use project::Project;
use regex::Regex;
use reqwest::StatusCode;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: maven-downloader <path-to-pom.xml>");
        std::process::exit(1);
    }
    let path = &args[1];

    process_file(path).await.expect("");
}

#[async_recursion]
async fn process_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let xml = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Не удалось открыть файл {}: {}", path, e));
    let project: Project = quick_xml::de::from_str(&xml)?;

    let mut props = props_to_map(project.properties.clone());
    if let Some(p) = project.properties.clone() {
        props.extend(
            p.values
                .into_iter()
                .filter_map(|(k, v)| v.into_string().map(|s| (k, s))),
        );
    }

    if let Some(parent) = &project.parent {
        process_artifact(&project, parent, &mut props).await?;
    }
    if let Some(dependencies) = &project.dependencies {
        for dep in &dependencies.dependency {
            process_artifact(&project, dep, &mut props).await?;
        }
    }
    Ok(())
}

async fn process_artifact(
    project: &Project,
    dep: &Dependency,
    props: &mut HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_repository_path = if let Some(mut path) = dirs::home_dir() {
        path.push(".m2");
        path.push("repository");
        path.to_string_lossy().into_owned()
    } else {
        panic!("Не удалось найти домашнюю папку");
    };
    let maven_repository_path = "https://repo.maven.apache.org/maven2";

    let version: Option<String> = dep.version.as_ref().and_then(|v| match v {
        TextOrNode::Text(s) => Some(s.clone()),
        TextOrNode::Node { text } => text.clone(),
    });
    let version = match version {
        Some(v) => v,
        None => {
            eprintln!(
                "Нет версии у зависимости {}:{} — пропускаю",
                dep.group_id, dep.artifact_id
            );
            return Ok(());
        }
    };

    let resolved_version: String = if version.starts_with("${") {
        let key: &str = version.trim_matches(|c: char| c == '$' || c == '{' || c == '}');
        if key == "project.version" {
            project
                .version
                .clone()
                .or_else(|| {
                    project
                        .parent
                        .as_ref()
                        .and_then(|p| p.version.as_ref().and_then(|v| v.as_string()))
                })
                .unwrap_or_else(|| version.clone())
        } else {
            props.get(key).cloned().unwrap_or_else(|| version.clone())
        }
    } else {
        version.clone()
    };

    let jar_path = format!(
        "{}/{}/{}/{}-{}.jar",
        &dep.group_id.replace(".", "/"),
        &dep.artifact_id,
        &resolved_version,
        &dep.artifact_id,
        &resolved_version,
    );
    let local_jar_path = format!("{local_repository_path}/{jar_path}");
    if !Path::new(&local_jar_path).exists() {
        let url = format!("{maven_repository_path}/{jar_path}");
        download_artifact_file(&local_jar_path, &url)
            .await
            .expect("ERROR downloading jar");
    }

    let pom_path = format!(
        "{}/{}/{}/{}-{}.pom",
        &dep.group_id.replace(".", "/"),
        &dep.artifact_id,
        &resolved_version,
        &dep.artifact_id,
        &resolved_version,
    );
    let local_pom_path = format!("{local_repository_path}/{pom_path}");
    if !Path::new(&local_pom_path).exists() {
        let url = format!("{maven_repository_path}/{pom_path}");
        download_artifact_file(&local_pom_path, &url)
            .await
            .expect("ERROR downloading pom");
        process_file(local_pom_path.as_str())
            .await
            .expect("ERROR processing file");
    }
    Ok(())
}

async fn download_artifact_file(path: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    match response.status() {
        StatusCode::OK => {
            println!("Downloading {}", path);
        }
        StatusCode::NOT_FOUND => {
            return Ok(());
        }
        other => {
            eprintln!("Ошибка: {} при скачивании {}", other, url);
            return Ok(());
        }
    }

    let bytes = response.bytes().await?;

    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, &bytes)?;

    Ok(())
}

fn resolve_placeholders(s: &str, props: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\$\{([^}]+)}").unwrap();
    re.replace_all(s, |caps: &regex::Captures| {
        let key = &caps[1];
        props.get(key).cloned().unwrap_or(caps[0].to_string())
    })
    .to_string()
}

fn props_to_map(p: Option<Properties>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(p) = p {
        for (k, v) in p.values {
            if let Some(s) = v.into_string() {
                out.insert(k, s);
            }
        }
    }
    out
}
