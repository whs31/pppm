use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use colored::Colorize;
use log::{debug, info, trace, warn};
use walkdir::WalkDir;
use crate::args::Args;
use crate::artifactory::Artifactory;
use crate::registry;
use crate::registry::entry::{RegistryEntry, RegistryEntryRaw};
use crate::resolver::Dependency;
use crate::utils::Config;

pub struct Registry
{
  pub packages: Vec<RegistryEntry>,
  config: Rc<RefCell<Config>>,
  artifactory: Rc<Artifactory>,
  registry_path: String,
  args: Rc<Args>
}

impl Registry
{
  pub fn new(config: Rc<RefCell<Config>>, artifactory: Rc<Artifactory>, path: &str, args: Rc<Args>) -> Self
  {
    Self
    {
      packages: vec![],
      config,
      artifactory,
      registry_path: String::from(path),
      args
    }
  }

  pub fn sync(&mut self, reclone: bool) -> anyhow::Result<()>
  {
    info!("syncing with remote repository");
    debug!("syncing into cache ({})", &self.registry_path.dimmed());
    std::fs::create_dir_all(Path::new(&self.registry_path).parent().unwrap())?;

    if !reclone {
      warn!("lazy sync is enabled. updating remote registry will not be performed unless cached registry is broken.");
    }
    if reclone || !std::path::Path::new(&self.registry_path).exists() {
      registry::git::clone_repository(
        &self.config.borrow().remotes.registry_url,
        &self.registry_path,
        "main", // todo: branch
        self.args.ci_git_username.clone(),
        self.args.ci_git_token.clone()
      )?;
    }

    self.sync_aql(false)?; // todo
    self.fetch_local_cache()?;

    info!("sync completed");
    Ok(())
  }

  pub fn sync_aql(&mut self, lazy: bool) -> anyhow::Result<&mut Self>
  {
    info!("syncing with remote repository {}", "via aql".green().bold());
    debug!("syncing into cache ({})", &self.registry_path.dimmed());
    std::fs::create_dir_all(Path::new(&self.registry_path).parent().unwrap())?;

    if lazy {
      warn!("lazy sync is enabled. updating remote registry will not be performed unless cached registry is broken.");
      if Path::new(&self.registry_path).exists() {
        warn!("older registry found. skipping aql sync");
        return Ok(self);
      }
    }

    let raw = self.artifactory.query(r#"items.find({"repo": "poppy-cxx-repo", "name": {"$match": "*"}}).sort({"$desc": ["created"]})"#)?;
    let json: serde_json::Value = serde_json::from_str(raw.as_str())?;
    let mut packages = Vec::new();
    // todo

    Ok(self)
  }

  pub fn contains(&self, dependency: &Dependency) -> bool
  {
    self.packages.iter().any(|x| {
      x.name == dependency.name
        && x.versions.contains_key(&dependency.version)
        && x.versions[&dependency.version].contains_key(&dependency.distribution)
        && x.versions[&dependency.version][&dependency.distribution].contains(&dependency.arch)
    })
  }

  fn fetch_local_cache(&mut self) -> anyhow::Result<()>
  {
    debug!("fetching registry cache");
    let yamls = self.collect_yamls()?
      .into_iter()
      .map(|y| Self::parse_yaml(&y))
      .collect::<Result<Vec<_>, _>>()?;
    self.packages = yamls;
    for entry in &self.packages
    {
      debug!("found package: {}", &entry.pretty_format());
    }
    Ok(())
  }

  fn collect_yamls(&self) -> anyhow::Result<Vec<String>>
  {
    trace!("collecting yamls");
    let mut yamls = vec![];
    for entry in WalkDir::new(self.registry_path.as_str())
      .into_iter()
      .filter_map(|e| e.ok())
      .filter(|e| e.file_type().is_file()
        && e.path().extension().is_some()
        && e.path().extension().unwrap() == "yml"
        && !e.path().file_name().unwrap().to_str().unwrap().starts_with(".")
      )
    {
      let content = std::fs::read_to_string(entry.path())?;
      yamls.push(content);
    }
    trace!("found {} yamls!", yamls.len());
    Ok(yamls)
  }

  fn parse_yaml(yaml: &str) -> anyhow::Result<RegistryEntry>
  {
    let entry: RegistryEntryRaw = serde_yaml::from_str(yaml)?;
    Ok(entry.try_into()?)
  }
}
