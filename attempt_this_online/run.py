import json
from hashlib import sha256
from os import getenv, mkdir
from pathlib import Path
from shutil import rmtree
from subprocess import run

import msgpack
from pydantic import BaseModel, conint, validator

from attempt_this_online import metadata


class Invocation(BaseModel):
    language: str
    code: bytes
    input: bytes
    arguments: list[bytes]
    options: list[bytes]
    timeout: conint(le=60, ge=1) = 60

    @validator("language")
    def validate_language(cls, value: str):
        if value not in metadata.languages:
            raise ValueError("no such language")
        else:
            return value

    @validator("arguments", "options", each_item=True)
    def validate_args(cls, arg: bytes):
        if 0 in arg:
            raise ValueError("null bytes not allowed")
        else:
            return arg


def execute_once(ip_hash: str, invocation_id: str, invocation: Invocation) -> dict:
    try:
        hashed_invocation_id = sha256(invocation_id.encode()).hexdigest()
        dir_i = Path("/run/ATO_i") / hashed_invocation_id
        mkdir(dir_i)
        with (dir_i / "code").open("wb") as f:
            f.write(invocation.code)
        with (dir_i / "input").open("wb") as f:
            f.write(invocation.input)

        with (dir_i / "arguments").open("wb") as f:
            f.write(b"".join(arg + b"\0" for arg in invocation.arguments))
        with (dir_i / "options").open("wb") as f:
            f.write(b"".join(opt + b"\0" for opt in invocation.options))

        run(
            ["sudo", "-u", "sandbox", "/usr/local/bin/ATO_sandbox", ip_hash, invocation_id, invocation.language, str(invocation.timeout)],
            env={"PATH": getenv("PATH")}
        )
        dir_o = Path("/run/ATO_o") / hashed_invocation_id
        with (dir_o / "stdout").open("rb") as f:
            stdout = f.read()
        with (dir_o / "stderr").open("rb") as f:
            stderr = f.read()
        with (dir_o / "status").open("r") as f:
            status = json.load(f)

        status["stdout"] = stdout
        status["stderr"] = stderr
    finally:
        rmtree(dir_i)
        run(["sudo", "-u", "sandbox", "/usr/local/bin/ATO_rm", invocation_id])
    return status


__all__ = ["execute_once", "Invocation"]
