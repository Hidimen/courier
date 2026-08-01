from dataclasses import dataclass
from pathlib import Path
import shutil
import git

@dataclass
class GitCloner:
  path: Path
  url: str

  def clone(self) -> Path:
    project_name = self.url.split('/')[-1].replace(".git", "")
    target_path = self.path / project_name

    if target_path.exists():
      shutil.rmtree(target_path)

    git.Repo.clone_from(self.url, target_path)

    return target_path