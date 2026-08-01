from dataclasses import dataclass
from enum import Enum
import tomllib
from pathlib import Path
from typing import List
import re
import os
import logging
from git_cloner import GitCloner

# PATH
ROOT = Path(os.getcwd())
BUILTINS_PATH = ROOT / "builtins"
BUILTIN_MANIFEST_PATH = ROOT / "builtin" / "Cargo.toml"
WAYBILL_MANIFEST_PATH = ROOT / "waybill" / "Cargo.toml"
COURIER_MANIFEST_PATH = ROOT / "courier" / "Cargo.toml"
BUILTIN_GENERATED_PATH = ROOT / "builtin" / "src" / "generated.rs"
BOOTLOADER_COMMAND_GENERATED_PATH = ROOT / "waybill" / "src" / "commands" / "generated" / "bootloader.rs"

class BuiltinModuleKind(Enum):
  BOOTLOADER = 0
  ENHANCEMENT = 1
  FULL = 2

@dataclass
class BuiltinModule:
  name: str
  kind: BuiltinModuleKind
  entry: str
  path: Path

def list_all_modules() -> List[BuiltinModule]:
  list: List[BuiltinModule] = []

  for item in Path.iterdir(BUILTINS_PATH):
    if item.is_dir():
      toml_path = item / "Cargo.toml"
      with open(toml_path, "rb") as f:
        try:
          manifest = tomllib.load(f)
          if "package" in manifest and "metadata" in manifest["package"] and "carrier" in manifest["package"]["metadata"]:
            metadata = manifest["package"]["metadata"]["carrier"]
            if metadata["kind"] == "bootloader":
              list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.BOOTLOADER, metadata["entry"], item))
            elif metadata["kind"] == "enhancement":
              list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.ENHANCEMENT, "", item))
            elif metadata["kind"] == "full":
              list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.FULL, metadata["entry"], item))
            else:
              logging.error("Unknown builtin kind in {}".format(toml_path))
          else:
            logging.error("{} is not carrier-managed crate".format(item))
        except tomllib.TOMLDecodeError as e:
          logging.error("Failed to decode {}: {}".format(toml_path, e))

    elif item.is_file():
      with open(item, "rb") as f:
        placeholder = tomllib.load(f)
        if "dir" in placeholder:
          path = Path(placeholder["dir"])
          with open(path / "Cargo.toml", "rb") as fd:
            manifest = tomllib.load(fd)
            if "package" in manifest and "metadata" in manifest["package"] and "carrier" in manifest["package"]["metadata"]:
              metadata = manifest["package"]["metadata"]["carrier"]
              if metadata["kind"] == "bootloader":
                list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.BOOTLOADER, metadata["entry"], path))
              elif metadata["kind"] == "enhancement":
                list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.ENHANCEMENT, "", path))
              elif metadata["kind"] == "full":
                list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.FULL, metadata["entry"], path))
              else:
                logging.error("Unknown builtin kind in {}".format(path / "Cargo.toml"))
            else:
              logging.error("{} is not carrier-managed crate".format(item))
        elif "git" in placeholder:
          cache_dir_path = ROOT / "build_cache"
          if not cache_dir_path.exists():
            os.makedirs(cache_dir_path)

          clone = GitCloner(cache_dir_path, placeholder["git"])
          path = clone.clone()

          with open(path / "Cargo.toml", "rb") as fd:
            manifest = tomllib.load(fd)
            if "package" in manifest and "metadata" in manifest["package"] and "carrier" in manifest["package"]["metadata"]:
              metadata = manifest["package"]["metadata"]["carrier"]
              if metadata["kind"] == "bootloader":
                list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.BOOTLOADER, metadata["entry"], path))
              elif metadata["kind"] == "enhancement":
                list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.ENHANCEMENT, "", path))
              elif metadata["kind"] == "full":
                list.append(BuiltinModule(manifest["package"]["name"], BuiltinModuleKind.FULL, metadata["entry"], path))
              else:
                logging.error("Unknown builtin kind in {}".format(path / "Cargo.toml"))
            else:
              logging.error("{} is not carrier-managed crate".format(item))
          pass

    else:
      continue
  return list

def generate_files(list: List[BuiltinModule]) -> None:
  builtin_manifest = ""
  waybill_manifest = ""
  courier_manifest = ""
  builtin_generated = ""
  bootloader_generated = ""

  with open(BUILTIN_MANIFEST_PATH, "r") as b:
    builtin_manifest = b.read()

  with open(WAYBILL_MANIFEST_PATH, "r") as w:
    waybill_manifest = w.read()

  with open(COURIER_MANIFEST_PATH, "r") as c:
    courier_manifest = c.read()

  builtin_dep: List[str] = []
  builtin_feat: List[str] = []
  waybill_feat: List[str] = []
  courier_feat: List[str] = []
  builtin_generated_cache: List[str] = []
  bootloader_generated_enum: List[str] = []
  bootloader_generated_impl: List[str] = []

  for module in list:
    if module.kind == BuiltinModuleKind.BOOTLOADER:
      builtin_dep.append('{} = {{ path = "{}", optional = true }}'.format(module.name, module.path.as_posix()))
      builtin_feat.append('{} = ["dep:{}"]'.format(module.name, module.name))
      waybill_feat.append('{} = ["builtin/{}"]'.format(module.name, module.name))
      courier_feat.append('{} = ["builtin/{}", "waybill/{}"]'.format(module.name, module.name, module.name))
      builtin_generated_cache.append(
        '''#[cfg(feature = "{}")]
#[doc(hidden)]
pub use {};'''.format(module.name, module.name))
      bootloader_generated_enum.append(
        '''  #[cfg(feature = "{}")]
  {},'''.format(module.name, to_pascal_case(module.name)))
      bootloader_generated_impl.append(
        '''      #[cfg(feature = "{}")]
      BootloaderList::{} => builtin::{}().await,'''.format(module.name, to_pascal_case(module.name), f"{module.name}::{module.entry}"))
    elif module.kind == BuiltinModuleKind.ENHANCEMENT:
      builtin_dep.append('{} = {{ path = "{}", optional = true }}'.format(module.name, module.path.as_posix()))
      builtin_feat.append('{} = ["dep:{}"]'.format(module.name, module.name))
      waybill_feat.append('{} = ["builtin/{}"]'.format(module.name, module.name))
      courier_feat.append('{} = ["builtin/{}"]'.format(module.name, module.name))
      builtin_generated_cache.append(
        '''#[cfg(feature = "{}")]
#[doc(inline)]
pub use {};'''.format(module.name, module.name))
    elif module.kind == BuiltinModuleKind.FULL:
      builtin_dep.append('{} = {{ path = "{}", optional = true }}'.format(module.name, module.path.as_posix()))
      builtin_feat.append('{} = ["dep:{}"]'.format(module.name, module.name))
      waybill_feat.append('{} = ["builtin/{}"]'.format(module.name, module.name))
      courier_feat.append('{} = ["builtin/{}", "waybill/{}"]'.format(module.name, module.name, module.name))
      builtin_generated_cache.append(
        '''#[cfg(feature = "{}")]
#[doc(inline)]
pub use {};'''.format(module.name, module.name))
      bootloader_generated_enum.append(
        '''  #[cfg(feature = "{}")]
  {},'''.format(module.name, to_pascal_case(module.name)))
      bootloader_generated_impl.append(
        '''      #[cfg(feature = "{}")]
      BootloaderList::{} => builtin::{}().await,'''.format(module.name, to_pascal_case(module.name), f"{module.name}::{module.entry}"))

  # Replace placeholders
  builtin_manifest = replace_with_marker("FEAT", replace_with_marker("DEP", builtin_manifest, "\n".join(builtin_dep)), "\n".join(builtin_feat))
  waybill_manifest = replace_with_marker("FEAT", waybill_manifest, "\n".join(waybill_feat))
  courier_manifest = replace_with_marker("FEAT", courier_manifest, "\n".join(courier_feat))
  builtin_generated = r'''// @generated by build/generator.py --- DO NOT EDIT
{}
'''.format("\n".join(builtin_generated_cache))
  bootloader_generated = r'''// @generated by build/generator.py --- DO NOT EDIT
#[derive(ValueEnum, Clone)]
enum BootloaderList {{
{}
}}

impl StartArgs {{
  pub async fn execute(&self) {{
    match self.bootloader {{
{}
    }}
  }}
}}
'''.format("\n".join(bootloader_generated_enum), "\n".join(bootloader_generated_impl))

  # Write files
  with open(BUILTIN_MANIFEST_PATH, "w") as f:
    logging.info("WRITE {}".format(BUILTIN_MANIFEST_PATH))
    f.write(builtin_manifest)

  with open(WAYBILL_MANIFEST_PATH, "w") as f:
    logging.info("WRITE {}".format(WAYBILL_MANIFEST_PATH))
    f.write(waybill_manifest)

  with open(COURIER_MANIFEST_PATH, "w") as f:
    logging.info("WRITE {}".format(COURIER_MANIFEST_PATH))
    f.write(courier_manifest)

  with open(BUILTIN_GENERATED_PATH, "w") as f:
    logging.info("WRITE {}".format(BUILTIN_GENERATED_PATH))
    f.write(builtin_generated)

  with open(BOOTLOADER_COMMAND_GENERATED_PATH, "w") as f:
    logging.info("WRITE {}".format(BOOTLOADER_COMMAND_GENERATED_PATH))
    f.write(bootloader_generated)

  return

def clean_files() -> None:
  builtin_manifest = ""
  waybill_manifest = ""
  courier_manifest = ""
  builtin_generated = '''// @generated by build/generator.py --- DO NOT EDIT

// NO BUILTIN MODULES HERE
'''
  bootloader_generated = '''// @generated by build/generator.py --- DO NOT EDIT
#[derive(ValueEnum, Clone)]
enum BootloaderList {
  // NO BUILTIN MODULES HERE
}

impl StartArgs {
  pub async fn execute(&self) {
    match self.bootloader {
      // NO BUILTIN MODULES HERE
    }
  }
}
'''

  with open(BUILTIN_MANIFEST_PATH, "r") as b:
    builtin_manifest = b.read()

  with open(WAYBILL_MANIFEST_PATH, "r") as w:
    waybill_manifest = w.read()

  with open(COURIER_MANIFEST_PATH, "r") as c:
    courier_manifest = c.read()

  builtin_manifest = replace_with_marker("FEAT", replace_with_marker("DEP", builtin_manifest, "# --- NO BUILTIN MODULES --- #"), "# --- NO BUILTIN MODULES --- #")
  waybill_manifest = replace_with_marker("FEAT", waybill_manifest, "# --- NO BUILTIN MODULES --- #")
  courier_manifest = replace_with_marker("FEAT", courier_manifest, "# --- NO BUILTIN MODULES --- #")

  with open(BUILTIN_MANIFEST_PATH, "w") as f:
    logging.info("CLEAN {}".format(BUILTIN_MANIFEST_PATH))
    f.write(builtin_manifest)
  
  with open(WAYBILL_MANIFEST_PATH, "w") as f:
    logging.info("CLEAN {}".format(WAYBILL_MANIFEST_PATH))
    f.write(waybill_manifest)
  
  with open(COURIER_MANIFEST_PATH, "w") as f:
    logging.info("CLEAN {}".format(COURIER_MANIFEST_PATH))
    f.write(courier_manifest)
  
  with open(BUILTIN_GENERATED_PATH, "w") as f:
    logging.info("CLEAN {}".format(BUILTIN_GENERATED_PATH))
    f.write(builtin_generated)
  
  with open(BOOTLOADER_COMMAND_GENERATED_PATH, "w") as f:
    logging.info("CLEAN {}".format(BOOTLOADER_COMMAND_GENERATED_PATH))
    f.write(bootloader_generated)

  return

def replace_with_marker(marker: str, content: str, new: str) -> str:
  pattern = re.compile(f"# AUTOGEN {marker}: START #.*?# AUTOGEN {marker}: END #", re.DOTALL)
  replacement = (
    f"# AUTOGEN {marker}: START #\n"
    f"# --- Generated by build/generator.py - DO NOT EDIT--- #\n"
    f"{new}\n"
    f"# AUTOGEN {marker}: END #"
  )

  return pattern.sub(replacement, content)

def to_pascal_case(name: str) -> str:
  return "".join(w.capitalize() for w in name.split("_"))

def main():
  logging.basicConfig(
    level=logging.INFO,
    format="%(levelname)s - %(message)s"
  )

  list = list_all_modules()
  generate_files(list)

if __name__ == "__main__":
  main()
