use std::env::temp_dir;
use anyhow::{bail, Context, ensure};
use crate::builder::Recipe;
use crate::types::Distribution;

pub struct CMakeToolchain
{
  pub configure_command: String,
  pub configure_additional_arguments: Vec<String>,
}

impl CMakeToolchain
{
  pub fn new(config: &crate::core::Config) -> Self
  {
    Self
    {
      configure_command: config.toolchain.cmake.configure_command.clone(),
      configure_additional_arguments: config.toolchain.cmake.configure_additional_definitions
        .iter()
        .map(|x| format!("{}={}", x.0, x.1))
        .collect()
    }
  }

  pub fn build_from_recipe(&self, recipe: &Recipe, source_directory: &str, distribution: Distribution) -> anyhow::Result<()>
  {
    let target_temp = temp_dir()
      .join(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos().to_string())
      .join(crate::names::TARGET_FOLDER);
    let mut command = std::process::Command::new(self.configure_command.clone());
    command
      .arg("-S")
      .arg(source_directory)
      .arg("-B")
      .arg(target_temp);
    for x in &self.configure_additional_arguments {
      command.arg("-D").arg(x);
    }
    command.arg(format!("-DCMAKE_PREFIX_PATH={}", crate::names::DEPENDENCIES_FOLDER));

    // todo: rewrite
    match distribution {
      Distribution::Static => {
        command.arg("-DBUILD_SHARED_LIBS=OFF");
        let definitions = recipe
          .static_toolchain
          .as_ref()
          .context("cmake toolchain was requested to build as static but no static toolchain was provided")?
          .toolchain
          .cmake
          .as_ref()
          .context("cmake toolchain was requested to build package but recipe is not configured for cmake")?
          .definitions
          .as_ref();
        if let Some(definitions) = definitions {
          for x in definitions {
            command.arg(format!("-D{}={}", x.0.to_uppercase(), x.1.to_uppercase()));
          }
        }
      },
      Distribution::Shared => {
        command.arg("-DBUILD_SHARED_LIBS=ON");
        let definitions = recipe
          .shared_toolchain
          .as_ref()
          .context("cmake toolchain was requested to build as shared but no shared toolchain was provided")?
          .toolchain
          .cmake
          .as_ref()
          .context("cmake toolchain was requested to build package but recipe is not configured for cmake")?
          .definitions
          .as_ref();
        if let Some(definitions) = definitions {
          for x in definitions {
            command.arg(format!("-D{}={}", x.0.to_uppercase(), x.1.to_uppercase()));
          }
        }
      },
      _ => bail!("unsupported distribution: {:?}", distribution)
    }

    println!("cmake command: {:?}", command);

    command
      .output()
      .context("failed to execute cmake command")?;

    ensure!(command.status()?.success(), "failed to execute cmake command");

    println!("cmake command output: {}", String::from_utf8_lossy(&command.output().unwrap().stdout));

    Ok(())
  }
}