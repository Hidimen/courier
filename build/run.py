from typing import List
import build.generate as generate
import logging
import os
from pathlib import Path
import subprocess
import sys
import shutil

# Paths
ROOT = Path(os.getcwd())
TARGET = ROOT / "target"
OUTPUT = ROOT / "output"

def run_project(mode: str, args: List[str]):
  logging.basicConfig(
    level=logging.INFO,
    format="%(levelname)s - %(message)s"
  )

  logging.info("Running project...")
  list = generate.list_all_modules()
  generate.generate_files(list)

  subprocess.run(["cargo", "run", *args])

def main():
  args = sys.argv[1:]
  run_project("debug", args)

if __name__ == "__main__":
  main()
