import generator
import logging
from pathlib import Path
import subprocess
import os
import shutil

ROOT = Path(os.getcwd())
OUTPUT = ROOT / "output"
CACHE_DIR = ROOT / "build_cache"

def main():
  logging.basicConfig(
    level=logging.INFO,
    format="%(levelname)s - %(message)s"
  )

  try:
    generator.clean_files()
    logging.info("Generated files are cleaned completely")

    subprocess.run(["cargo", "clean"])
    logging.info("Successfully cleaned cache")

    shutil.rmtree(OUTPUT)
    logging.info("Successfully deleted output folder")

    shutil.rmtree(CACHE_DIR)
    logging.info("Successfully deleted cache folder")

  except Exception as e:
    pass
  
  return

if __name__ == "__main__":
  main()