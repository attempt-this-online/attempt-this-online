from hashlib import sha256
from secrets import token_bytes, token_hex

import msgpack
from pydantic import ValidationError
from starlette.applications import Starlette
from starlette.concurrency import run_in_threadpool
from starlette.middleware import Middleware
from starlette.middleware.cors import CORSMiddleware
from starlette.requests import Request
from starlette.responses import RedirectResponse, Response
from starlette.routing import Route

from attempt_this_online import metadata
from attempt_this_online.run import execute_once, Invocation

# Change to True if running behind a trusted reverse proxy
TRUST_PROXY_HEADER = False

IP_ADDRESS_SALT = token_bytes()
MAX_REQUEST_SIZE = 2 ** 16


async def not_found_handler(_request, _exc):
    return RedirectResponse("https://github.com/attempt-this-online/attempt-this-online", 303)


async def execute_once_route(request: Request) -> Response:
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
    status = await run_in_threadpool(execute_once, ip_hash, invocation_id, invocation)
    return Response(msgpack.dumps(status), 200)


async def get_metadata(_request) -> Response:
    return Response(msgpack.dumps(metadata.languages))


app = Starlette(
    routes=[
        Route("/api/v0/execute", methods=["POST"], endpoint=execute_once_route),
        Route("/api/v0/metadata", methods=["GET"], endpoint=get_metadata),
    ],
    exception_handlers={
        404: not_found_handler,
    },
    middleware=[
        Middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["POST"]),
    ],
)
