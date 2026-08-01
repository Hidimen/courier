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

def build_project(mode: str, args: List[str]):
  logging.basicConfig(
    level=logging.INFO,
    format="%(levelname)s - %(message)s"
  )

  logging.info("Building project...")
  list = generate.list_all_modules()
  generate.generate_files(list)

  if mode == "release":
    subprocess.run(["cargo", "build", "--release" , *args])
  else:
    subprocess.run(["cargo", "build", *args])

  if not os.path.exists(OUTPUT):
    os.makedirs(OUTPUT)
  if sys.platform == "win32":
    EXECUTE = TARGET / mode / "courier.exe"
    TO = OUTPUT / "courier.exe"
    shutil.copy2(EXECUTE, TO)
  else:
    EXECUTE = TARGET / mode / "courier"
    TO = OUTPUT / "courier"
    shutil.copy2(EXECUTE, TO)

def main():
  mode = sys.argv[1]
  args = sys.argv[2:]
  build_project(mode, args)

if __name__ == "__main__":
  main()
