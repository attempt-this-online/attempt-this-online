import json
from base64 import b64encode
from hashlib import sha256
from os import access, getenv, mkdir, R_OK, X_OK
from pathlib import Path
from secrets import token_bytes, token_hex
from shutil import rmtree
from subprocess import run
from tempfile import TemporaryDirectory

import msgpack
from pydantic import BaseModel, validator, ValidationError
from starlette.applications import Starlette
from starlette.concurrency import run_in_threadpool
from starlette.exceptions import HTTPException
from starlette.requests import Request
from starlette.responses import RedirectResponse, Response
from starlette.routing import Route

# Change to True if running behind a trusted reverse proxy
TRUST_PROXY_HEADER = True

IP_ADDRESS_SALT = token_bytes()
MAX_REQUEST_SIZE = 2 ** 16


class Invocation(BaseModel):
    language: str
    code: bytes
    input: bytes
    arguments: list[bytes]
    options: list[bytes]

    @validator("language")
    def validate_language(cls, value: str):
        if value not in languages:
            raise ValueError("no such language")
        else:
            return value


languages = {l.name for l in Path("/usr/local/share/ATO/runners").iterdir()}


def execute(ip_hash: str, invocation_id: str, invocation: Invocation) -> dict:
    try:
        hashed_invocation_id = sha256(invocation_id.encode()).hexdigest()
        dir_i = Path("/run/ATO_i") / hashed_invocation_id
        mkdir(dir_i)
        with (dir_i / "code").open("wb") as f:
            f.write(invocation.code)
        with (dir_i / "input").open("wb") as f:
            f.write(invocation.input)

        if any("b\0" in arg for arg in invocation.arguments) or any(b"\0" in opt for opt in invocation.options):
            raise HTTPException("arguments and options cannot contain null bytes", 400)

        with (dir_i / "arguments").open("wb") as f:
            f.write(b"".join(arg + b"\0" for arg in invocation.arguments))
        with (dir_i / "options").open("wb") as f:
            f.write(b"".join(opt + b"\0" for opt in invocation.options))

        run(
            ["sudo", "-u", "sandbox", "/usr/local/bin/ATO_sandbox", ip_hash, invocation_id, invocation.language],
            env={"PATH": getenv("PATH")}
        )
        dir_o = Path("/run/ATO_o") / hashed_invocation_id
        with (dir_o / "stdout").open("rb") as f:
            stdout = f.read()
        with (dir_o / "stderr").open("rb") as f:
            stderr = f.read()
        with (dir_o / "status").open("r") as f:
            status = json.load(f)
        del status["_dummy"]
        # remove "us" (microsecond) suffix on all timing info to get just an integer
        status["user"] = int(status["user"][:-2])
        status["kernel"] = int(status["kernel"][:-2])
        status["real"] = int(status["real"][:-2])

        status["stdout"] = stdout
        status["stderr"] = stderr
    finally:
        rmtree(dir_i)
        run(["sudo", "-u", "sandbox", "/usr/local/bin/ATO_rm", invocation_id])
    return status


async def not_found_handler(_request, _exc):
    return RedirectResponse("https://github.com/pxeger/attempt_this_online", 303)


async def execute_route(request: Request) -> Response:
    try:
        if int(request.headers.get("Content-Length")) > MAX_REQUEST_SIZE:
            return Response(
                # Error message in the style of Pydantics' so that it's consistent
                msgpack.dumps([{"loc": (), "msg": "request too large", "type": "value_error.size"}]),
                # HTTP Request Body Too Large
                413
            )
    except (ValueError, TypeError):
        return Response("invalid content length", 400)
    data = msgpack.loads(await request.body())
    try:
        invocation = Invocation(**data)
    except ValidationError as e:
        return Response(msgpack.dumps(e.errors()), 400)
    if TRUST_PROXY_HEADER:
        ip = request.headers.get("X-Real-IP", request.client.host)
    else:
        ip = request.client.host
    ip_hash = sha256(IP_ADDRESS_SALT + ip.encode()).hexdigest()
    invocation_id = token_hex()
    status = await run_in_threadpool(execute, ip_hash, invocation_id, invocation)
    return Response(msgpack.dumps(status), 200)


app = Starlette(
    routes=[
        Route("/api/v0/execute", methods=["POST"], endpoint=execute_route),
    ],
    exception_handlers={
        404: not_found_handler,
    },
)
