use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use anyhow::{Context, ensure};
use colored::Colorize;
use log::{debug, info, trace, warn};
use crate::args::Args;
use crate::artifactory::Artifactory;
use crate::registry::entry::RegistryEntry;
use crate::resolver::Dependency;
use crate::utils::Config;
use crate::utils::helper_types::Version;

pub struct Registry
{
  pub packages: Vec<RegistryEntry>,
  #[allow(dead_code)] config: Rc<RefCell<Config>>,
  artifactory: Rc<Artifactory>,
  registry_path: String,
  #[allow(dead_code)] args: Rc<Args>
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

  pub fn sync_aql(&mut self, lazy: bool) -> anyhow::Result<&mut Self>
  {
    info!("syncing with remote repository {}", "via aql".green().bold());
    debug!("syncing into cache ({})", &self.registry_path.dimmed());
    std::fs::create_dir_all(Path::new(&self.registry_path).parent().unwrap())?;

    if lazy {
      warn!("lazy sync is enabled. updating remote registry will not be performed unless cached registry is broken.");
      if Path::new(&self.registry_path).exists() {
        warn!("older registry found. skipping aql sync");
        let cached_path = Path::new(&self.registry_path).join("registry.yml");
        return Ok(self.sync_local(cached_path.to_str().unwrap())?);
      }
    }

    let raw = self.artifactory.query(r#"items.find({"repo": "poppy-cxx-repo", "name": {"$match": "*"}}).sort({"$desc": ["created"]})"#)?;
    let parsed: crate::artifactory::query::PackageQueryResponse = serde_json::from_str(&raw)?;

    let mut packages: Vec<RegistryEntry> = Vec::new();
    for entry in parsed.results
    {
      let y = Dependency::from_package_name(entry.name.as_str())?;
      if packages
        .iter()
        .any(|x| x.name == y.name)
      {
        let reg_entry = packages
          .iter_mut()
          .find(|x| x.name == y.name)
          .context("weird things happened...")?;
        if reg_entry.versions
          .contains_key(&y.version)
        {
          if reg_entry.versions
            .get_mut(&y.version)
            .unwrap()
            .contains_key(&y.distribution)
          {
            if reg_entry.versions
              .get_mut(&y.version)
              .unwrap()
              .get_mut(&y.distribution)
              .unwrap()
              .contains(&y.arch)
            {
              trace!("duplicate package: {}/{}/{}@{}", y.name, y.arch, y.distribution, y.version);
              continue;
            }

            reg_entry.versions
              .get_mut(&y.version)
              .unwrap()
              .get_mut(&y.distribution)
              .unwrap()
              .push(y.arch);
          } else {
            reg_entry.versions
              .get_mut(&y.version)
              .unwrap()
              .insert(y.distribution, vec![y.arch]);
          }
        } else {
          reg_entry.versions
            .insert(y.version, HashMap::from([(y.distribution, vec![y.arch])]));
        }
      } else {
        let reg_entry = RegistryEntry {
          name: y.name,
          versions: HashMap::from([(y.version, HashMap::from([(y.distribution, vec![y.arch])]))]),
        };
        packages.push(reg_entry);
      }
    }

    let serialized = serde_yaml::to_string(&packages)?;
    std::fs::create_dir_all(Path::new(&self.registry_path))?;
    let cached_path = Path::new(&self.registry_path).join("registry.yml");
    if Path::new(&cached_path).exists() {
      trace!("removing old cache...");
      std::fs::remove_file(&cached_path)?;
    }
    std::fs::write(Path::new(&cached_path), serialized)?;
    debug!("wrote registry to cache ({})", &self.registry_path.dimmed());

    info!("online sync done (found {} packages)", packages.len());

    Ok(self.sync_local(cached_path.to_str().unwrap())?)
  }

  fn sync_local(&mut self, path: &str) -> anyhow::Result<&mut Self>
  {
    ensure!(Path::new(path).exists(), "non-existent registry cache file");

    debug!("loading registry cache from {}", path.dimmed());
    let deserialized: Vec<RegistryEntry> = serde_yaml::from_str(
      &std::fs::read_to_string(Path::new(path))?
    )?;

    debug!("loaded {} packages from cache", deserialized.len());

    self.packages = deserialized;

    for entry in &self.packages
    {
      debug!("found package: {}", &entry.pretty_format());
    }

    info!("offline sync done (found {} packages)", self.packages.len());

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

  pub fn latest_poppy_version(&self) -> anyhow::Result<Version>
  {
    let mut versions = self
      .packages
      .iter()
      .filter(|x| x.name == "poppy")
      .map(|x| x.versions.keys().collect::<Vec<_>>())
      .flatten()
      .collect::<Vec<_>>();

    versions.sort();
    versions.reverse();
    let latest = versions
      .first()
      .context("no poppy versions found (latest poppy version routine)")?
      .to_string();
    Ok(Version::try_from(latest)?)
  }
}
