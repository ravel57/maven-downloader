mod dependencies;
mod dependency;
mod dependency_management;
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
use std::error::Error;
use std::fs;
use std::path::Path;

type ManagedVersions = HashMap<(String, String), String>;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: maven-downloader <path-to-pom.xml>");
        std::process::exit(1);
    }
    let path = &args[1];

    let mut managed_versions: ManagedVersions = HashMap::new();
    process_file(path, &mut managed_versions).await.expect("");
}

#[async_recursion]
async fn process_file(
    path: &str,
    managed_versions: &mut ManagedVersions,
) -> Result<(), Box<dyn Error>> {
    let xml = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Не удалось открыть файл {}: {}", path, e);
            // просто пропускаем этот pom и идём дальше
            return Ok(());
        }
    };

    let project: Project = quick_xml::de::from_str(&xml)?;

    let mut props = props_to_map(project.properties.clone());
    if let Some(p) = project.properties.clone() {
        props.extend(
            p.values
                .into_iter()
                .filter_map(|(k, v)| v.into_string().map(|s| (k, s))),
        );
    }
    if let Some(dm) = &project.dependency_management {
        if let Some(deps) = &dm.dependencies {
            for d in &deps.dependency {
                if let Some(raw_ver) = d.version.as_ref().and_then(|v| v.as_string()) {
                    // подставляем свойства
                    let ver = if raw_ver.starts_with("${") {
                        let key = raw_ver.trim_matches(|c: char| c == '$' || c == '{' || c == '}');

                        if key == "project.version" {
                            project
                                .version
                                .clone()
                                .or_else(|| {
                                    project.parent.as_ref().and_then(|p| {
                                        p.version.as_ref().and_then(|v| v.as_string())
                                    })
                                })
                                .unwrap_or(raw_ver.clone())
                        } else {
                            props.get(key).cloned().unwrap_or(raw_ver.clone())
                        }
                    } else {
                        raw_ver
                    };

                    managed_versions.insert((d.group_id.clone(), d.artifact_id.clone()), ver);
                }
            }
        }
    }
    let mut trace = vec![];
    if let Some(parent) = &project.parent {
        // parent тоже обрабатываем с картой версий
        process_artifact(&project, parent, &mut props, &trace, managed_versions).await?;
    }
    if let Some(dependencies) = &project.dependencies {
        for dep in &dependencies.dependency {
            let new_trace = trace.clone();
            match process_artifact(&project, dep, &mut props, &new_trace, managed_versions).await {
                Ok(_) => {}
                Err(_) => {
                    for node in &trace {
                        eprintln!("\t-> {node}");
                    }
                }
            }
        }
    }

    Ok(())
}

async fn process_artifact(
    project: &Project,
    dep: &Dependency,
    props: &mut HashMap<String, String>,
    trace: &Vec<String>,
    managed_versions: &mut ManagedVersions,
) -> Result<(), Box<dyn Error>> {
    let mut trace = trace.clone();
    trace.push(format!("{:?}:{}", project.group_id, project.artifact_id));
    let local_repository_path = if let Some(mut path) = dirs::home_dir() {
        path.push(".m2");
        path.push("repository");
        path.to_string_lossy().into_owned()
    } else {
        panic!("Не удалось найти домашнюю папку");
    };
    let maven_repository_path = "https://repo.maven.apache.org/maven2";

    // 1. пробуем взять версию прямо из <dependency>
    let mut version: Option<String> = dep.version.as_ref().and_then(|v| match v {
        TextOrNode::Text(s) => Some(s.clone()),
        TextOrNode::Node { text } => text.clone(),
    });

    // 2. если версии нет — ищем в собранной карте dependencyManagement (текущего и родительских pom'ов)
    if version.is_none() {
        if let Some(v) = managed_versions.get(&(dep.group_id.clone(), dep.artifact_id.clone())) {
            version = Some(v.clone());
        }
    }

    let version = match version {
        Some(v) => v,
        None => {
            let ped_stack = format!(
                "{}.{}",
                project.group_id.clone().unwrap_or_default(),
                project.artifact_id
            );
            eprintln!(
                "Не смог найти версию у зависимости {}:{}; Trace:\n\t{}",
                &dep.group_id, &dep.artifact_id, ped_stack
            );
            // можно оставить Err, если хочешь видеть фейл явно:
            // return Err(Box::from("".to_string()));
            return Ok(()); // либо тихо пропускать
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
        if let Err(e) = download_artifact_file(&local_pom_path, &url).await {
            eprintln!("Не удалось скачать pom {}: {}", url, e);
            // нет смысла продолжать разбирать этот pom
            return Ok(());
        }
    }
    if Path::new(&local_pom_path).exists() {
        if let Err(e) = process_file(local_pom_path.as_str(), managed_versions).await {
            eprintln!("Не удалось обработать pom {}: {}", local_pom_path, e);
        }
    }
    Ok(())
}

async fn download_artifact_file(path: &str, url: &str) -> Result<(), Box<dyn Error>> {
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
