"extract metadata from runner scripts"
from os import access, R_OK, X_OK
from pathlib import Path
import re

__all__ = ["languages"]

known_keys = {"name", "image", "version", "url"}
languages = {}
for path in Path("/usr/local/share/ATO/runners").iterdir():
# for path in (Path(__file__).parents[1] / "runners").iterdir():
    if not access(path, R_OK | X_OK):    
        # not executable
        continue
    metadata = {}
    with path.open("r") as f:
        for line in f:
            if match := re.match(r"^#:(?P<key>\w+): (?P<value>.*)", line):
                if match["key"] in metadata:
                    raise Exception(f"duplicate metadata item {key} in runner {path.name}")
                elif match["key"] not in known_keys:
                    raise Exception(f"unknown metadata item {key} in runner {path.name}")
                metadata[match["key"]] = match["value"]
    languages[path.name] = metadata
